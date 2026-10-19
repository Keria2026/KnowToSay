use crate::{
    db::{AppState, validate_manifest_content},
    error::{Error, Result},
    files::hash_file,
    models::*,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};

#[derive(Serialize, Deserialize)]
struct BackupData {
    format_version: u32,
    workspace: Workspace,
    draft_nodes: Vec<ScriptNode>,
    official_nodes: Vec<ScriptNode>,
    #[serde(default)]
    draft_flows: Vec<Flow>,
    #[serde(default)]
    official_flows: Vec<Flow>,
    draft_materials: Vec<Material>,
    official_materials: Vec<Material>,
    #[serde(default)]
    draft_material_categories: Vec<String>,
    #[serde(default)]
    official_material_categories: Vec<String>,
    #[serde(default)]
    draft_quick_categories: Vec<String>,
    #[serde(default)]
    official_quick_categories: Vec<String>,
    releases: Vec<Manifest>,
}

async fn directory(
    state: &AppState,
    requested: Option<String>,
    label: &str,
) -> Result<(PathBuf, PathBuf)> {
    let base = requested
        .map(PathBuf::from)
        .unwrap_or_else(|| state.root.join("exports"));
    if !base.is_absolute() {
        return Err(Error::invalid("请选择绝对路径的导出文件夹"));
    }
    tokio::fs::create_dir_all(&base).await?;
    let id = uuid::Uuid::new_v4();
    let final_path = base.join(format!("knowtosay-{label}-{id}"));
    let staging = base.join(format!(".knowtosay-incomplete-{id}"));
    tokio::fs::create_dir_all(staging.join("blobs")).await?;
    Ok((staging, final_path))
}

async fn copy_materials(from: &Path, to: &Path, materials: &[Material]) -> Result<()> {
    let mut hashes = BTreeMap::new();
    for m in materials {
        if let Some(hash) = &m.hash {
            if !crate::db::valid_hash(hash) {
                return Err(Error::invalid("备份素材校验值无效"));
            }
            if let Some(size) = hashes.insert(hash, m.size)
                && size != m.size
            {
                return Err(Error::invalid("备份素材长度冲突"));
            }
        }
    }
    let mut pending = Vec::new();
    let mut required = 64 * 1024 * 1024u64;
    for (hash, size) in hashes {
        let source = from.join(hash);
        let target = to.join(hash);
        if tokio::fs::metadata(&source).await?.len() != size || hash_file(&source).await? != *hash {
            return Err(Error::conflict("备份或素材文件校验失败，当前资料未更改"));
        }
        if source == target
            || (target.exists()
                && tokio::fs::metadata(&target).await?.len() == size
                && hash_file(&target).await? == *hash)
        {
            continue;
        }
        required = required
            .checked_add(size)
            .ok_or_else(|| Error::invalid("素材总大小无效"))?;
        pending.push((hash, size, source, target));
    }
    if fs2::available_space(to)? < required {
        return Err(Error::conflict("目标磁盘空间不足"));
    }
    for (hash, size, source, target) in pending {
        let temp = to.join(format!("{}.copying", uuid::Uuid::new_v4()));
        tokio::fs::copy(&source, &temp).await?;
        if hash_file(&temp).await? != *hash {
            let _ = tokio::fs::remove_file(temp).await;
            return Err(Error::conflict("素材复制校验失败"));
        }
        if target.exists()
            && tokio::fs::metadata(&target).await?.len() == size
            && hash_file(&target).await? == *hash
        {
            tokio::fs::remove_file(&temp).await?;
        } else {
            if target.exists() {
                tokio::fs::remove_file(&target).await?;
            }
            tokio::fs::rename(temp, target).await?;
        }
    }
    Ok(())
}

