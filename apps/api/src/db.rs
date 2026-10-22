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
    pub lan_available: Arc<std::sync::atomic::AtomicBool>,
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
            lan_available: Arc::new(std::sync::atomic::AtomicBool::new(false)),
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
                flows: vec![],
                materials: vec![],
                folders: vec![],
                material_categories: vec![],
                quick_categories: vec![],
                manageable_quick_categories: vec![],
                draft_dirty: false,
            });
        };
        let flows = self.effective_flows(&workspace.role).await?;
        let quick_categories = self.quick_categories(&workspace.role).await?;
        let manageable_quick_categories = if workspace.role == Role::Admin {
            quick_categories.clone()
        } else {
            let official: BTreeSet<String> = self
                .list::<String>("official_quick_categories")
                .await?
                .into_iter()
                .collect();
            self.list::<String>("personal_quick_categories")
                .await?
                .into_iter()
                .filter(|name| !official.contains(name))
                .collect()
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
        let official_categories = self.list::<String>("official_material_categories").await?;
        let (nodes, mut materials, material_categories, dirty) = if workspace.role == Role::Admin {
            let n = self.list::<ScriptNode>("draft_nodes").await?;
            let m = self.list::<Material>("draft_materials").await?;
            let categories = self.list::<String>("draft_material_categories").await?;
            let dirty = quick_categories
                != self.list::<String>("official_quick_categories").await?
                || flows != self.list::<Flow>("official_flows").await?
                || n != official
                || m != self.list::<Material>("official_materials").await?
                || categories != official_categories
                || self.list::<Folder>("draft_folders").await?
                    != self.list::<Folder>("official_folders").await?;
            (n, m, categories, dirty)
        } else {
            (
                official,
                self.list::<Material>("official_materials").await?,
                official_categories,
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
            workspace: Some(workspace.clone()),
            nodes: map.into_values().collect(),
            flows,
            materials,
            folders: self
                .list(if workspace.role == Role::Admin {
                    "draft_folders"
                } else {
                    "official_folders"
                })
                .await?,
            material_categories,
            quick_categories,
            manageable_quick_categories,
            draft_dirty: dirty,
        })
    }
    pub async fn create_material_category(&self, name: &str) -> Result<()> {
        let _guard = self.write.lock().await;
        self.admin().await?;
        let name = name.trim();
        if name.is_empty() || name.chars().count() > 80 || name.chars().any(char::is_control) {
            return Err(Error::invalid(
                "分类名称需为 1–80 个字符，且不能包含换行或控制字符",
            ));
        }
        if name == "未分类" {
            return Err(Error::conflict("此分类已存在"));
        }
        let result = sqlx::query(
            "INSERT INTO draft_material_categories(id,json) VALUES(?,?) ON CONFLICT(id) DO NOTHING",
        )
        .bind(name)
        .bind(serde_json::to_string(name)?)
        .execute(&self.pool)
        .await?;
        if result.rows_affected() == 0 {
            return Err(Error::conflict("此分类已存在"));
        }
        if !self
            .list::<Folder>("draft_folders")
            .await?
            .iter()
            .any(|f| f.parent_id.is_none() && f.name == name)
        {
            let folder = Folder {
                id: uuid::Uuid::new_v4().to_string(),
                name: name.into(),
                parent_id: None,
            };
            self.put("draft_folders", &folder.id, &folder).await?;
        }
        Ok(())
    }
    pub async fn quick_categories(&self, role: &Role) -> Result<Vec<String>> {
        let tables = if *role == Role::Admin {
            vec!["draft_quick_categories"]
        } else {
            vec!["official_quick_categories", "personal_quick_categories"]
        };
        let mut categories = BTreeSet::new();
        for table in tables {
            categories.extend(self.list::<String>(table).await?);
        }
        Ok(categories.into_iter().collect())
    }
    pub async fn create_quick_category(&self, name: &str) -> Result<()> {
        let name = name.trim();
        if name.is_empty() || name.chars().count() > 80 || name.chars().any(char::is_control) {
            return Err(Error::invalid(
                "分类名称需为 1–80 个字符，且不能包含换行或控制字符",
            ));
        }
        let _guard = self.write.lock().await;
        let workspace = self.required_workspace().await?;
        if name == "未分类"
            || self
                .quick_categories(&workspace.role)
                .await?
                .iter()
                .any(|n| n == name)
        {
            return Err(Error::conflict("此分类已存在"));
        }
        let table = if workspace.role == Role::Admin {
            "draft_quick_categories"
        } else {
            "personal_quick_categories"
        };
        self.put(table, name, &name).await
    }
    pub async fn update_quick_category(&self, old: &str, name: Option<&str>) -> Result<()> {
        let old = old.trim();
        if old.is_empty() || old == "未分类" {
            return Err(Error::invalid("未分类是默认分类，不能重命名或删除"));
        }
        let replacement = name.map(str::trim).unwrap_or("未分类");
        if replacement.is_empty()
            || replacement.chars().count() > 80
            || replacement.chars().any(char::is_control)
        {
            return Err(Error::invalid(
                "分类名称需为 1–80 个字符，且不能包含换行或控制字符",
            ));
        }
        let _guard = self.write.lock().await;
        let workspace = self.required_workspace().await?;
        let (table, node_tables) = if workspace.role == Role::Admin {
            ("draft_quick_categories", vec!["draft_nodes"])
        } else {
            if self
                .list::<String>("official_quick_categories")
                .await?
                .iter()
                .any(|n| n == old)
            {
                return Err(Error::denied());
            }
            (
                "personal_quick_categories",
                vec!["personal_nodes", "personal_dependencies"],
            )
        };
        if !self.list::<String>(table).await?.iter().any(|n| n == old) {
            return Err(Error::missing("分类不存在"));
        }
        if name.is_some() && old == replacement {
            return Ok(());
        }
        if name.is_some()
            && (replacement == "未分类"
                || self
                    .quick_categories(&workspace.role)
                    .await?
                    .iter()
                    .any(|n| n == replacement))
        {
            return Err(Error::conflict("此分类已存在"));
        }
        let mut tx = self.pool.begin().await?;
        if name.is_some() {
            sqlx::query(&format!("INSERT INTO {table}(id,json) VALUES(?,?)"))
                .bind(replacement)
                .bind(serde_json::to_string(replacement)?)
                .execute(&mut *tx)
                .await?;
        }
        for node_table in node_tables {
            for mut node in self.list::<ScriptNode>(node_table).await? {
                if node.kind == ScriptKind::Quick && node.category.trim() == old {
                    node.category = replacement.into();
                    sqlx::query(&format!("UPDATE {node_table} SET json=? WHERE id=?"))
                        .bind(serde_json::to_string(&node)?)
                        .bind(&node.id)
                        .execute(&mut *tx)
                        .await?;
                }
            }
        }
        sqlx::query(&format!("DELETE FROM {table} WHERE id=?"))
            .bind(old)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }
    pub async fn effective_flows(&self, role: &Role) -> Result<Vec<Flow>> {
        let tables = if *role == Role::Admin {
            vec!["draft_flows"]
        } else {
            vec![
                "personal_flow_dependencies",
                "official_flows",
                "personal_flows",
            ]
        };
        let mut flows = BTreeMap::new();
        for table in tables {
            for flow in self.list::<Flow>(table).await? {
                flows.insert(flow.id.clone(), flow);
            }
        }
        Ok(flows.into_values().collect())
    }
    pub async fn save_flow(&self, mut flow: Flow) -> Result<()> {
        flow.name = flow.name.trim().to_owned();
        validate_flow(&flow)?;
        let _guard = self.write.lock().await;
        let workspace = self.required_workspace().await?;
        let flows = self.effective_flows(&workspace.role).await?;
        if flows
            .iter()
            .any(|f| f.id != flow.id && f.name.to_lowercase() == flow.name.to_lowercase())
        {
            return Err(Error::conflict("已存在同名流程"));
        }
        let table = if workspace.role == Role::Admin {
            "draft_flows"
        } else {
            "personal_flows"
        };
        self.put(table, &flow.id, &flow).await
    }
    pub async fn save_node(&self, node: ScriptNode) -> Result<()> {
        self.save_nodes(vec![node]).await
    }
    pub async fn save_nodes(&self, nodes: Vec<ScriptNode>) -> Result<()> {
        if nodes.is_empty() || nodes.len() > 100_000 {
            return Err(Error::invalid("请选择需要保存的话术，单次最多 100000 条"));
        }
        let mut ids = BTreeSet::new();
        for node in &nodes {
            validate_node(node)?;
            if !ids.insert(&node.id) {
                return Err(Error::invalid("同一次保存不能包含重复的话术标识"));
            }
        }
        let _guard = self.write.lock().await;
        let w = self.required_workspace().await?;
        let mut effective: BTreeMap<String, ScriptNode> = BTreeMap::new();
        let tables = if w.role == Role::Admin {
            vec!["draft_nodes"]
        } else {
            vec!["personal_dependencies", "official_nodes", "personal_nodes"]
        };
        for table in tables {
            for n in self.list::<ScriptNode>(table).await? {
                effective.insert(n.id.clone(), n);
            }
        }
        let conversions: BTreeSet<_> = nodes
            .iter()
            .filter(|node| {
                node.kind == ScriptKind::Quick
                    && effective
                        .get(&node.id)
                        .is_none_or(|n| n.kind != ScriptKind::Quick)
            })
            .map(|node| node.id.clone())
            .collect();
        for node in &nodes {
            effective.insert(node.id.clone(), node.clone());
        }
        if effective.values().any(|n| {
            n.branches
                .iter()
                .any(|b| conversions.contains(&b.target_id))
        }) {
            return Err(Error::invalid("此话术仍被流程引用，请先移除指向它的连接"));
        }
        for node in &nodes {
            if node.branches.iter().any(|b| {
                effective
                    .get(&b.target_id)
                    .is_some_and(|n| n.kind == ScriptKind::Quick)
            }) {
                return Err(Error::invalid("流程分支只能连接流程话术，请选择其他下一步"));
            }
        }
        let flows = self.effective_flows(&w.role).await?;
        validate_flow_membership(&effective.values().cloned().collect::<Vec<_>>(), &flows)?;
        let mut tx = self.pool.begin().await?;
        if w.role == Role::Sales {
            for flow in self.list::<Flow>("official_flows").await? {
                if nodes.iter().any(|n| n.flow_id.as_deref() == Some(&flow.id)) {
                    sqlx::query("INSERT INTO personal_flow_dependencies(id,json) VALUES(?,?) ON CONFLICT(id) DO NOTHING")
                        .bind(&flow.id).bind(serde_json::to_string(&flow)?).execute(&mut *tx).await?;
                }
            }
            // Keep official dependencies of all changed nodes in the same transaction.
            let official: BTreeMap<_, _> = self
                .list::<ScriptNode>("official_nodes")
                .await?
                .into_iter()
                .map(|n| (n.id.clone(), n))
                .collect();
            let mut seen = BTreeSet::new();
            let mut pending: Vec<_> = nodes
                .iter()
                .flat_map(|n| {
                    std::iter::once(n.id.clone())
                        .chain(n.branches.iter().map(|b| b.target_id.clone()))
                })
                .collect();
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
        }
        let table = if w.role == Role::Admin {
            "draft_nodes"
        } else {
            "personal_nodes"
        };
        for node in &nodes {
            sqlx::query(&format!("INSERT INTO {table}(id,json) VALUES(?,?) ON CONFLICT(id) DO UPDATE SET json=excluded.json"))
                .bind(&node.id).bind(serde_json::to_string(node)?).execute(&mut *tx).await?;
        }
        tx.commit().await?;
        Ok(())
    }
    pub async fn latest(&self) -> Result<Manifest> {
        self.admin().await?;
        let json: Option<String> =
            sqlx::query_scalar("SELECT json FROM releases ORDER BY version DESC LIMIT 1")
                .fetch_optional(&self.pool)
                .await?;
        crate::folders::decode_manifest(
            json.ok_or_else(|| Error::conflict("管理员尚未发布资料"))?
                .as_bytes(),
        )
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
        let mut retained_flows = BTreeSet::new();
        for node in self
            .list::<ScriptNode>("personal_nodes")
            .await?
            .into_iter()
            .chain(self.list::<ScriptNode>("personal_dependencies").await?)
        {
            if let Some(flow_id) = node.flow_id {
                retained_flows.insert(flow_id);
            }
        }
        for flow in self.list::<Flow>("personal_flow_dependencies").await? {
            if !retained_flows.contains(&flow.id) {
                sqlx::query("DELETE FROM personal_flow_dependencies WHERE id=?")
                    .bind(flow.id)
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
        crate::folders::validate(&self.list::<Folder>("draft_folders").await?, &materials)?;
        let flows = self.list::<Flow>("draft_flows").await?;
        validate_flow_membership(&nodes, &flows)?;
        let max: Option<i64> = sqlx::query_scalar("SELECT MAX(version) FROM releases")
            .fetch_one(&self.pool)
            .await?;
        workspace.version = workspace.version.max(max.unwrap_or(0)) + 1;
        let manifest = Manifest {
            format_version: 2,
            group_id: workspace.group_id.clone(),
            group_name: workspace.name.clone(),
            version: workspace.version,
            nodes,
            flows,
            materials,
            folders: self.list("draft_folders").await?,
            material_categories: self.list("draft_material_categories").await?,
            quick_categories: self.list("draft_quick_categories").await?,
        };
        let mut tx = self.pool.begin().await?;
        sqlx::query("DELETE FROM official_folders")
            .execute(&mut *tx)
            .await?;
        sqlx::query("INSERT INTO official_folders SELECT * FROM draft_folders")
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM official_quick_categories")
            .execute(&mut *tx)
            .await?;
        sqlx::query("INSERT INTO official_quick_categories SELECT * FROM draft_quick_categories")
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM official_flows")
            .execute(&mut *tx)
            .await?;
        sqlx::query("INSERT INTO official_flows SELECT * FROM draft_flows")
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM official_nodes")
            .execute(&mut *tx)
            .await?;
        sqlx::query("INSERT INTO official_nodes SELECT * FROM draft_nodes")
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM official_materials")
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM official_material_categories")
            .execute(&mut *tx)
            .await?;
        sqlx::query(
            "INSERT INTO official_material_categories SELECT * FROM draft_material_categories",
        )
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
        if manifest.format_version != 2 {
            return Err(Error::invalid("不支持此资料格式"));
        }
        validate_manifest_content(&manifest.nodes, &manifest.materials)?;
        crate::folders::validate(&manifest.folders, &manifest.materials)?;
        validate_flow_membership(&manifest.nodes, &manifest.flows)?;
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
        sqlx::query("DELETE FROM official_folders")
            .execute(&mut *tx)
            .await?;
        for folder in &manifest.folders {
            sqlx::query("INSERT INTO official_folders(id,json) VALUES(?,?)")
                .bind(&folder.id)
                .bind(serde_json::to_string(folder)?)
                .execute(&mut *tx)
                .await?;
        }
        sqlx::query("DELETE FROM official_quick_categories")
            .execute(&mut *tx)
            .await?;
        for category in &manifest.quick_categories {
            sqlx::query("INSERT INTO official_quick_categories(id,json) VALUES(?,?) ON CONFLICT(id) DO NOTHING")
                .bind(category).bind(serde_json::to_string(category)?).execute(&mut *tx).await?;
        }
        sqlx::query("DELETE FROM official_flows")
            .execute(&mut *tx)
            .await?;
        for flow in &manifest.flows {
            sqlx::query("INSERT INTO official_flows(id,json) VALUES(?,?)")
                .bind(&flow.id)
                .bind(serde_json::to_string(flow)?)
                .execute(&mut *tx)
                .await?;
        }
        sqlx::query("DELETE FROM official_nodes")
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM official_materials")
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM official_material_categories")
            .execute(&mut *tx)
            .await?;
        for name in &manifest.material_categories {
            sqlx::query("INSERT OR IGNORE INTO official_material_categories(id,json) VALUES(?,?)")
                .bind(name)
                .bind(serde_json::to_string(name)?)
                .execute(&mut *tx)
                .await?;
        }
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
        "draft_folders"
        | "official_folders"
        | "draft_quick_categories"
        | "official_quick_categories"
        | "personal_quick_categories"
        | "draft_flows"
        | "official_flows"
        | "personal_flows"
        | "personal_flow_dependencies"
        | "official_nodes"
        | "draft_nodes"
        | "personal_nodes"
        | "personal_dependencies"
        | "official_materials"
        | "draft_materials"
        | "draft_material_categories"
        | "official_material_categories" => Ok(table),
        _ => Err(Error::invalid("未知数据表")),
    }
}
pub fn validate_flow(flow: &Flow) -> Result<()> {
    if flow.id.is_empty()
        || flow.id.len() > 100
        || flow.name.trim().is_empty()
        || flow.name.chars().count() > 80
        || flow.name.chars().any(char::is_control)
    {
        return Err(Error::invalid("流程需要有效标识和名称，名称最多 80 个字"));
    }
    Ok(())
}
pub fn validate_flow_membership(nodes: &[ScriptNode], flows: &[Flow]) -> Result<()> {
    if flows.len() > 10_000 {
        return Err(Error::invalid("流程数量超过限制"));
    }
    let mut ids = BTreeSet::new();
    for flow in flows {
        validate_flow(flow)?;
        if !ids.insert(&flow.id) {
            return Err(Error::invalid("流程标识重复"));
        }
    }
    let map: BTreeMap<_, _> = nodes.iter().map(|n| (&n.id, n)).collect();
    for node in nodes {
        if node.kind == ScriptKind::Quick {
            if node.flow_id.is_some() {
                return Err(Error::invalid("快捷话术不属于流程"));
            }
            continue;
        }
        if node.flow_id.as_ref().is_none_or(|id| !ids.contains(id)) {
            return Err(Error::invalid("请先选择或新建所属流程"));
        }
        if node.branches.iter().any(|b| {
            map.get(&b.target_id)
                .is_some_and(|target| target.flow_id != node.flow_id)
        }) {
            return Err(Error::invalid("分支只能连接同一流程中的话术"));
        }
    }
    Ok(())
}
pub fn validate_node(n: &ScriptNode) -> Result<()> {
    if n.id.is_empty() || n.id.len() > 100 || n.title.trim().is_empty() || n.title.len() > 500 {
        return Err(Error::invalid("话术需要有效标识和标题"));
    }
    if n.replies.len() > 50 || n.branches.len() > 200 || n.variables.len() > 100 {
        return Err(Error::invalid("话术内容超过限制"));
    }
    if n.reply_options.len() > 49 || (n.kind != ScriptKind::Quick && !n.reply_options.is_empty()) {
        return Err(Error::invalid("仅快捷话术支持多条回复，最多 50 条"));
    }
    let mut option_ids = BTreeSet::new();
    for option in &n.reply_options {
        if option.id.is_empty()
            || option.id.len() > 100
            || !option_ids.insert(&option.id)
            || option.name.trim().is_empty()
            || option.name.chars().count() > 80
            || option.replies.len() > 50
        {
            return Err(Error::invalid("回复需要有效且不重复的标识及名称"));
        }
    }
    if n.kind == ScriptKind::Quick && (n.is_entry || !n.branches.is_empty()) {
        return Err(Error::invalid(
            "快捷话术不参与流程，请先取消流程入口并移除分支",
        ));
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
pub fn validate_script_kinds(nodes: &[ScriptNode]) -> Result<()> {
    let quick: BTreeSet<_> = nodes
        .iter()
        .filter(|n| n.kind == ScriptKind::Quick)
        .map(|n| &n.id)
        .collect();
    for n in nodes {
        if n.branches.iter().any(|b| quick.contains(&b.target_id)) {
            return Err(Error::invalid(format!(
                "话术「{}」仍连接到快捷话术，请先移除该流程连接",
                n.title
            )));
        }
    }
    Ok(())
}
pub fn validate_manifest_content(nodes: &[ScriptNode], materials: &[Material]) -> Result<()> {
    validate_script_kinds(nodes)?;
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
        let image_ids: BTreeSet<_> = m.image_ids.iter().collect();
        if (m.kind != "case" && !m.image_ids.is_empty())
            || m.image_ids.len() > 50
            || image_ids.len() != m.image_ids.len()
            || m.image_ids.iter().any(|id| {
                materials
                    .iter()
                    .any(|image| image.id == *id && image.kind != "image")
            })
        {
            return Err(Error::invalid("案例图片引用无效"));
        }
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
