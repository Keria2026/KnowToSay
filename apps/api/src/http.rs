use crate::{
    backup,
    db::AppState,
    error::{Error, Result},
    files,
    models::*,
    sync,
};
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Multipart, Path, Query, Request, State},
    http::{HeaderMap, Method, StatusCode},
    middleware::{self, Next},
    response::Response,
    routing::{delete, get, post, put},
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use tower_http::cors::CorsLayer;
use utoipa::ToSchema;

pub fn local_router(state: AppState) -> Router {
    let upload = Router::new()
        .route("/api/v1/materials/upload", post(upload_materials))
        .layer(DefaultBodyLimit::disable());
    Router::new()
        .route("/api/v1/snapshot", get(snapshot))
        .route("/api/v1/group", post(create_group))
        .route("/api/v1/group/join", post(join_group))
        .route("/api/v1/group/discover", get(discover))
        .route("/api/v1/group/address", put(update_address))
        .route("/api/v1/scripts/{id}", put(save_node).delete(delete_node))
        .route("/api/v1/scripts", put(save_nodes))
        .route("/api/v1/flows/{id}", put(save_flow))
        .route("/api/v1/quick-categories", post(create_quick_category))
        .route(
            "/api/v1/quick-categories/{name}",
            put(rename_quick_category).delete(delete_quick_category),
        )
        .route("/api/v1/scripts/{id}/reset", post(reset_node))
        .route("/api/v1/scripts/import", post(import_scripts))
        .route("/api/v1/materials", post(save_material))
        .route(
            "/api/v1/material-categories",
            post(create_material_category),
        )
        .route("/api/v1/materials/{id}", delete(delete_material))
        .route("/api/v1/materials/{id}/file", get(material_file))
        .route("/api/v1/materials/{id}/rows", get(material_rows))
        .route("/api/v1/search/materials", get(search_materials))
        .route("/api/v1/publish", post(publish))
        .route("/api/v1/sync", post(start_sync))
        .route("/api/v1/sync/status", get(sync_status))
        .route("/api/v1/demo", post(demo))
        .route("/api/v1/backup", post(create_backup))
        .route("/api/v1/backup/import", post(import_backup))
        .route("/api/v1/restore", post(restore_backup))
        .route("/api/v1/package/export", post(export_package))
        .route("/api/v1/package/import", post(import_package))
        .route("/api/v1/personal/export", post(export_personal))
        .route("/api/v1/personal/import", post(import_personal))
        .route("/api/v1/manifest", get(manifest))
        .merge(upload)
        .layer(DefaultBodyLimit::max(64 * 1024 * 1024))
        .layer(middleware::from_fn(origin_guard))
        .layer(
            CorsLayer::new()
                .allow_origin([
                    "http://127.0.0.1:1420".parse().unwrap(),
                    "http://localhost:1420".parse().unwrap(),
                    "http://127.0.0.1:1421".parse().unwrap(),
                    "tauri://localhost".parse().unwrap(),
                    "http://tauri.localhost".parse().unwrap(),
                    "https://tauri.localhost".parse().unwrap(),
                ])
                .allow_methods([
                    Method::GET,
                    Method::POST,
                    Method::PUT,
                    Method::DELETE,
                    Method::OPTIONS,
                ])
                .allow_headers([
                    axum::http::header::CONTENT_TYPE,
                    axum::http::HeaderName::from_static("x-knowtosay-local"),
                ]),
        )
        .with_state(state)
}

pub fn lan_router(state: AppState) -> Router {
    // The LAN exposes published content only, never local editing/backup endpoints.
    Router::new()
        .route("/api/v1/manifest", get(manifest))
        .route("/api/v1/blobs/{hash}", get(lan_blob))
        .with_state(state)
}