pub async fn backup(state: &AppState, requested: Option<String>) -> Result<FileResult> {
    let _guard = state.write.lock().await;
    let workspace = state.admin().await?;
    let releases: Vec<String> = sqlx::query_scalar("SELECT json FROM releases ORDER BY version")
        .fetch_all(&state.pool)
        .await?;
    let data = BackupData {
        format_version: 2,
        workspace,
        draft_nodes: state.list("draft_nodes").await?,
        official_nodes: state.list("official_nodes").await?,
        draft_flows: state.list("draft_flows").await?,
        official_flows: state.list("official_flows").await?,
        draft_materials: state.list("draft_materials").await?,
        official_materials: state.list("official_materials").await?,
        draft_material_categories: state.list("draft_material_categories").await?,
        official_material_categories: state.list("official_material_categories").await?,
        draft_quick_categories: state.list("draft_quick_categories").await?,
        official_quick_categories: state.list("official_quick_categories").await?,
        releases: releases
            .into_iter()
            .map(|s| serde_json::from_str(&s))
            .collect::<std::result::Result<_, _>>()?,
    };
    let (staging, final_path) = directory(state, requested, "backup").await?;
    let mut materials = data.draft_materials.clone();
    materials.extend(data.official_materials.clone());
    for r in &data.releases {
        materials.extend(r.materials.clone());
    }
    copy_materials(
        &state.root.join("blobs"),
        &staging.join("blobs"),
        &materials,
    )
    .await?;
    tokio::fs::write(
        staging.join("backup.json"),
        serde_json::to_vec_pretty(&data)?,
    )
    .await?;
    tokio::fs::rename(staging, &final_path).await?;
    Ok(FileResult {
        path: final_path.to_string_lossy().into(),
    })
}

pub async fn export_package(state: &AppState, requested: Option<String>) -> Result<FileResult> {
    let _guard = state.write.lock().await;
    let manifest = state.latest().await?;
    let (staging, final_path) = directory(state, requested, "package").await?;
    copy_materials(
        &state.root.join("blobs"),
        &staging.join("blobs"),
        &manifest.materials,
    )
    .await?;
    tokio::fs::write(
        staging.join("manifest.json"),
        serde_json::to_vec_pretty(&manifest)?,
    )
    .await?;
    tokio::fs::rename(staging, &final_path).await?;
    Ok(FileResult {
        path: final_path.to_string_lossy().into(),
    })
}

pub async fn import_package(state: &AppState, directory: &str) -> Result<()> {
    let _sync = state
        .sync
        .try_lock()
        .map_err(|_| Error::conflict("请等待当前同步完成"))?;
    let path = PathBuf::from(directory);
    let bytes = tokio::fs::read(path.join("manifest.json")).await?;
    let mut manifest: Manifest = serde_json::from_slice(&bytes)?;
    manifest.upgrade();
    if manifest.format_version != 2
        || manifest.version < 1
        || uuid::Uuid::parse_str(&manifest.group_id).is_err()
    {
        return Err(Error::invalid("资料包无效"));
    }
    validate_manifest_content(&manifest.nodes, &manifest.materials)?;
    crate::db::validate_flow_membership(&manifest.nodes, &manifest.flows)?;
    let address = if let Some(w) = state.workspace().await? {
        if w.role != Role::Sales || w.group_id != manifest.group_id {
            return Err(Error::conflict("资料包不属于当前业务员工作组"));
        }
        w.admin_address.unwrap_or_default()
    } else {
        String::new()
    };
    copy_materials(
        &path.join("blobs"),
        &state.root.join("blobs"),
        &manifest.materials,
    )
    .await?;
    state.apply(&manifest, &address).await
}

