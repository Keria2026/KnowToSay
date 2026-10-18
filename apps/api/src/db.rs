use crate::{
    error::{Error, Result},
    models::*,
};
use serde::{Serialize, de::DeserializeOwned};
use sqlx::{
    SqlitePool,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous},
};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::sync::{Mutex, RwLock};

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub root: Arc<PathBuf>,
    pub write: Arc<Mutex<()>>,
    pub sync: Arc<Mutex<()>>,
    pub progress: Arc<RwLock<SyncStatus>>,
    pub client: reqwest::Client,
}

impl AppState {
    pub async fn open(root: impl AsRef<Path>) -> Result<Self> {
        let root = root.as_ref().to_path_buf();
        tokio::fs::create_dir_all(root.join("blobs")).await?;
        tokio::fs::create_dir_all(root.join("staging")).await?;
        let opts = SqliteConnectOptions::new()
            .filename(root.join("workspace.sqlite"))
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .synchronous(SqliteSynchronous::Full)
            .foreign_keys(true)
            .busy_timeout(std::time::Duration::from_secs(15));
        let pool = SqlitePoolOptions::new()
            .max_connections(4)
            .connect_with(opts)
            .await?;
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .map_err(Error::internal)?;
        let client = reqwest::Client::builder()
            .no_proxy()
            .connect_timeout(std::time::Duration::from_secs(4))
            .timeout(std::time::Duration::from_secs(3600))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(Error::internal)?;
        Ok(Self {
            pool,
            root: Arc::new(root),
            write: Arc::new(Mutex::new(())),
            sync: Arc::new(Mutex::new(())),
            progress: Arc::new(RwLock::new(SyncStatus::default())),
            client,
        })
    }
    pub async fn workspace(&self) -> Result<Option<Workspace>> {
        let json: Option<String> = sqlx::query_scalar("SELECT json FROM workspace WHERE id=1")
            .fetch_optional(&self.pool)
            .await?;
        json.map(|s| serde_json::from_str(&s).map_err(Error::from))
            .transpose()
    }
    pub async fn required_workspace(&self) -> Result<Workspace> {
        self.workspace()
            .await?
            .ok_or_else(|| Error::conflict("请先创建或加入工作组"))
    }
    pub async fn admin(&self) -> Result<Workspace> {
        let w = self.required_workspace().await?;
        if w.role != Role::Admin {
            return Err(Error::denied());
        }
        Ok(w)
    }
    pub async fn list<T: DeserializeOwned>(&self, table: &str) -> Result<Vec<T>> {
        let table = checked_table(table)?;
        let values: Vec<String> =
            sqlx::query_scalar(&format!("SELECT json FROM {table} ORDER BY id"))
                .fetch_all(&self.pool)
                .await?;
        values
            .into_iter()
            .map(|v| serde_json::from_str(&v).map_err(Error::from))
            .collect()
    }
    pub async fn put<T: Serialize>(&self, table: &str, id: &str, value: &T) -> Result<()> {
        let table = checked_table(table)?;
        sqlx::query(&format!("INSERT INTO {table}(id,json) VALUES(?,?) ON CONFLICT(id) DO UPDATE SET json=excluded.json")).bind(id).bind(serde_json::to_string(value)?).execute(&self.pool).await?;
        Ok(())
    }
    pub async fn snapshot(&self) -> Result<Snapshot> {
        let _guard = self.write.lock().await;
        let Some(workspace) = self.workspace().await? else {
            return Ok(Snapshot {
                workspace: None,
                nodes: vec![],
                materials: vec![],
                draft_dirty: false,
            });
        };
        let official = self.list::<ScriptNode>("official_nodes").await?;
        let official_map: BTreeMap<_, _> = official
            .iter()
            .cloned()
            .map(|n| (n.id.clone(), n))
            .collect();
        let mut map: BTreeMap<String, NodeRecord> = BTreeMap::new();
        if workspace.role == Role::Sales {
            for n in self.list::<ScriptNode>("personal_dependencies").await? {
                map.insert(
                    n.id.clone(),
                    NodeRecord {
                        node: n,
                        source: "retained".into(),
                        official: None,
                    },
                );
            }
        }
        let (nodes, mut materials, dirty) = if workspace.role == Role::Admin {
            let n = self.list::<ScriptNode>("draft_nodes").await?;
            let m = self.list::<Material>("draft_materials").await?;
            let dirty = n != official || m != self.list::<Material>("official_materials").await?;
            (n, m, dirty)
        } else {
            (
                official,
                self.list::<Material>("official_materials").await?,
                false,
            )
        };
        for n in nodes {
            map.insert(
                n.id.clone(),
                NodeRecord {
                    official: official_map.get(&n.id).cloned(),
                    node: n,
                    source: if workspace.role == Role::Admin {
                        "draft"
                    } else {
                        "official"
                    }
                    .into(),
                },
            );
        }
        if workspace.role == Role::Sales {
            for n in self.list::<ScriptNode>("personal_nodes").await? {
                map.insert(
                    n.id.clone(),
                    NodeRecord {
                        official: official_map.get(&n.id).cloned(),
                        node: n,
                        source: "personal".into(),
                    },
                );
            }
        }
        for material in &mut materials {
            material.rows.truncate(150);
        }
        Ok(Snapshot {
            workspace: Some(workspace),
            nodes: map.into_values().collect(),
            materials,
            draft_dirty: dirty,
        })
    }
    pub async fn save_node(&self, node: ScriptNode) -> Result<()> {
        validate_node(&node)?;
        let _guard = self.write.lock().await;
        let w = self.required_workspace().await?;
        if w.role == Role::Admin {
            return self.put("draft_nodes", &node.id, &node).await;
        }
        // Retain only official dependencies reachable from this personal node. New official
        // versions still take precedence over retained copies, personal edits over both.
        let official: BTreeMap<_, _> = self
            .list::<ScriptNode>("official_nodes")
            .await?
            .into_iter()
            .map(|n| (n.id.clone(), n))
            .collect();
        let mut seen = BTreeSet::new();
        let mut pending = vec![node.id.clone()];
        pending.extend(node.branches.iter().map(|b| b.target_id.clone()));
        let mut tx = self.pool.begin().await?;
        while let Some(id) = pending.pop() {
            if !seen.insert(id.clone()) {
                continue;
            }
            if let Some(n) = official.get(&id) {
                pending.extend(n.branches.iter().map(|b| b.target_id.clone()));
                sqlx::query("INSERT OR IGNORE INTO personal_dependencies(id,json) VALUES(?,?)")
                    .bind(&id)
                    .bind(serde_json::to_string(n)?)
                    .execute(&mut *tx)
                    .await?;
            }
        }
        sqlx::query("INSERT INTO personal_nodes(id,json) VALUES(?,?) ON CONFLICT(id) DO UPDATE SET json=excluded.json").bind(&node.id).bind(serde_json::to_string(&node)?).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(())
    }
    pub async fn latest(&self) -> Result<Manifest> {
        self.admin().await?;
        let json: Option<String> =
            sqlx::query_scalar("SELECT json FROM releases ORDER BY version DESC LIMIT 1")
                .fetch_optional(&self.pool)
                .await?;
        serde_json::from_str(&json.ok_or_else(|| Error::conflict("管理员尚未发布资料"))?)
            .map_err(Error::from)
    }
    pub async fn prune_dependencies(&self) -> Result<()> {
        let personal = self.list::<ScriptNode>("personal_nodes").await?;
        let dependencies = self.list::<ScriptNode>("personal_dependencies").await?;
        let mut effective: BTreeMap<_, _> = dependencies
            .iter()
            .cloned()
            .map(|n| (n.id.clone(), n))
            .collect();
        effective.extend(
            self.list::<ScriptNode>("official_nodes")
                .await?
                .into_iter()
                .map(|n| (n.id.clone(), n)),
        );
        effective.extend(personal.iter().cloned().map(|n| (n.id.clone(), n)));
        let mut pending: Vec<_> = personal.iter().map(|n| n.id.clone()).collect();
        let mut seen = BTreeSet::new();
        while let Some(id) = pending.pop() {
            if !seen.insert(id.clone()) {
                continue;
            }
            if let Some(n) = effective.get(&id) {
                pending.extend(n.branches.iter().map(|b| b.target_id.clone()));
            }
        }
        for n in dependencies {
            if !seen.contains(&n.id) {
                sqlx::query("DELETE FROM personal_dependencies WHERE id=?")
                    .bind(&n.id)
                    .execute(&self.pool)
                    .await?;
            }
        }
        Ok(())
    }
    pub async fn publish(&self) -> Result<Manifest> {
        let _guard = self.write.lock().await;
        let mut workspace = self.admin().await?;
        let nodes = self.list::<ScriptNode>("draft_nodes").await?;
        let materials = self.list::<Material>("draft_materials").await?;
        validate_manifest_content(&nodes, &materials)?;
        let max: Option<i64> = sqlx::query_scalar("SELECT MAX(version) FROM releases")
            .fetch_one(&self.pool)
            .await?;
        workspace.version = workspace.version.max(max.unwrap_or(0)) + 1;
        let manifest = Manifest {
            format_version: 1,
            group_id: workspace.group_id.clone(),
            group_name: workspace.name.clone(),
            version: workspace.version,
            nodes,
            materials,
        };
        let mut tx = self.pool.begin().await?;
        sqlx::query("DELETE FROM official_nodes")
            .execute(&mut *tx)
            .await?;
        sqlx::query("INSERT INTO official_nodes SELECT * FROM draft_nodes")
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM official_materials")
            .execute(&mut *tx)
            .await?;
        sqlx::query("INSERT INTO official_materials SELECT * FROM draft_materials")
            .execute(&mut *tx)
            .await?;
        sqlx::query("INSERT INTO releases(version,json) VALUES(?,?)")
            .bind(manifest.version)
            .bind(serde_json::to_string(&manifest)?)
            .execute(&mut *tx)
            .await?;
        sqlx::query("UPDATE workspace SET json=? WHERE id=1")
            .bind(serde_json::to_string(&workspace)?)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(manifest)
    }
    pub async fn apply(&self, manifest: &Manifest, address: &str) -> Result<()> {
        if manifest.format_version != 1 {
            return Err(Error::invalid("不支持此资料格式"));
        }
        validate_manifest_content(&manifest.nodes, &manifest.materials)?;
        let _guard = self.write.lock().await;
        let current = self.workspace().await?;
        if let Some(w) = &current {
            if w.role != Role::Sales || w.group_id != manifest.group_id {
                return Err(Error::conflict("资料不属于当前工作组"));
            }
            if manifest.version <= w.version {
                return Ok(());
            }
        }
        let w = Workspace {
            group_id: manifest.group_id.clone(),
            name: manifest.group_name.clone(),
            role: Role::Sales,
            version: manifest.version,
            admin_address: Some(address.into()),
        };
        let mut tx = self.pool.begin().await?;
        sqlx::query("DELETE FROM official_nodes")
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM official_materials")
            .execute(&mut *tx)
            .await?;
        for n in &manifest.nodes {
            sqlx::query("INSERT INTO official_nodes(id,json) VALUES(?,?)")
                .bind(&n.id)
                .bind(serde_json::to_string(n)?)
                .execute(&mut *tx)
                .await?;
        }
        for m in &manifest.materials {
            sqlx::query("INSERT INTO official_materials(id,json) VALUES(?,?)")
                .bind(&m.id)
                .bind(serde_json::to_string(m)?)
                .execute(&mut *tx)
                .await?;
        }
        sqlx::query("INSERT INTO workspace(id,json) VALUES(1,?) ON CONFLICT(id) DO UPDATE SET json=excluded.json").bind(serde_json::to_string(&w)?).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(())
    }
}