async fn origin_guard(request: Request, next: Next) -> Result<Response> {
    if !matches!(
        *request.method(),
        Method::GET | Method::HEAD | Method::OPTIONS
    ) {
        if request
            .headers()
            .get("x-knowtosay-local")
            .and_then(|v| v.to_str().ok())
            != Some("1")
        {
            return Err(Error(
                StatusCode::FORBIDDEN,
                "LOCAL_REQUEST_REQUIRED".into(),
                "仅允许本机客户端编辑资料".into(),
            ));
        }
        if let Some(origin) = request.headers().get("origin") {
            let accepted = [
                "http://localhost:1420",
                "http://127.0.0.1:1420",
                "http://127.0.0.1:1421",
                "tauri://localhost",
                "http://tauri.localhost",
                "https://tauri.localhost",
            ];
            if !accepted.iter().any(|x| origin == *x) {
                return Err(Error(
                    StatusCode::FORBIDDEN,
                    "ORIGIN_DENIED".into(),
                    "请求来源不受支持".into(),
                ));
            }
        }
    }
    Ok(next.run(request).await)
}

#[utoipa::path(get,path="/api/v1/snapshot",responses((status=200,body=Snapshot)))]
pub async fn snapshot(State(state): State<AppState>) -> Result<Json<Snapshot>> {
    Ok(Json(state.snapshot().await?))
}

#[utoipa::path(post,path="/api/v1/group",request_body=CreateGroup,responses((status=200,body=Workspace)))]
pub async fn create_group(
    State(state): State<AppState>,
    Json(input): Json<CreateGroup>,
) -> Result<Json<Workspace>> {
    let _guard = state.write.lock().await;
    if state.workspace().await?.is_some() {
        return Err(Error::conflict("这台电脑已创建或加入工作组"));
    }
    if input.name.trim().is_empty() || input.name.len() > 200 {
        return Err(Error::invalid("请输入有效的工作组名称"));
    }
    let w = Workspace {
        group_id: uuid::Uuid::new_v4().to_string(),
        name: input.name.trim().into(),
        role: Role::Admin,
        version: 0,
        admin_address: None,
    };
    sqlx::query("INSERT INTO workspace(id,json) VALUES(1,?)")
        .bind(serde_json::to_string(&w)?)
        .execute(&state.pool)
        .await?;
    Ok(Json(w))
}

#[utoipa::path(post,path="/api/v1/group/join",request_body=JoinGroup,responses((status=200,body=Snapshot)))]
pub async fn join_group(
    State(state): State<AppState>,
    Json(input): Json<JoinGroup>,
) -> Result<Json<Snapshot>> {
    if state.workspace().await?.is_some() {
        return Err(Error::conflict("这台电脑已经属于一个工作组"));
    }
    sync::synchronize(&state, &input.address).await?;
    Ok(Json(state.snapshot().await?))
}

#[utoipa::path(get,path="/api/v1/group/discover",responses((status=200,body=Vec<Discovery>)))]
pub async fn discover() -> Result<Json<Vec<Discovery>>> {
    Ok(Json(sync::discover().await?))
}

pub async fn update_address(
    State(state): State<AppState>,
    Json(input): Json<JoinGroup>,
) -> Result<Json<Workspace>> {
    let address = sync::normalize_address(&input.address)?;
    let manifest = sync::fetch_manifest(&state, &address).await?;
    let _guard = state.write.lock().await;
    let mut w = state.required_workspace().await?;
    if w.role != Role::Sales || w.group_id != manifest.group_id {
        return Err(Error::conflict("此地址不属于当前业务员工作组"));
    }
    w.admin_address = Some(address);
    sqlx::query("UPDATE workspace SET json=? WHERE id=1")
        .bind(serde_json::to_string(&w)?)
        .execute(&state.pool)
        .await?;
    Ok(Json(w))
}

#[utoipa::path(put,path="/api/v1/flows/{id}",request_body=Flow,params(("id"=String,Path)),responses((status=200,body=Snapshot)))]
pub async fn save_flow(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(flow): Json<Flow>,
) -> Result<Json<Snapshot>> {
    if id != flow.id {
        return Err(Error::invalid("流程标识不匹配"));
    }
    state.save_flow(flow).await?;
    Ok(Json(state.snapshot().await?))
}

#[utoipa::path(put,path="/api/v1/scripts/{id}",request_body=ScriptNode,params(("id"=String,Path)),responses((status=200,body=Snapshot)))]
pub async fn save_node(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(node): Json<ScriptNode>,
) -> Result<Json<Snapshot>> {
    if id != node.id {
        return Err(Error::invalid("话术标识不一致"));
    }
    state.save_node(node).await?;
    Ok(Json(state.snapshot().await?))
}