pub async fn restore(state: &AppState, directory: &str) -> Result<()> {
    let _guard = state.write.lock().await;
    let current = state.workspace().await?;
    if current.as_ref().is_some_and(|w| w.role != Role::Admin) {
        return Err(Error::denied());
    }
    let path = PathBuf::from(directory);
    let data = read_backup(&path).await?;
    if current
        .as_ref()
        .is_some_and(|w| w.group_id != data.workspace.group_id)
    {
        return Err(Error::conflict("备份属于其他工作组，不能覆盖当前工作组"));
    }
    let mut all = data.official_materials.clone();
    all.extend(data.draft_materials.clone());
    for r in &data.releases {
        all.extend(r.materials.clone());
    }
    copy_materials(&path.join("blobs"), &state.root.join("blobs"), &all).await?;
    let mut workspace = data.workspace.clone();
    let current_max: Option<i64> = sqlx::query_scalar("SELECT MAX(version) FROM releases")
        .fetch_one(&state.pool)
        .await?;
    workspace.version = workspace
        .version
        .max(current.as_ref().map_or(0, |w| w.version))
        .max(current_max.unwrap_or(0))
        + 1;
    let manifest = Manifest {
        format_version: 2,
        group_id: workspace.group_id.clone(),
        group_name: workspace.name.clone(),
        version: workspace.version,
        nodes: data.official_nodes.clone(),
        flows: data.official_flows.clone(),
        materials: data.official_materials.clone(),
        material_categories: data.official_material_categories.clone(),
        quick_categories: data.official_quick_categories.clone(),
    };
    let mut tx = state.pool.begin().await?;
    sqlx::query("DELETE FROM draft_quick_categories")
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM official_quick_categories")
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM draft_material_categories")
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM official_material_categories")
        .execute(&mut *tx)
        .await?;
    for (table, entries) in [
        (
            "draft_nodes",
            data.draft_nodes
                .iter()
                .map(|n| Ok((n.id.clone(), serde_json::to_string(n)?)))
                .collect::<Result<Vec<_>>>()?,
        ),
        (
            "official_nodes",
            data.official_nodes
                .iter()
                .map(|n| Ok((n.id.clone(), serde_json::to_string(n)?)))
                .collect::<Result<Vec<_>>>()?,
        ),
        (
            "draft_materials",
            data.draft_materials
                .iter()
                .map(|n| Ok((n.id.clone(), serde_json::to_string(n)?)))
                .collect::<Result<Vec<_>>>()?,
        ),
        (
            "official_materials",
            data.official_materials
                .iter()
                .map(|n| Ok((n.id.clone(), serde_json::to_string(n)?)))
                .collect::<Result<Vec<_>>>()?,
        ),
    ] {
        sqlx::query(&format!("DELETE FROM {table}"))
            .execute(&mut *tx)
            .await?;
        for (id, json) in entries {
            sqlx::query(&format!("INSERT INTO {table}(id,json) VALUES(?,?)"))
                .bind(id)
                .bind(json)
                .execute(&mut *tx)
                .await?;
        }
    }
    for (table, flows) in [
        ("draft_flows", &data.draft_flows),
        ("official_flows", &data.official_flows),
    ] {
        sqlx::query(&format!("DELETE FROM {table}"))
            .execute(&mut *tx)
            .await?;
        for flow in flows {
            sqlx::query(&format!("INSERT INTO {table}(id,json) VALUES(?,?)"))
                .bind(&flow.id)
                .bind(serde_json::to_string(flow)?)
                .execute(&mut *tx)
                .await?;
        }
    }
    for (table, categories) in [
        ("draft_quick_categories", &data.draft_quick_categories),
        ("official_quick_categories", &data.official_quick_categories),
        ("draft_material_categories", &data.draft_material_categories),
        (
            "official_material_categories",
            &data.official_material_categories,
        ),
    ] {
        for name in categories {
            sqlx::query(&format!(
                "INSERT OR IGNORE INTO {table}(id,json) VALUES(?,?)"
            ))
            .bind(name)
            .bind(serde_json::to_string(name)?)
            .execute(&mut *tx)
            .await?;
        }
    }
    sqlx::query("INSERT OR REPLACE INTO releases(version,json) VALUES(?,?)")
        .bind(manifest.version)
        .bind(serde_json::to_string(&manifest)?)
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO workspace(id,json) VALUES(1,?) ON CONFLICT(id) DO UPDATE SET json=excluded.json").bind(serde_json::to_string(&workspace)?).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

async fn read_backup(path: &Path) -> Result<BackupData> {
    let mut data: BackupData =
        serde_json::from_slice(&tokio::fs::read(path.join("backup.json")).await?)?;
    if data.format_version == 1 {
        upgrade_single_flow(&mut data.draft_nodes, &mut data.draft_flows);
        upgrade_single_flow(&mut data.official_nodes, &mut data.official_flows);
        data.format_version = 2;
    }
    for release in &mut data.releases {
        release.upgrade();
    }
    if data.format_version != 2
        || data.workspace.role != Role::Admin
        || uuid::Uuid::parse_str(&data.workspace.group_id).is_err()
    {
        return Err(Error::invalid("不支持此备份格式"));
    }
    validate_manifest_content(&data.official_nodes, &data.official_materials)?;
    crate::db::validate_flow_membership(&data.official_nodes, &data.official_flows)?;
    crate::db::validate_flow_membership(&data.draft_nodes, &data.draft_flows)?;
    for n in &data.draft_nodes {
        crate::db::validate_node(n)?;
    }
    validate_manifest_content(&[], &data.draft_materials)?;
    for release in &data.releases {
        if release.group_id != data.workspace.group_id {
            return Err(Error::invalid("备份中含其他工作组版本"));
        }
        validate_manifest_content(&release.nodes, &release.materials)?;
        crate::db::validate_flow_membership(&release.nodes, &release.flows)?;
    }
    Ok(data)
}

fn imported_id(ids: &mut BTreeMap<String, String>, original: &str) -> String {
    ids.entry(original.to_owned())
        .or_insert_with(|| uuid::Uuid::new_v4().to_string())
        .clone()
}

/// Copy data into this administrator's draft without adopting the source identity or releases.
pub async fn import_as_draft(state: &AppState, directory: &str) -> Result<()> {
    let _guard = state.write.lock().await;
    state.admin().await?;
    let path = PathBuf::from(directory);
    let data = read_backup(&path).await?;
    let mut flow_ids = BTreeMap::new();
    let mut node_ids = BTreeMap::new();
    let mut material_ids = BTreeMap::new();
    let mut names: BTreeSet<String> = state
        .list::<Flow>("draft_flows")
        .await?
        .into_iter()
        .map(|f| f.name.to_lowercase())
        .collect();
    let mut flows = data.draft_flows;
    for flow in &mut flows {
        flow.id = imported_id(&mut flow_ids, &flow.id);
        let original = flow.name.clone();
        let mut index = 1;
        while !names.insert(flow.name.to_lowercase()) {
            index += 1;
            let suffix = format!("（导入 {index}）");
            flow.name = original
                .chars()
                .take(80 - suffix.chars().count())
                .collect::<String>()
                + &suffix;
        }
    }
    let mut materials = data.draft_materials;
    for material in &mut materials {
        material.id = imported_id(&mut material_ids, &material.id);
    }
    let mut nodes = data.draft_nodes;
    for node in &mut nodes {
        node.id = imported_id(&mut node_ids, &node.id);
        if let Some(id) = &node.flow_id {
            node.flow_id = Some(imported_id(&mut flow_ids, id));
        }
        for branch in &mut node.branches {
            branch.id = uuid::Uuid::new_v4().to_string();
            branch.target_id = imported_id(&mut node_ids, &branch.target_id);
        }
        for id in &mut node.material_ids {
            *id = imported_id(&mut material_ids, id);
        }
    }
    crate::db::validate_flow_membership(&nodes, &flows)?;
    copy_materials(&path.join("blobs"), &state.root.join("blobs"), &materials).await?;
    let mut tx = state.pool.begin().await?;
    for (table, entries) in [
        (
            "draft_flows",
            flows
                .iter()
                .map(|v| Ok((&v.id, serde_json::to_string(v)?)))
                .collect::<Result<Vec<_>>>()?,
        ),
        (
            "draft_materials",
            materials
                .iter()
                .map(|v| Ok((&v.id, serde_json::to_string(v)?)))
                .collect::<Result<Vec<_>>>()?,
        ),
        (
            "draft_nodes",
            nodes
                .iter()
                .map(|v| Ok((&v.id, serde_json::to_string(v)?)))
                .collect::<Result<Vec<_>>>()?,
        ),
    ] {
        for (id, json) in entries {
            sqlx::query(&format!("INSERT INTO {table}(id,json) VALUES(?,?)"))
                .bind(id)
                .bind(json)
                .execute(&mut *tx)
                .await?;
        }
    }
    for (table, categories) in [
        ("draft_quick_categories", data.draft_quick_categories),
        ("draft_material_categories", data.draft_material_categories),
    ] {
        for name in categories {
            sqlx::query(&format!(
                "INSERT INTO {table}(id,json) VALUES(?,?) ON CONFLICT(id) DO NOTHING"
            ))
            .bind(&name)
            .bind(serde_json::to_string(&name)?)
            .execute(&mut *tx)
            .await?;
        }
    }
    tx.commit().await?;
    Ok(())
}
