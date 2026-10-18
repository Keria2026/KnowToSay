use axum::{
    body::{Body, to_bytes},
    extract::Request as AxumRequest,
    http::{Request, StatusCode},
    middleware::{self, Next},
};
use knowtosay::{backup, db::AppState, files, http, models::*, sync};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};
use tower::ServiceExt;

async fn local(state: &AppState, method: &str, path: &str, value: Value) -> (StatusCode, Value) {
    let request = Request::builder()
        .method(method)
        .uri(format!("/api/v1{path}"))
        .header("content-type", "application/json")
        .header("x-knowtosay-local", "1")
        .body(Body::from(value.to_string()))
        .unwrap();
    let response = http::local_router(state.clone())
        .oneshot(request)
        .await
        .unwrap();
    let status = response.status();
    let body = to_bytes(response.into_body(), 64 * 1024 * 1024)
        .await
        .unwrap();
    (
        status,
        serde_json::from_slice(&body)
            .unwrap_or_else(|_| json!({"raw":String::from_utf8_lossy(&body)})),
    )
}
async fn create_admin(root: &std::path::Path) -> AppState {
    let s = AppState::open(root).await.unwrap();
    assert_eq!(
        local(&s, "POST", "/group", json!({"name":"测试工作组"}))
            .await
            .0,
        StatusCode::OK
    );
    s
}
fn node(id: &str, text: &str) -> ScriptNode {
    ScriptNode {
        id: id.into(),
        title: format!("话术{id}"),
        question: "测试问题".into(),
        category: "测试".into(),
        is_entry: true,
        replies: BTreeMap::from([("ko".into(), text.into())]),
        variables: vec![],
        branches: vec![],
        material_ids: vec![],
        position: None,
    }
}
async fn binary_material(state: &AppState, name: &str, content: &[u8]) -> Material {
    let source = state.root.join(name);
    tokio::fs::write(&source, content).await.unwrap();
    files::import_paths(state, vec![source.to_string_lossy().into()])
        .await
        .unwrap()
        .remove(0)
}
async fn server(state: AppState, ranges: Option<Arc<Mutex<Vec<String>>>>) -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let router = if let Some(ranges) = ranges {
        http::lan_router(state).layer(middleware::from_fn(
            move |request: AxumRequest, next: Next| {
                let ranges = ranges.clone();
                async move {
                    if let Some(range) = request.headers().get("range") {
                        ranges.lock().unwrap().push(range.to_str().unwrap().into());
                    }
                    next.run(request).await
                }
            },
        ))
    } else {
        http::lan_router(state)
    };
    tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    format!("http://{address}")
}