#[utoipa::path(put,path="/api/v1/scripts",request_body=SaveScripts,responses((status=200,body=Snapshot)))]
pub async fn save_nodes(
    State(state): State<AppState>,
    Json(input): Json<SaveScripts>,
) -> Result<Json<Snapshot>> {
    state.save_nodes(input.nodes).await?;
    Ok(Json(state.snapshot().await?))
}

pub async fn delete_node(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Snapshot>> {
    let _guard = state.write.lock().await;
    let w = state.required_workspace().await?;
    if w.role == Role::Admin {
        sqlx::query("DELETE FROM draft_nodes WHERE id=?")
            .bind(id)
            .execute(&state.pool)
            .await?;
    } else {
        let exists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM official_nodes WHERE id=?)")
                .bind(&id)
                .fetch_one(&state.pool)
                .await?;
        if exists {
            return Err(Error::invalid(
                "官方话术可编辑个人版本或恢复官方，不能直接删除",
            ));
        }
        sqlx::query("DELETE FROM personal_nodes WHERE id=?")
            .bind(id)
            .execute(&state.pool)
            .await?;
        state.prune_dependencies().await?;
    }
    drop(_guard);
    Ok(Json(state.snapshot().await?))
}

pub async fn reset_node(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Snapshot>> {
    let _guard = state.write.lock().await;
    if state.required_workspace().await?.role != Role::Sales {
        return Err(Error::invalid("管理员无需恢复个人版本"));
    }
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM official_nodes WHERE id=?)")
        .bind(&id)
        .fetch_one(&state.pool)
        .await?;
    if !exists {
        return Err(Error::conflict("此话术已无当前官方版本，个人内容保持不变"));
    }
    sqlx::query("DELETE FROM personal_nodes WHERE id=?")
        .bind(id)
        .execute(&state.pool)
        .await?;
    state.prune_dependencies().await?;
    drop(_guard);
    Ok(Json(state.snapshot().await?))
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ImportScripts {
    pub flow_id: Option<String>,
    pub material_id: String,
    pub mapping: BTreeMap<String, String>,
    pub preview: bool,
}
#[utoipa::path(post,path="/api/v1/scripts/import",request_body=ImportScripts,responses((status=200,body=Vec<ScriptNode>)))]
pub async fn import_scripts(
    State(state): State<AppState>,
    Json(input): Json<ImportScripts>,
) -> Result<Json<Vec<ScriptNode>>> {
    let _guard = state.write.lock().await;
    state.admin().await?;
    let m = state
        .list::<Material>("draft_materials")
        .await?
        .into_iter()
        .find(|m| m.id == input.material_id)
        .ok_or_else(|| Error::missing("请选择已导入的 Excel 素材"))?;
    let mut nodes = files::scripts_from_rows(&m.rows, &input.mapping)?;
    for node in &mut nodes {
        if node.kind == ScriptKind::Flow {
            node.flow_id = input.flow_id.clone();
        }
    }
    let mut effective: BTreeMap<_, _> = state
        .list::<ScriptNode>("draft_nodes")
        .await?
        .into_iter()
        .map(|n| (n.id.clone(), n))
        .collect();
    effective.extend(nodes.iter().cloned().map(|n| (n.id.clone(), n)));
    let effective: Vec<_> = effective.into_values().collect();
    crate::db::validate_script_kinds(&effective)?;
    crate::db::validate_flow_membership(&effective, &state.list::<Flow>("draft_flows").await?)?;
    if !input.preview {
        let mut tx = state.pool.begin().await?;
        for node in &nodes {
            sqlx::query("INSERT INTO draft_nodes(id,json) VALUES(?,?) ON CONFLICT(id) DO UPDATE SET json=excluded.json").bind(&node.id).bind(serde_json::to_string(node)?).execute(&mut *tx).await?;
        }
        tx.commit().await?;
    }
    Ok(Json(nodes))
}

#[utoipa::path(post,path="/api/v1/materials",request_body=Material,responses((status=200,body=Snapshot)))]
pub async fn save_material(
    State(state): State<AppState>,
    Json(mut material): Json<Material>,
) -> Result<Json<Snapshot>> {
    let _guard = state.write.lock().await;
    state.admin().await?;
    if material.name.trim().is_empty() {
        return Err(Error::invalid("请输入素材名称"));
    }
    if material.id.is_empty() {
        material.id = uuid::Uuid::new_v4().to_string()
    }
    let old = state
        .list::<Material>("draft_materials")
        .await?
        .into_iter()
        .find(|m| m.id == material.id);
    if let Some(old) = old {
        if old.hash != material.hash || old.size != material.size || old.kind != material.kind {
            return Err(Error::invalid("请通过文件导入更新素材文件"));
        }
        material.rows = old.rows;
        material.mime = old.mime;
    } else if material.hash.is_some() || material.kind != "text" {
        return Err(Error::invalid("文件素材需要通过导入添加"));
    }
    state
        .put("draft_materials", &material.id, &material)
        .await?;
    drop(_guard);
    Ok(Json(state.snapshot().await?))
}

#[utoipa::path(post,path="/api/v1/quick-categories",request_body=CreateQuickCategory,responses((status=200,body=Snapshot)))]
pub async fn create_quick_category(
    State(state): State<AppState>,
    Json(input): Json<CreateQuickCategory>,
) -> Result<Json<Snapshot>> {
    state.create_quick_category(&input.name).await?;
    Ok(Json(state.snapshot().await?))
}

#[utoipa::path(put,path="/api/v1/quick-categories/{name}",params(("name"=String,Path)),request_body=CreateQuickCategory,responses((status=200,body=Snapshot)))]
pub async fn rename_quick_category(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(input): Json<CreateQuickCategory>,
) -> Result<Json<Snapshot>> {
    state
        .update_quick_category(&name, Some(&input.name))
        .await?;
    Ok(Json(state.snapshot().await?))
}
#[utoipa::path(delete,path="/api/v1/quick-categories/{name}",params(("name"=String,Path)),responses((status=200,body=Snapshot)))]
pub async fn delete_quick_category(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<Snapshot>> {
    state.update_quick_category(&name, None).await?;
    Ok(Json(state.snapshot().await?))
}

#[utoipa::path(post,path="/api/v1/material-categories",request_body=CreateMaterialCategory,responses((status=200,body=Snapshot)))]
pub async fn create_material_category(
    State(state): State<AppState>,
    Json(input): Json<CreateMaterialCategory>,
) -> Result<Json<Snapshot>> {
    state.create_material_category(&input.name).await?;
    Ok(Json(state.snapshot().await?))
}

pub async fn upload_materials(
    State(state): State<AppState>,
    multipart: Multipart,
) -> Result<Json<Vec<Material>>> {
    Ok(Json(files::upload(&state, multipart).await?))
}

pub async fn delete_material(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Snapshot>> {
    let _guard = state.write.lock().await;
    state.admin().await?;
    sqlx::query("DELETE FROM draft_materials WHERE id=?")
        .bind(id)
        .execute(&state.pool)
        .await?;
    drop(_guard);
    Ok(Json(state.snapshot().await?))
}

#[derive(Deserialize)]
pub struct DownloadQuery {
    #[serde(default)]
    pub download: bool,
}
pub async fn material_file(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(q): Query<DownloadQuery>,
    headers: HeaderMap,
) -> Result<Response> {
    let snapshot = state.snapshot().await?;
    let m = snapshot
        .materials
        .into_iter()
        .find(|m| m.id == id)
        .ok_or_else(|| Error::missing("素材已移除"))?;
    let hash = m
        .hash
        .as_ref()
        .ok_or_else(|| Error::invalid("文字素材请直接复制正文"))?;
    files::stream_blob(&state, hash, m.size, &m.mime, &headers, q.download).await
}

#[derive(Deserialize, Default)]
pub struct SearchQuery {
    #[serde(default)]
    pub q: String,
    #[serde(default)]
    pub offset: usize,
}
#[utoipa::path(get,path="/api/v1/search/materials",params(("q"=String,Query)),responses((status=200,body=Vec<MaterialSearchResult>)))]
pub async fn search_materials(
    State(state): State<AppState>,
    Query(q): Query<SearchQuery>,
) -> Result<Json<Vec<MaterialSearchResult>>> {
    Ok(Json(crate::search::search(&state, &q.q).await?))
}
#[utoipa::path(get,path="/api/v1/materials/{id}/rows",params(("id"=String,Path),("q"=Option<String>,Query),("offset"=Option<usize>,Query)),responses((status=200,body=SheetPage)))]
pub async fn material_rows(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(q): Query<SearchQuery>,
) -> Result<Json<SheetPage>> {
    Ok(Json(
        crate::search::rows(&state, &id, &q.q, q.offset).await?,
    ))
}

#[utoipa::path(post,path="/api/v1/publish",responses((status=200,body=Manifest)))]
pub async fn publish(State(state): State<AppState>) -> Result<Json<Manifest>> {
    Ok(Json(state.publish().await?))
}

#[utoipa::path(get,path="/api/v1/manifest",responses((status=200,body=Manifest)))]
pub async fn manifest(State(state): State<AppState>) -> Result<Json<Manifest>> {
    Ok(Json(state.latest().await?))
}

pub async fn lan_blob(
    State(state): State<AppState>,
    Path(hash): Path<String>,
    headers: HeaderMap,
) -> Result<Response> {
    let manifest = state.latest().await?;
    let m = manifest
        .materials
        .iter()
        .find(|m| m.hash.as_deref() == Some(&hash))
        .ok_or_else(|| Error::missing("此素材不在当前发布版本中"))?;
    files::stream_blob(
        &state,
        &hash,
        m.size,
        "application/octet-stream",
        &headers,
        false,
    )
    .await
}

#[utoipa::path(post,path="/api/v1/sync",responses((status=200,body=SyncStatus)))]
pub async fn start_sync(State(state): State<AppState>) -> Result<Json<SyncStatus>> {
    let w = state.required_workspace().await?;
    if w.role != Role::Sales {
        return Err(Error::invalid("管理员请使用发布功能"));
    }
    let address = w
        .admin_address
        .filter(|a| !a.is_empty())
        .ok_or_else(|| Error::invalid("当前通过资料包加入，请在同步页设置管理员地址"))?;
    let cloned = state.clone();
    tokio::spawn(async move {
        let _ = sync::synchronize(&cloned, &address).await;
    });
    Ok(Json(state.progress.read().await.clone()))
}

#[utoipa::path(get,path="/api/v1/sync/status",responses((status=200,body=SyncStatus)))]
pub async fn sync_status(State(state): State<AppState>) -> Json<SyncStatus> {
    Json(state.progress.read().await.clone())
}

pub async fn create_backup(
    State(state): State<AppState>,
    Json(input): Json<BackupRequest>,
) -> Result<Json<FileResult>> {
    Ok(Json(backup::backup(&state, input.directory).await?))
}
pub async fn restore_backup(
    State(state): State<AppState>,
    Json(input): Json<RestoreRequest>,
) -> Result<Json<Snapshot>> {
    backup::restore(&state, &input.directory).await?;
    Ok(Json(state.snapshot().await?))
}
#[utoipa::path(post,path="/api/v1/backup/import",request_body=RestoreRequest,responses((status=200,body=Snapshot)))]
pub async fn import_backup(
    State(state): State<AppState>,
    Json(input): Json<RestoreRequest>,
) -> Result<Json<Snapshot>> {
    backup::import_as_draft(&state, &input.directory).await?;
    Ok(Json(state.snapshot().await?))
}
pub async fn export_package(
    State(state): State<AppState>,
    Json(input): Json<BackupRequest>,
) -> Result<Json<FileResult>> {
    Ok(Json(backup::export_package(&state, input.directory).await?))
}
pub async fn import_package(
    State(state): State<AppState>,
    Json(input): Json<RestoreRequest>,
) -> Result<Json<Snapshot>> {
    backup::import_package(&state, &input.directory).await?;
    Ok(Json(state.snapshot().await?))
}

pub async fn demo(State(state): State<AppState>) -> Result<Json<Snapshot>> {
    state.admin().await?;
    if !state.list::<ScriptNode>("draft_nodes").await?.is_empty() {
        return Err(Error::conflict("示例仅可添加到空的话术库"));
    }
    state
        .save_flow(Flow {
            id: "demo-flow".into(),
            name: "销售入门（示例）".into(),
        })
        .await?;
    let material = Material {
        id: "demo-intro".into(),
        name: "产品介绍（示例）".into(),
        kind: "text".into(),
        category: "产品资料".into(),
        tags: vec!["示例".into()],
        notes: "这是演示内容，请替换为真实销售资料后再发布。".into(),
        text: "提供客户需要的信息，核对真实型号、价格与服务范围。".into(),
        mime: "text/plain".into(),
        hash: None,
        size: 0,
        rows: vec![],
    };
    state
        .put("draft_materials", &material.id, &material)
        .await?;
    for (id, title, question, zh, ko, branches) in [
        (
            "demo-start",
            "开场问候",
            "先与客户建立沟通",
            "您好，{客户姓名}，您吃饭了吗？",
            "{客户姓名}님, 안녕하세요. 식사하셨어요?",
            vec![("吃了", "demo-yes"), ("还没吃", "demo-no")],
        ),
        (
            "demo-yes",
            "已经用餐",
            "回应客户已经用餐",
            "好的，希望您今天一切顺利！",
            "네, 오늘도 좋은 하루 보내세요!",
            vec![("继续介绍", "demo-product")],
        ),
        (
            "demo-no",
            "尚未用餐",
            "回应客户尚未用餐",
            "记得按时吃饭，照顾好自己。",
            "식사 꼭 챙기시고 건강 잘 챙기세요.",
            vec![("继续介绍", "demo-product")],
        ),
        (
            "demo-product",
            "介绍产品",
            "共同的产品介绍步骤",
            "这款产品的价格是 {价格}，我把详细资料发给您。",
            "이 제품의 가격은 {价格}입니다. 자세한 자료를 보내드릴게요.",
            vec![],
        ),
    ] {
        let variables = if id == "demo-start" {
            vec![Variable {
                name: "客户姓名".into(),
                label: "客户姓名".into(),
                required: true,
            }]
        } else if id == "demo-product" {
            vec![Variable {
                name: "价格".into(),
                label: "价格".into(),
                required: true,
            }]
        } else {
            vec![]
        };
        let node = ScriptNode {
            id: id.into(),
            title: title.into(),
            question: question.into(),
            category: String::new(),
            kind: ScriptKind::Flow,
            flow_id: Some("demo-flow".into()),
            is_entry: id == "demo-start",
            replies: BTreeMap::from([("zh-CN".into(), zh.into()), ("ko".into(), ko.into())]),
            variables,
            branches: branches
                .into_iter()
                .map(|(label, target)| Branch {
                    id: uuid::Uuid::new_v4().to_string(),
                    label: label.into(),
                    target_id: target.into(),
                })
                .collect(),
            material_ids: if id == "demo-product" {
                vec!["demo-intro".into()]
            } else {
                vec![]
            },
            position: None,
        };
        state.save_node(node).await?;
    }
    Ok(Json(state.snapshot().await?))
}

#[utoipa::path(post,path="/api/v1/personal/export",request_body=BackupRequest,responses((status=200,body=FileResult)))]
pub async fn export_personal(
    State(state): State<AppState>,
    Json(input): Json<BackupRequest>,
) -> Result<Json<FileResult>> {
    Ok(Json(
        crate::personal::export(&state, input.directory).await?,
    ))
}
#[utoipa::path(post,path="/api/v1/personal/import",request_body=RestoreRequest,responses((status=200,body=Snapshot)))]
pub async fn import_personal(
    State(state): State<AppState>,
    Json(input): Json<RestoreRequest>,
) -> Result<Json<Snapshot>> {
    crate::personal::import(&state, &input.directory).await?;
    Ok(Json(state.snapshot().await?))
}
