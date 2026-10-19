use crate::{
    db::AppState,
    error::{Error, Result},
    models::*,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize)]
struct PersonalBackup {
    format_version: u32,
    group_id: String,
    personal_nodes: Vec<ScriptNode>,
    dependencies: Vec<ScriptNode>,
    #[serde(default)]
    flows: Vec<Flow>,
    #[serde(default)]
    flow_dependencies: Vec<Flow>,
}
pub async fn export(state: &AppState, directory: Option<String>) -> Result<FileResult> {
    let _guard = state.write.lock().await;
    let w = state.required_workspace().await?;
    if w.role != Role::Sales {
        return Err(Error::invalid("个人备份只用于业务员分支"));
    }
    let data = PersonalBackup {
        format_version: 2,
        group_id: w.group_id,
        personal_nodes: state.list("personal_nodes").await?,
        dependencies: state.list("personal_dependencies").await?,
        flows: state.list("personal_flows").await?,
        flow_dependencies: state.list("personal_flow_dependencies").await?,
    };
    let base = directory
        .map(PathBuf::from)
        .unwrap_or_else(|| state.root.join("exports"));
    if !base.is_absolute() {
        return Err(Error::invalid("请选择绝对路径的备份位置"));
    }
    let destination = base.join(format!("knowtosay-personal-{}", uuid::Uuid::new_v4()));
    tokio::fs::create_dir_all(&destination).await?;
    tokio::fs::write(
        destination.join("personal.json"),
        serde_json::to_vec_pretty(&data)?,
    )
    .await?;
    Ok(FileResult {
        path: destination.to_string_lossy().into(),
    })
}
pub async fn import(state: &AppState, directory: &str) -> Result<()> {
    let _guard = state.write.lock().await;
    let w = state.required_workspace().await?;
    let path = PathBuf::from(directory).join("personal.json");
    if tokio::fs::metadata(&path).await?.len() > 64 * 1024 * 1024 {
        return Err(Error::invalid("个人备份过大"));
    }
    let mut data: PersonalBackup = serde_json::from_slice(&tokio::fs::read(path).await?)?;
    if data.format_version == 1 {
        upgrade_single_flow(&mut data.personal_nodes, &mut data.flow_dependencies);
        upgrade_single_flow(&mut data.dependencies, &mut data.flow_dependencies);
        data.format_version = 2;
    }
    if w.role != Role::Sales || w.group_id != data.group_id || data.format_version != 2 {
        return Err(Error::conflict("备份不属于当前业务员工作组"));
    }
    for n in data.personal_nodes.iter().chain(data.dependencies.iter()) {
        crate::db::validate_node(n)?;
    }
    let mut flows = data.flow_dependencies.clone();
    flows.extend(state.list::<Flow>("official_flows").await?);
    let mut map: std::collections::BTreeMap<_, _> =
        flows.into_iter().map(|f| (f.id.clone(), f)).collect();
    for flow in &data.flows {
        map.insert(flow.id.clone(), flow.clone());
    }
    let mut nodes = data.dependencies.clone();
    nodes.extend(state.list::<ScriptNode>("official_nodes").await?);
    let mut effective: std::collections::BTreeMap<_, _> =
        nodes.into_iter().map(|n| (n.id.clone(), n)).collect();
    for node in &data.personal_nodes {
        effective.insert(node.id.clone(), node.clone());
    }
    crate::db::validate_flow_membership(
        &effective.into_values().collect::<Vec<_>>(),
        &map.into_values().collect::<Vec<_>>(),
    )?;
    let mut tx = state.pool.begin().await?;
    for (table, nodes) in [
        ("personal_nodes", data.personal_nodes),
        ("personal_dependencies", data.dependencies),
    ] {
        sqlx::query(&format!("DELETE FROM {table}"))
            .execute(&mut *tx)
            .await?;
        for node in nodes {
            sqlx::query(&format!("INSERT INTO {table}(id,json) VALUES(?,?)"))
                .bind(&node.id)
                .bind(serde_json::to_string(&node)?)
                .execute(&mut *tx)
                .await?;
        }
    }
    for (table, flows) in [
        ("personal_flows", data.flows),
        ("personal_flow_dependencies", data.flow_dependencies),
    ] {
        sqlx::query(&format!("DELETE FROM {table}"))
            .execute(&mut *tx)
            .await?;
        for flow in flows {
            crate::db::validate_flow(&flow)?;
            sqlx::query(&format!("INSERT INTO {table}(id,json) VALUES(?,?)"))
                .bind(&flow.id)
                .bind(serde_json::to_string(&flow)?)
                .execute(&mut *tx)
                .await?;
        }
    }
    tx.commit().await?;
    Ok(())
}