#[tokio::test]
async fn explicit_initialization_roles_and_draft_visibility() {
    let dir = tempfile::tempdir().unwrap();
    let admin = create_admin(&dir.path().join("admin")).await;
    assert_eq!(
        local(&admin, "POST", "/group", json!({"name":"重复创建"}))
            .await
            .0,
        StatusCode::CONFLICT
    );
    admin.save_node(node("one", "发布版本")).await.unwrap();
    let address = server(admin.clone(), None).await;
    let sales = AppState::open(dir.path().join("sales")).await.unwrap();
    assert!(sync::synchronize(&sales, &address).await.is_err());
    assert!(sales.workspace().await.unwrap().is_none());
    admin.publish().await.unwrap();
    sync::synchronize(&sales, &address).await.unwrap();
    let w = sales.required_workspace().await.unwrap();
    assert_eq!(w.role, Role::Sales);
    admin
        .save_node(node("one", "尚未发布的草稿"))
        .await
        .unwrap();
    sync::synchronize(&sales, &address).await.unwrap();
    assert_eq!(
        sales.snapshot().await.unwrap().nodes[0].node.replies["ko"],
        "发布版本"
    );
    assert_eq!(
        local(&sales, "POST", "/publish", Value::Null).await.0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        local(
            &sales,
            "POST",
            "/materials",
            serde_json::to_value(Material {
                id: "x".into(),
                name: "非法".into(),
                kind: "text".into(),
                category: "".into(),
                tags: vec![],
                notes: "".into(),
                text: "".into(),
                mime: "text/plain".into(),
                hash: None,
                size: 0,
                rows: vec![]
            })
            .unwrap()
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    let res = reqwest::Client::builder()
        .no_proxy()
        .build()
        .unwrap()
        .post(format!("{address}/api/v1/group"))
        .json(&json!({"name":"LAN write"}))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn personal_edits_new_nodes_edges_and_removed_material_survive_sync() {
    let dir = tempfile::tempdir().unwrap();
    let admin = create_admin(&dir.path().join("a")).await;
    let m = binary_material(&admin, "intro.txt", b"official material").await;
    let mut a = node("a", "官方原文");
    a.material_ids = vec![m.id.clone()];
    let b = node("b", "旧B");
    a.branches.push(Branch {
        id: "a-b".into(),
        label: "继续".into(),
        target_id: "b".into(),
    });
    admin.save_node(a.clone()).await.unwrap();
    admin.save_node(b).await.unwrap();
    admin.publish().await.unwrap();
    let address = server(admin.clone(), None).await;
    let sales = AppState::open(dir.path().join("s")).await.unwrap();
    sync::synchronize(&sales, &address).await.unwrap();
    let mut personal = a.clone();
    personal.replies.insert("ko".into(), "我的改写".into());
    personal.branches.push(Branch {
        id: "a-c".into(),
        label: "个人路径".into(),
        target_id: "c".into(),
    });
    sales.save_node(node("c", "个人新增")).await.unwrap();
    sales.save_node(personal.clone()).await.unwrap();
    a.replies.insert("ko".into(), "新版官方".into());
    admin.save_node(a).await.unwrap();
    admin.save_node(node("b", "新版B")).await.unwrap();
    assert_eq!(
        local(
            &admin,
            "DELETE",
            &format!("/materials/{}", m.id),
            Value::Null
        )
        .await
        .0,
        StatusCode::OK
    );
    admin.publish().await.unwrap();
    sync::synchronize(&sales, &address).await.unwrap();
    let view = sales.snapshot().await.unwrap();
    let updated = view.nodes.iter().find(|n| n.node.id == "a").unwrap();
    assert_eq!(updated.node, personal);
    assert_eq!(updated.official.as_ref().unwrap().replies["ko"], "新版官方");
    assert_eq!(
        view.nodes
            .iter()
            .find(|n| n.node.id == "b")
            .unwrap()
            .node
            .replies["ko"],
        "新版B"
    );
    assert!(view.nodes.iter().any(|n| n.node.id == "c"));
    assert!(view.materials.is_empty());
    assert_eq!(
        local(
            &sales,
            "GET",
            &format!("/materials/{}/file", m.id),
            Value::Null
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    assert!(
        sales.root.join("blobs").join(m.hash.unwrap()).exists(),
        "Rollback file may remain, but current material endpoint must reject it"
    );
    let reopened = AppState::open(&*sales.root).await.unwrap();
    assert_eq!(
        reopened
            .snapshot()
            .await
            .unwrap()
            .nodes
            .iter()
            .find(|n| n.node.id == "a")
            .unwrap()
            .node
            .replies["ko"],
        "我的改写"
    );
}

#[tokio::test]
async fn resumed_transfer_uses_range_and_verifies_sha256() {
    let dir = tempfile::tempdir().unwrap();
    let admin = create_admin(&dir.path().join("a")).await;
    let content: Vec<_> = (0..60_000).map(|i| (i % 251) as u8).collect();
    let m = binary_material(&admin, "video.mp4", &content).await;
    admin.save_node(node("a", "hello")).await.unwrap();
    admin.publish().await.unwrap();
    let ranges = Arc::new(Mutex::new(Vec::new()));
    let address = server(admin, Some(ranges.clone())).await;
    let sales = AppState::open(dir.path().join("s")).await.unwrap();
    let hash = m.hash.unwrap();
    tokio::fs::write(
        sales.root.join("staging").join(format!("{hash}.part")),
        &content[..16_000],
    )
    .await
    .unwrap();
    sync::synchronize(&sales, &address).await.unwrap();
    assert_eq!(*ranges.lock().unwrap(), vec!["bytes=16000-"]);
    assert_eq!(
        tokio::fs::read(sales.root.join("blobs").join(hash))
            .await
            .unwrap(),
        content
    );
    assert_eq!(sales.required_workspace().await.unwrap().version, 1);
}

#[tokio::test]
async fn checksum_failure_keeps_current_version_and_personal_changes() {
    let dir = tempfile::tempdir().unwrap();
    let admin = create_admin(&dir.path().join("a")).await;
    admin.save_node(node("a", "v1")).await.unwrap();
    admin.publish().await.unwrap();
    let address = server(admin.clone(), None).await;
    let sales = AppState::open(dir.path().join("s")).await.unwrap();
    sync::synchronize(&sales, &address).await.unwrap();
    sales.save_node(node("a", "个人保留")).await.unwrap();
    let m = binary_material(&admin, "test.mp4", b"original").await;
    admin.save_node(node("a", "v2")).await.unwrap();
    admin.publish().await.unwrap();
    tokio::fs::write(admin.root.join("blobs").join(m.hash.unwrap()), b"modified")
        .await
        .unwrap();
    assert!(sync::synchronize(&sales, &address).await.is_err());
    assert_eq!(sales.required_workspace().await.unwrap().version, 1);
    assert_eq!(
        sales.snapshot().await.unwrap().nodes[0].node.replies["ko"],
        "个人保留"
    );
    assert_eq!(sales.progress.read().await.phase, "failed");
}

#[tokio::test]
async fn deleted_official_dependency_is_retained_until_personal_reset() {
    let dir = tempfile::tempdir().unwrap();
    let admin = create_admin(&dir.path().join("a")).await;
    let mut a = node("a", "原文");
    a.branches.push(Branch {
        id: "b".into(),
        label: "next".into(),
        target_id: "b".into(),
    });
    admin.save_node(a.clone()).await.unwrap();
    admin.save_node(node("b", "被引用的原文")).await.unwrap();
    admin.publish().await.unwrap();
    let address = server(admin.clone(), None).await;
    let sales = AppState::open(dir.path().join("s")).await.unwrap();
    sync::synchronize(&sales, &address).await.unwrap();
    a.replies.insert("ko".into(), "我的话术".into());
    sales.save_node(a.clone()).await.unwrap();
    a.branches.clear();
    admin.save_node(a).await.unwrap();
    local(&admin, "DELETE", "/scripts/b", Value::Null).await;
    admin.publish().await.unwrap();
    sync::synchronize(&sales, &address).await.unwrap();
    let view = sales.snapshot().await.unwrap();
    assert_eq!(
        view.nodes.iter().find(|n| n.node.id == "b").unwrap().source,
        "retained"
    );
    assert_eq!(
        local(&sales, "POST", "/scripts/a/reset", Value::Null)
            .await
            .0,
        StatusCode::OK
    );
    assert!(
        !sales
            .snapshot()
            .await
            .unwrap()
            .nodes
            .iter()
            .any(|n| n.node.id == "b")
    );
}

#[tokio::test]
async fn backup_restore_checks_files_and_advances_release_version() {
    let dir = tempfile::tempdir().unwrap();
    let admin = create_admin(&dir.path().join("a")).await;
    let m = binary_material(&admin, "data.txt", b"backup content").await;
    admin.save_node(node("a", "备份版本")).await.unwrap();
    admin.publish().await.unwrap();
    let file = backup::backup(
        &admin,
        Some(dir.path().join("backups").to_string_lossy().into()),
    )
    .await
    .unwrap();
    admin.save_node(node("a", "后续内容")).await.unwrap();
    admin.publish().await.unwrap();
    backup::restore(&admin, &file.path).await.unwrap();
    assert_eq!(admin.required_workspace().await.unwrap().version, 3);
    assert_eq!(
        admin.snapshot().await.unwrap().nodes[0].node.replies["ko"],
        "备份版本"
    );
    let other = create_admin(&dir.path().join("other")).await;
    assert!(backup::restore(&other, &file.path).await.is_err());
    tokio::fs::write(
        std::path::Path::new(&file.path)
            .join("blobs")
            .join(m.hash.unwrap()),
        b"corrupted",
    )
    .await
    .unwrap();
    assert!(backup::restore(&admin, &file.path).await.is_err());
    assert_eq!(admin.required_workspace().await.unwrap().version, 3);
}

#[tokio::test]
async fn offline_package_import_preserves_personal_branch_and_group_identity() {
    let dir = tempfile::tempdir().unwrap();
    let admin = create_admin(&dir.path().join("a")).await;
    admin.save_node(node("a", "原文")).await.unwrap();
    admin.publish().await.unwrap();
    let package = backup::export_package(&admin, None).await.unwrap();
    let sales = AppState::open(dir.path().join("s")).await.unwrap();
    backup::import_package(&sales, &package.path).await.unwrap();
    sales.save_node(node("a", "个人内容")).await.unwrap();
    admin.save_node(node("a", "管理员更新")).await.unwrap();
    admin.publish().await.unwrap();
    let package = backup::export_package(&admin, None).await.unwrap();
    backup::import_package(&sales, &package.path).await.unwrap();
    assert_eq!(sales.required_workspace().await.unwrap().version, 2);
    assert_eq!(
        sales.snapshot().await.unwrap().nodes[0].node.replies["ko"],
        "个人内容"
    );
}

#[tokio::test]
async fn local_mutations_require_origin_and_local_header() {
    let dir = tempfile::tempdir().unwrap();
    let state = AppState::open(dir.path()).await.unwrap();
    let request = Request::builder()
        .method("POST")
        .uri("/api/v1/group")
        .header("content-type", "application/json")
        .header("origin", "https://unrelated.example")
        .header("x-knowtosay-local", "1")
        .body(Body::from(r#"{"name":"bad"}"#))
        .unwrap();
    let response = http::local_router(state.clone())
        .oneshot(request)
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert!(state.workspace().await.unwrap().is_none());
    assert!(sync::normalize_address("https://example.com").is_err());
    assert!(sync::normalize_address("http://8.8.8.8:80").is_err());
    assert!(sync::normalize_address("http://127.0.0.1:51876/another").is_err());
}

#[tokio::test]
async fn excel_template_import_is_idempotent_and_searches_beyond_preview() {
    let dir = tempfile::tempdir().unwrap();
    let state = create_admin(dir.path()).await;
    let template =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../web/static/script-template.xlsx");
    let material = files::import_paths(&state, vec![template.to_string_lossy().into()])
        .await
        .unwrap()
        .remove(0);
    let payload = json!({"materialId":material.id,"mapping":{},"preview":false});
    for _ in 0..2 {
        let (status, nodes) = local(&state, "POST", "/scripts/import", payload.clone()).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(nodes.as_array().unwrap().len(), 4);
    }
    assert_eq!(state.snapshot().await.unwrap().nodes.len(), 4);
    let mut long = material.clone();
    long.id = "long-sheet".into();
    long.name = "大表格".into();
    long.rows = (1..=1200)
        .map(|i| SheetRow {
            sheet: "报价表".into(),
            row: i,
            cells: vec![if i == 1177 {
                "产品-KX-987-搜索命中".into()
            } else {
                format!("普通数据{i}")
            }],
        })
        .collect();
    state.put("draft_materials", &long.id, &long).await.unwrap();
    let snapshot = state.snapshot().await.unwrap();
    assert_eq!(
        snapshot
            .materials
            .iter()
            .find(|m| m.id == long.id)
            .unwrap()
            .rows
            .len(),
        150
    );
    let hits = knowtosay::search::search(&state, "KX-987").await.unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].rows[0].row, 1177);
    let page = knowtosay::search::rows(&state, &long.id, "KX-987", 0)
        .await
        .unwrap();
    assert_eq!(page.rows[0].sheet, "报价表");
    assert_eq!(page.total, 1);
    let page = knowtosay::search::rows(&state, &long.id, "", 150)
        .await
        .unwrap();
    assert_eq!(page.rows[0].row, 151);
    assert_eq!(page.total, 1200);
    state.publish().await.unwrap();
    local(
        &state,
        "DELETE",
        &format!("/materials/{}", long.id),
        Value::Null,
    )
    .await;
    assert!(
        knowtosay::search::search(&state, "KX-987")
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn personal_backup_restores_only_personal_content() {
    let dir = tempfile::tempdir().unwrap();
    let admin = create_admin(&dir.path().join("a")).await;
    admin.save_node(node("a", "原官方内容")).await.unwrap();
    admin.publish().await.unwrap();
    let address = server(admin.clone(), None).await;
    let sales = AppState::open(dir.path().join("s")).await.unwrap();
    sync::synchronize(&sales, &address).await.unwrap();
    sales.save_node(node("a", "个人备份内容")).await.unwrap();
    let backup = knowtosay::personal::export(&sales, None).await.unwrap();
    sales.save_node(node("a", "随后个人改动")).await.unwrap();
    admin.save_node(node("a", "最新官方内容")).await.unwrap();
    admin.publish().await.unwrap();
    sync::synchronize(&sales, &address).await.unwrap();
    knowtosay::personal::import(&sales, &backup.path)
        .await
        .unwrap();
    let snapshot = sales.snapshot().await.unwrap();
    assert_eq!(snapshot.nodes[0].node.replies["ko"], "个人备份内容");
    assert_eq!(
        snapshot.nodes[0].official.as_ref().unwrap().replies["ko"],
        "最新官方内容"
    );
    assert_eq!(snapshot.workspace.unwrap().version, 2);
    assert!(
        knowtosay::personal::import(&admin, &backup.path)
            .await
            .is_err()
    );
}