fn checked_table(table: &str) -> Result<&str> {
    match table {
        "official_nodes"
        | "draft_nodes"
        | "personal_nodes"
        | "personal_dependencies"
        | "official_materials"
        | "draft_materials" => Ok(table),
        _ => Err(Error::invalid("未知数据表")),
    }
}
pub fn validate_node(n: &ScriptNode) -> Result<()> {
    if n.id.is_empty() || n.id.len() > 100 || n.title.trim().is_empty() || n.title.len() > 500 {
        return Err(Error::invalid("话术需要有效标识和标题"));
    }
    if n.replies.len() > 50 || n.branches.len() > 200 || n.variables.len() > 100 {
        return Err(Error::invalid("话术内容超过限制"));
    }
    if n.position
        .as_ref()
        .is_some_and(|p| !p.x.is_finite() || !p.y.is_finite())
    {
        return Err(Error::invalid("流程坐标无效"));
    }
    let mut names = BTreeSet::new();
    for v in &n.variables {
        if v.name.is_empty() || !names.insert(&v.name) || v.name.contains(['{', '}']) {
            return Err(Error::invalid("变量名不能为空或重复，也不能包含花括号"));
        }
    }
    Ok(())
}
pub fn validate_manifest_content(nodes: &[ScriptNode], materials: &[Material]) -> Result<()> {
    if nodes.len() > 100_000 || materials.len() > 100_000 {
        return Err(Error::invalid("资料数量超过限制"));
    }
    let ids: BTreeSet<_> = nodes.iter().map(|n| &n.id).collect();
    if ids.len() != nodes.len() {
        return Err(Error::invalid("话术标识重复"));
    }
    let material_ids: BTreeSet<_> = materials.iter().map(|m| &m.id).collect();
    if material_ids.len() != materials.len() {
        return Err(Error::invalid("素材标识重复"));
    }
    for n in nodes {
        validate_node(n)?;
        for b in &n.branches {
            if b.label.trim().is_empty() || !ids.contains(&b.target_id) {
                return Err(Error::invalid(format!(
                    "话术「{}」存在失效的流程连接",
                    n.title
                )));
            }
        }
        // Material tombstones are valid: removed material references remain visible as missing.
    }
    for m in materials {
        if m.id.is_empty() || m.name.trim().is_empty() {
            return Err(Error::invalid("素材标识和名称不能为空"));
        }
        if let Some(hash) = &m.hash
            && !valid_hash(hash)
        {
            return Err(Error::invalid("素材文件校验值无效"));
        }
    }
    Ok(())
}
pub fn valid_hash(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
}
