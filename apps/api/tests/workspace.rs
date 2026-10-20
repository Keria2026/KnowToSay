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
    s.save_flow(Flow {
        id: "test-flow".into(),
        name: "测试流程".into(),
    })
    .await
    .unwrap();
    s
}
fn node(id: &str, text: &str) -> ScriptNode {
    ScriptNode {
        id: id.into(),
        title: format!("话术{id}"),
        question: "测试问题".into(),
        category: "测试".into(),
        kind: ScriptKind::Flow,
        flow_id: Some("test-flow".into()),
        is_entry: true,
        replies: BTreeMap::from([("ko".into(), text.into())]),
        reply_options: vec![],
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
                folder_id: None,
                image_ids: vec![],
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
    let payload =
        json!({"materialId":material.id,"mapping":{},"preview":false,"flowId":"test-flow"});
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
    let page = knowtosay::search::rows(&state, &long.id, "KX-987", 0, None)
        .await
        .unwrap();
    assert_eq!(page.rows[0].sheet, "报价表");
    assert_eq!(page.total, 1);
    let page = knowtosay::search::rows(&state, &long.id, "", 150, None)
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

#[tokio::test]
async fn empty_material_categories_survive_publish_sync_packages_and_backup_restore() {
    let dir = tempfile::tempdir().unwrap();
    let admin = create_admin(&dir.path().join("admin")).await;
    let create = |name: &str| json!({"name":name});
    let (status, view) = local(
        &admin,
        "POST",
        "/material-categories",
        create("  产品资料  "),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(view["materialCategories"], json!(["产品资料"]));
    assert_eq!(view["materials"], json!([]));
    assert_eq!(view["draftDirty"], true);
    for name in ["产品资料", " 产品资料 ", "未分类"] {
        assert_eq!(
            local(&admin, "POST", "/material-categories", create(name))
                .await
                .0,
            StatusCode::CONFLICT
        );
    }
    for name in ["   ".to_string(), "长".repeat(81), "错误\n分类".into()] {
        assert_eq!(
            local(&admin, "POST", "/material-categories", create(&name))
                .await
                .0,
            StatusCode::BAD_REQUEST
        );
    }
    let reopened = AppState::open(&*admin.root).await.unwrap();
    assert_eq!(
        reopened.snapshot().await.unwrap().material_categories,
        vec!["产品资料"]
    );
    admin.publish().await.unwrap();
    assert!(!admin.snapshot().await.unwrap().draft_dirty);
    let address = server(admin.clone(), None).await;
    let sales = AppState::open(dir.path().join("sales")).await.unwrap();
    sync::synchronize(&sales, &address).await.unwrap();
    assert_eq!(
        sales.snapshot().await.unwrap().material_categories,
        vec!["产品资料"]
    );
    assert_eq!(
        local(
            &sales,
            "POST",
            "/material-categories",
            create("业务员不能创建")
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    admin.create_material_category("未发布分类").await.unwrap();
    sync::synchronize(&sales, &address).await.unwrap();
    assert_eq!(
        sales.snapshot().await.unwrap().material_categories,
        vec!["产品资料"]
    );
    let backup = backup::backup(&admin, None).await.unwrap();
    admin
        .create_material_category("备份之后新增")
        .await
        .unwrap();
    admin.publish().await.unwrap();
    backup::restore(&admin, &backup.path).await.unwrap();
    assert_eq!(
        admin.snapshot().await.unwrap().material_categories,
        vec!["产品资料", "未发布分类"]
    );
    assert_eq!(
        admin.latest().await.unwrap().material_categories,
        vec!["产品资料"]
    );
    sync::synchronize(&sales, &address).await.unwrap();
    assert_eq!(
        sales.snapshot().await.unwrap().material_categories,
        vec!["产品资料"]
    );
    let package = backup::export_package(&admin, None).await.unwrap();
    let offline = AppState::open(dir.path().join("offline")).await.unwrap();
    backup::import_package(&offline, &package.path)
        .await
        .unwrap();
    assert_eq!(
        offline.snapshot().await.unwrap().material_categories,
        vec!["产品资料"]
    );
}

#[tokio::test]
async fn material_category_migration_preserves_existing_assignments_and_empty_categories() {
    let dir = tempfile::tempdir().unwrap();
    let migrations = dir.path().join("old-migrations");
    tokio::fs::create_dir(&migrations).await.unwrap();
    for (name, sql) in [
        (
            "0001_initial.sql",
            include_str!("../migrations/0001_initial.sql"),
        ),
        (
            "0002_material_search.sql",
            include_str!("../migrations/0002_material_search.sql"),
        ),
    ] {
        tokio::fs::write(migrations.join(name), sql).await.unwrap();
    }
    let options = sqlx::sqlite::SqliteConnectOptions::new()
        .filename(dir.path().join("workspace.sqlite"))
        .create_if_missing(true);
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .connect_with(options)
        .await
        .unwrap();
    sqlx::migrate::Migrator::new(migrations.as_path())
        .await
        .unwrap()
        .run(&pool)
        .await
        .unwrap();
    let material = Material {
        id: "legacy".into(),
        name: "原有资料".into(),
        kind: "text".into(),
        category: "原有分类".into(),
        folder_id: None,
        image_ids: vec![],
        tags: vec![],
        notes: String::new(),
        text: "原有内容".into(),
        mime: "text/plain".into(),
        hash: None,
        size: 0,
        rows: vec![],
    };
    let encoded = serde_json::to_string(&material).unwrap();
    for table in ["draft_materials", "official_materials"] {
        sqlx::query(&format!("INSERT INTO {table}(id,json) VALUES(?,?)"))
            .bind(&material.id)
            .bind(&encoded)
            .execute(&pool)
            .await
            .unwrap();
    }
    pool.close().await;
    let state = AppState::open(dir.path()).await.unwrap();
    for table in ["draft_material_categories", "official_material_categories"] {
        assert_eq!(state.list::<String>(table).await.unwrap(), vec!["原有分类"]);
    }
    let migrated = state.list::<Material>("draft_materials").await.unwrap();
    let folders = state.list::<Folder>("draft_folders").await.unwrap();
    assert_eq!(folders.len(), 1);
    assert_eq!(folders[0].name, "原有分类");
    let mut expected = material.clone();
    expected.folder_id = Some(folders[0].id.clone());
    assert_eq!(migrated, vec![expected]);
    let changed = Material {
        category: "新增归类".into(),
        ..material.clone()
    };
    for _ in 0..2 {
        state
            .put("draft_materials", &changed.id, &changed)
            .await
            .unwrap();
    }
    sqlx::query("DELETE FROM draft_materials")
        .execute(&state.pool)
        .await
        .unwrap();
    assert_eq!(
        state
            .list::<String>("draft_material_categories")
            .await
            .unwrap(),
        vec!["原有分类", "新增归类"]
    );
    assert_eq!(
        state
            .list::<String>("official_material_categories")
            .await
            .unwrap(),
        vec!["原有分类"]
    );
}

#[tokio::test]
async fn flow_and_quick_scripts_stay_distinct_and_personal_quick_replies_survive_updates() {
    let dir = tempfile::tempdir().unwrap();
    let admin = create_admin(&dir.path().join("admin")).await;
    let mut terminal = node("terminal", "流程结束回复");
    terminal.is_entry = false;
    let mut root = node("root", "开始流程");
    root.branches.push(Branch {
        id: "next".into(),
        label: "继续".into(),
        target_id: terminal.id.clone(),
    });
    let mut quick = node("quick-price", "价格说明");
    quick.kind = ScriptKind::Quick;
    quick.flow_id = None;
    quick.is_entry = false;
    admin.save_node(terminal.clone()).await.unwrap();
    admin.save_node(root.clone()).await.unwrap();
    admin.save_node(quick.clone()).await.unwrap();
    let mut invalid = quick.clone();
    invalid.is_entry = true;
    assert!(admin.save_node(invalid).await.is_err());
    let mut linked = root.clone();
    linked.branches[0].target_id = quick.id.clone();
    assert!(admin.save_node(linked).await.is_err());
    let mut conversion = terminal.clone();
    conversion.kind = ScriptKind::Quick;
    assert!(admin.save_node(conversion).await.is_err());
    let mut legacy = serde_json::to_value(&terminal).unwrap();
    legacy.as_object_mut().unwrap().remove("kind");
    assert_eq!(
        serde_json::from_value::<ScriptNode>(legacy).unwrap().kind,
        ScriptKind::Flow
    );
    admin.publish().await.unwrap();
    let address = server(admin.clone(), None).await;
    let sales = AppState::open(dir.path().join("sales")).await.unwrap();
    sync::synchronize(&sales, &address).await.unwrap();
    let snapshot = sales.snapshot().await.unwrap();
    assert_eq!(
        snapshot
            .nodes
            .iter()
            .find(|r| r.node.id == "terminal")
            .unwrap()
            .node
            .kind,
        ScriptKind::Flow
    );
    assert_eq!(
        snapshot
            .nodes
            .iter()
            .find(|r| r.node.id == quick.id)
            .unwrap()
            .node
            .kind,
        ScriptKind::Quick
    );
    let mut personal = quick.clone();
    personal.replies.insert("ko".into(), "我的快捷回复".into());
    sales.save_node(personal.clone()).await.unwrap();
    quick
        .replies
        .insert("ko".into(), "管理员最新快捷回复".into());
    admin.save_node(quick.clone()).await.unwrap();
    admin.publish().await.unwrap();
    sync::synchronize(&sales, &address).await.unwrap();
    let snapshot = sales.snapshot().await.unwrap();
    let record = snapshot
        .nodes
        .iter()
        .find(|r| r.node.id == quick.id)
        .unwrap();
    assert_eq!(record.node, personal);
    assert_eq!(record.official.as_ref().unwrap(), &quick);
    let package = backup::export_package(&admin, None).await.unwrap();
    let offline = AppState::open(dir.path().join("offline")).await.unwrap();
    backup::import_package(&offline, &package.path)
        .await
        .unwrap();
    assert_eq!(
        offline
            .snapshot()
            .await
            .unwrap()
            .nodes
            .iter()
            .find(|r| r.node.id == quick.id)
            .unwrap()
            .node
            .kind,
        ScriptKind::Quick
    );
}

#[test]
fn excel_import_supports_quick_replies_and_rejects_flow_fields_on_them() {
    let row = |row, cells: &[&str]| SheetRow {
        sheet: "问答".into(),
        row,
        cells: cells.iter().map(|s| s.to_string()).collect(),
    };
    let mut rows = vec![
        row(
            1,
            &[
                "id",
                "title",
                "话术类型",
                "question",
                "reply:ko",
                "isEntry",
                "branches",
            ],
        ),
        row(
            2,
            &[
                "faq",
                "价格问题",
                "快捷话术",
                "客户说太贵了",
                "가격을 안내드립니다.",
                "false",
                "[]",
            ],
        ),
    ];
    let mapping = BTreeMap::from([("kind".into(), "话术类型".into())]);
    let nodes = files::scripts_from_rows(&rows, &mapping).unwrap();
    assert_eq!(nodes[0].kind, ScriptKind::Quick);
    assert_eq!(nodes[0].question, "客户说太贵了");
    rows[1].cells[5] = "true".into();
    assert!(files::scripts_from_rows(&rows, &mapping).is_err());
    rows[1].cells[5] = "false".into();
    rows[1].cells[6] = r#"[{"id":"next","label":"继续","targetId":"other"}]"#.into();
    assert!(files::scripts_from_rows(&rows, &mapping).is_err());
}

#[tokio::test]
async fn graph_batch_save_is_atomic_and_personal_layout_and_connections_survive_sync() {
    let dir = tempfile::tempdir().unwrap();
    let admin = create_admin(&dir.path().join("admin")).await;
    let original = node("root", "原始回复");
    admin.save_node(original.clone()).await.unwrap();
    let mut moved = original.clone();
    moved.position = Some(Position { x: 120.0, y: -45.0 });
    let mut child = node("child", "分支回复");
    child.is_entry = false;
    child.position = Some(Position { x: 420.0, y: 60.0 });
    moved.branches.push(Branch {
        id: "choice".into(),
        label: "想了解产品".into(),
        target_id: child.id.clone(),
    });
    // Force a database error after the first write to verify actual transaction rollback.
    sqlx::query("CREATE TRIGGER reject_child BEFORE INSERT ON draft_nodes WHEN NEW.id='child' BEGIN SELECT RAISE(ABORT, 'test rollback'); END").execute(&admin.pool).await.unwrap();
    assert!(
        admin
            .save_nodes(vec![moved.clone(), child.clone()])
            .await
            .is_err()
    );
    assert_eq!(
        admin.list::<ScriptNode>("draft_nodes").await.unwrap(),
        vec![original]
    );
    sqlx::query("DROP TRIGGER reject_child")
        .execute(&admin.pool)
        .await
        .unwrap();
    let (status, _) = local(&admin, "PUT", "/scripts", json!({"nodes":[moved,child]})).await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        admin
            .save_nodes(vec![moved.clone(), moved.clone()])
            .await
            .is_err()
    );
    admin.publish().await.unwrap();
    let address = server(admin.clone(), None).await;
    let sales_path = dir.path().join("sales");
    let sales = AppState::open(&sales_path).await.unwrap();
    sync::synchronize(&sales, &address).await.unwrap();
    let mut personal = moved.clone();
    personal.position = Some(Position {
        x: -300.0,
        y: 300.0,
    });
    personal.branches[0].label = "我的客户回答".into();
    let mut personal_child = node("personal-child", "我的新分支");
    personal_child.is_entry = false;
    personal_child.position = Some(Position { x: 0.0, y: 480.0 });
    personal.branches.push(Branch {
        id: "personal-choice".into(),
        label: "我的新选项".into(),
        target_id: personal_child.id.clone(),
    });
    sales
        .save_nodes(vec![personal.clone(), personal_child.clone()])
        .await
        .unwrap();
    moved.position = Some(Position { x: 900.0, y: 900.0 });
    moved.replies.insert("ko".into(), "管理员新回复".into());
    admin.save_node(moved.clone()).await.unwrap();
    admin.publish().await.unwrap();
    sync::synchronize(&sales, &address).await.unwrap();
    drop(sales);
    let reopened = AppState::open(&sales_path).await.unwrap();
    let snapshot = reopened.snapshot().await.unwrap();
    let root = snapshot.nodes.iter().find(|r| r.node.id == "root").unwrap();
    assert_eq!(root.node, personal);
    assert_eq!(root.official.as_ref().unwrap(), &moved);
    assert_eq!(
        snapshot
            .nodes
            .iter()
            .find(|r| r.node.id == personal_child.id)
            .unwrap()
            .node,
        personal_child
    );
    assert_eq!(
        admin
            .list::<ScriptNode>("personal_nodes")
            .await
            .unwrap()
            .len(),
        0
    );
}

#[tokio::test]
async fn named_flows_preserve_empty_pages_sync_packages_and_backups() {
    let dir = tempfile::tempdir().unwrap();
    let admin = create_admin(&dir.path().join("admin")).await;
    let empty = Flow {
        id: "empty".into(),
        name: "售后咨询".into(),
    };
    assert_eq!(
        local(&admin, "PUT", "/flows/empty", json!(empty)).await.0,
        StatusCode::OK
    );
    assert_eq!(
        local(
            &admin,
            "PUT",
            "/flows/duplicate",
            json!({"id":"duplicate","name":" 售后咨询 "})
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    let release = admin.publish().await.unwrap();
    assert_eq!(release.flows.len(), 2);
    let sales = AppState::open(dir.path().join("sales")).await.unwrap();
    sales.apply(&release, "").await.unwrap();
    assert!(sales.snapshot().await.unwrap().flows.contains(&empty));
    let package = backup::export_package(&admin, None).await.unwrap();
    let offline = AppState::open(dir.path().join("offline")).await.unwrap();
    backup::import_package(&offline, &package.path)
        .await
        .unwrap();
    assert!(offline.snapshot().await.unwrap().flows.contains(&empty));
    let saved = backup::backup(&admin, None).await.unwrap();
    let restored = AppState::open(dir.path().join("restored")).await.unwrap();
    backup::restore(&restored, &saved.path).await.unwrap();
    assert!(restored.snapshot().await.unwrap().flows.contains(&empty));
    assert!(restored.latest().await.unwrap().flows.contains(&empty));
    let reopened = AppState::open(&*restored.root).await.unwrap();
    assert!(reopened.snapshot().await.unwrap().flows.contains(&empty));
}

#[tokio::test]
async fn flow_boundaries_and_personal_flow_names_survive_official_changes() {
    let dir = tempfile::tempdir().unwrap();
    let admin = create_admin(&dir.path().join("a")).await;
    admin
        .save_flow(Flow {
            id: "second".into(),
            name: "第二流程".into(),
        })
        .await
        .unwrap();
    let first = node("first", "主库回复");
    let mut second = node("second-step", "第二页回复");
    second.flow_id = Some("second".into());
    admin
        .save_nodes(vec![first.clone(), second.clone()])
        .await
        .unwrap();
    let mut invalid = first.clone();
    invalid.branches.push(Branch {
        id: "cross".into(),
        label: "跨页连接".into(),
        target_id: second.id.clone(),
    });
    assert!(admin.save_node(invalid).await.is_err());
    let release = admin.publish().await.unwrap();
    let sales = AppState::open(dir.path().join("s")).await.unwrap();
    sales.apply(&release, "").await.unwrap();
    let private_name = Flow {
        id: "test-flow".into(),
        name: "我的咨询流程".into(),
    };
    sales.save_flow(private_name.clone()).await.unwrap();
    let mut personal = first.clone();
    personal.replies.insert("ko".into(), "个人回复".into());
    sales.save_node(personal.clone()).await.unwrap();
    let own_empty = Flow {
        id: "personal-empty".into(),
        name: "个人空流程".into(),
    };
    sales.save_flow(own_empty.clone()).await.unwrap();
    let personal_backup = knowtosay::personal::export(&sales, None).await.unwrap();
    admin
        .save_flow(Flow {
            id: "test-flow".into(),
            name: "官方新流程名".into(),
        })
        .await
        .unwrap();
    admin.save_node(node("first", "官方新回复")).await.unwrap();
    let updated = admin.publish().await.unwrap();
    sales.apply(&updated, "").await.unwrap();
    let snapshot = sales.snapshot().await.unwrap();
    assert!(snapshot.flows.contains(&private_name));
    assert_eq!(
        snapshot
            .nodes
            .iter()
            .find(|r| r.node.id == "first")
            .unwrap()
            .node,
        personal
    );
    let new_sales = AppState::open(dir.path().join("new-s")).await.unwrap();
    new_sales.apply(&updated, "").await.unwrap();
    knowtosay::personal::import(&new_sales, &personal_backup.path)
        .await
        .unwrap();
    let restored = new_sales.snapshot().await.unwrap();
    assert!(restored.flows.contains(&private_name));
    assert!(restored.flows.contains(&own_empty));
    // An official flow that was removed remains named when personal nodes depend on it.
    sqlx::query("DELETE FROM draft_nodes WHERE id='first'")
        .execute(&admin.pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM draft_flows WHERE id='test-flow'")
        .execute(&admin.pool)
        .await
        .unwrap();
    sales
        .apply(&admin.publish().await.unwrap(), "")
        .await
        .unwrap();
    assert!(
        sales
            .snapshot()
            .await
            .unwrap()
            .flows
            .contains(&private_name)
    );
    assert!(
        sales
            .snapshot()
            .await
            .unwrap()
            .nodes
            .iter()
            .any(|r| r.node == personal)
    );
}

#[tokio::test]
async fn single_canvas_migration_keeps_ids_edges_positions_and_personal_content() {
    let dir = tempfile::tempdir().unwrap();
    let migrations = dir.path().join("old-migrations");
    tokio::fs::create_dir(&migrations).await.unwrap();
    for (name, sql) in [
        (
            "0001_initial.sql",
            include_str!("../migrations/0001_initial.sql"),
        ),
        (
            "0002_material_search.sql",
            include_str!("../migrations/0002_material_search.sql"),
        ),
        (
            "0003_material_categories.sql",
            include_str!("../migrations/0003_material_categories.sql"),
        ),
    ] {
        tokio::fs::write(migrations.join(name), sql).await.unwrap();
    }
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .connect_with(
            sqlx::sqlite::SqliteConnectOptions::new()
                .filename(dir.path().join("workspace.sqlite"))
                .create_if_missing(true),
        )
        .await
        .unwrap();
    sqlx::migrate::Migrator::new(migrations.as_path())
        .await
        .unwrap()
        .run(&pool)
        .await
        .unwrap();
    let workspace = Workspace {
        group_id: uuid::Uuid::new_v4().to_string(),
        name: "旧工作组".into(),
        role: Role::Admin,
        version: 1,
        admin_address: None,
    };
    sqlx::query("INSERT INTO workspace(id,json) VALUES(1,?)")
        .bind(serde_json::to_string(&workspace).unwrap())
        .execute(&pool)
        .await
        .unwrap();
    let mut original = node("old", "原有回复");
    original.position = Some(Position {
        x: -123.0,
        y: 456.0,
    });
    original.branches.push(Branch {
        id: "loop".into(),
        label: "重新确认".into(),
        target_id: "old".into(),
    });
    let mut old = json!(original);
    old.as_object_mut().unwrap().remove("flowId");
    old.as_object_mut().unwrap().remove("kind");
    for table in [
        "draft_nodes",
        "official_nodes",
        "personal_nodes",
        "personal_dependencies",
    ] {
        sqlx::query(&format!("INSERT INTO {table}(id,json) VALUES('old',?)"))
            .bind(old.to_string())
            .execute(&pool)
            .await
            .unwrap();
    }
    let release = json!({"formatVersion":1,"groupId":workspace.group_id,"groupName":workspace.name,"version":1,"nodes":[old],"materials":[],"materialCategories":[]});
    sqlx::query("INSERT INTO releases(version,json) VALUES(1,?)")
        .bind(release.to_string())
        .execute(&pool)
        .await
        .unwrap();
    pool.close().await;
    let state = AppState::open(dir.path()).await.unwrap();
    original.flow_id = Some("legacy-flow".into());
    for table in [
        "draft_nodes",
        "official_nodes",
        "personal_nodes",
        "personal_dependencies",
    ] {
        assert_eq!(
            state.list::<ScriptNode>(table).await.unwrap(),
            vec![original.clone()]
        );
    }
    for table in [
        "draft_flows",
        "official_flows",
        "personal_flows",
        "personal_flow_dependencies",
    ] {
        assert_eq!(state.list::<Flow>(table).await.unwrap()[0].name, "原有流程");
    }
    let latest = state.latest().await.unwrap();
    assert_eq!(latest.format_version, 2);
    assert_eq!(latest.nodes, vec![original]);
    assert_eq!(latest.flows[0].id, "legacy-flow");
}

#[tokio::test]
async fn quick_categories_preserve_empty_groups_and_personal_content_across_transfers() {
    let dir = tempfile::tempdir().unwrap();
    let admin = create_admin(&dir.path().join("admin")).await;
    admin
        .save_node(node("flow", "流程分类不应成为快捷分类"))
        .await
        .unwrap();
    admin.publish().await.unwrap();
    assert!(admin.snapshot().await.unwrap().quick_categories.is_empty());
    assert_eq!(
        local(
            &admin,
            "POST",
            "/quick-categories",
            json!({"name":" 售后答疑 "})
        )
        .await
        .0,
        StatusCode::OK
    );
    assert!(admin.snapshot().await.unwrap().draft_dirty);
    for name in ["售后答疑", "未分类"] {
        assert_eq!(
            local(&admin, "POST", "/quick-categories", json!({"name":name}))
                .await
                .0,
            StatusCode::CONFLICT
        );
    }
    for name in [" ", "非法\n分类", &"长".repeat(81)] {
        assert_eq!(
            local(&admin, "POST", "/quick-categories", json!({"name":name}))
                .await
                .0,
            StatusCode::BAD_REQUEST
        );
    }
    let mut quick = node("quick", "한국어 답변");
    quick.kind = ScriptKind::Quick;
    quick.flow_id = None;
    quick.is_entry = false;
    quick.category = "产品咨询".into();
    admin.save_node(quick.clone()).await.unwrap();
    // Updating an existing node into an already registered category must not violate triggers.
    quick.question = "更新问题".into();
    admin.save_node(quick.clone()).await.unwrap();
    let release = admin.publish().await.unwrap();
    assert_eq!(release.quick_categories, vec!["产品咨询", "售后答疑"]);
    assert!(!admin.snapshot().await.unwrap().draft_dirty);
    let address = server(admin.clone(), None).await;
    let sales = AppState::open(dir.path().join("sales")).await.unwrap();
    sync::synchronize(&sales, &address).await.unwrap();
    assert_eq!(
        sales.snapshot().await.unwrap().quick_categories,
        release.quick_categories
    );
    let package = backup::export_package(&admin, None).await.unwrap();
    let offline = AppState::open(dir.path().join("offline")).await.unwrap();
    backup::import_package(&offline, &package.path)
        .await
        .unwrap();
    assert_eq!(
        offline.snapshot().await.unwrap().quick_categories,
        release.quick_categories
    );
    let saved = backup::backup(&admin, None).await.unwrap();
    admin.create_quick_category("备份之后新增").await.unwrap();
    backup::restore(&admin, &saved.path).await.unwrap();
    let reopened = AppState::open(&*admin.root).await.unwrap();
    assert_eq!(
        reopened.snapshot().await.unwrap().quick_categories,
        release.quick_categories
    );
    assert_eq!(
        reopened.latest().await.unwrap().quick_categories,
        release.quick_categories
    );
    sales.create_quick_category("个人空分类").await.unwrap();
    quick.category = "我的客户".into();
    quick.replies.insert("ko".into(), "내 답변".into());
    sales.save_node(quick.clone()).await.unwrap();
    let personal = knowtosay::personal::export(&sales, None).await.unwrap();
    admin.create_quick_category("管理员新增").await.unwrap();
    admin.publish().await.unwrap();
    sync::synchronize(&sales, &address).await.unwrap();
    let snapshot = sales.snapshot().await.unwrap();
    assert!(snapshot.quick_categories.contains(&"个人空分类".into()));
    assert!(snapshot.quick_categories.contains(&"我的客户".into()));
    assert!(snapshot.quick_categories.contains(&"管理员新增".into()));
    assert_eq!(
        snapshot
            .nodes
            .iter()
            .find(|r| r.node.id == quick.id)
            .unwrap()
            .node,
        quick
    );
    knowtosay::personal::import(&offline, &personal.path)
        .await
        .unwrap();
    assert!(
        offline
            .snapshot()
            .await
            .unwrap()
            .quick_categories
            .contains(&"个人空分类".into())
    );
    assert_eq!(
        offline
            .snapshot()
            .await
            .unwrap()
            .nodes
            .iter()
            .find(|r| r.node.id == quick.id)
            .unwrap()
            .node,
        quick
    );
    // Older manifests derive occupied categories from their quick nodes.
    let mut legacy = serde_json::to_value(&release).unwrap();
    legacy.as_object_mut().unwrap().remove("quickCategories");
    let old: Manifest = serde_json::from_value(legacy).unwrap();
    let previous = AppState::open(dir.path().join("previous")).await.unwrap();
    previous.apply(&old, "").await.unwrap();
    assert_eq!(
        previous.snapshot().await.unwrap().quick_categories,
        vec!["产品咨询"]
    );
}

#[tokio::test]
async fn quick_category_migration_preserves_existing_node_names_in_all_scopes() {
    let dir = tempfile::tempdir().unwrap();
    let migrations = dir.path().join("old-migrations");
    tokio::fs::create_dir(&migrations).await.unwrap();
    for (name, sql) in [
        (
            "0001_initial.sql",
            include_str!("../migrations/0001_initial.sql"),
        ),
        (
            "0002_material_search.sql",
            include_str!("../migrations/0002_material_search.sql"),
        ),
        (
            "0003_material_categories.sql",
            include_str!("../migrations/0003_material_categories.sql"),
        ),
        (
            "0004_named_flows.sql",
            include_str!("../migrations/0004_named_flows.sql"),
        ),
    ] {
        tokio::fs::write(migrations.join(name), sql).await.unwrap();
    }
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .connect_with(
            sqlx::sqlite::SqliteConnectOptions::new()
                .filename(dir.path().join("workspace.sqlite"))
                .create_if_missing(true),
        )
        .await
        .unwrap();
    sqlx::migrate::Migrator::new(migrations.as_path())
        .await
        .unwrap()
        .run(&pool)
        .await
        .unwrap();
    let mut quick = node("old-quick", "이전 답변");
    quick.kind = ScriptKind::Quick;
    quick.flow_id = None;
    quick.is_entry = false;
    quick.category = " 原有分类 ".into();
    for table in [
        "draft_nodes",
        "official_nodes",
        "personal_nodes",
        "personal_dependencies",
    ] {
        sqlx::query(&format!("INSERT INTO {table}(id,json) VALUES(?,?)"))
            .bind(&quick.id)
            .bind(serde_json::to_string(&quick).unwrap())
            .execute(&pool)
            .await
            .unwrap();
    }
    pool.close().await;
    let state = AppState::open(dir.path()).await.unwrap();
    for table in [
        "draft_quick_categories",
        "official_quick_categories",
        "personal_quick_categories",
    ] {
        assert_eq!(state.list::<String>(table).await.unwrap(), vec!["原有分类"]);
    }
    assert_eq!(
        state.list::<ScriptNode>("draft_nodes").await.unwrap()[0],
        quick
    );
}

#[tokio::test]
async fn quick_category_management_is_atomic_and_preserves_personal_overrides() {
    let dir = tempfile::tempdir().unwrap();
    let admin = create_admin(&dir.path().join("admin")).await;
    let mut quick = node("quick", "원래 답변");
    quick.kind = ScriptKind::Quick;
    quick.is_entry = false;
    quick.flow_id = None;
    quick.category = "原分类".into();
    admin.save_node(quick.clone()).await.unwrap();
    admin.create_quick_category("空分类").await.unwrap();
    admin.publish().await.unwrap();
    let sales = AppState::open(dir.path().join("sales")).await.unwrap();
    sales
        .apply(&admin.latest().await.unwrap(), "")
        .await
        .unwrap();
    let mut personal = quick.clone();
    personal.replies.insert("ko".into(), "내 답변".into());
    sales.save_node(personal.clone()).await.unwrap();
    assert!(
        sales
            .snapshot()
            .await
            .unwrap()
            .manageable_quick_categories
            .is_empty()
    );
    assert!(sales.update_quick_category("原分类", None).await.is_err());
    admin
        .update_quick_category("原分类", Some("新分类"))
        .await
        .unwrap();
    assert_eq!(
        admin.snapshot().await.unwrap().nodes[0].node.category,
        "新分类"
    );
    assert_eq!(admin.latest().await.unwrap().nodes[0].category, "原分类");
    assert!(
        admin
            .update_quick_category("空分类", Some("新分类"))
            .await
            .is_err()
    );
    assert!(admin.update_quick_category("未分类", None).await.is_err());
    admin
        .update_quick_category("空分类", Some("空分类改名"))
        .await
        .unwrap();
    admin
        .update_quick_category("空分类改名", None)
        .await
        .unwrap();
    let before = serde_json::to_value(admin.snapshot().await.unwrap()).unwrap();
    sqlx::query("CREATE TRIGGER reject_category_update BEFORE UPDATE ON draft_nodes BEGIN SELECT RAISE(ABORT,'test rollback'); END").execute(&admin.pool).await.unwrap();
    assert!(
        admin
            .update_quick_category("新分类", Some("应当回滚"))
            .await
            .is_err()
    );
    assert_eq!(
        serde_json::to_value(admin.snapshot().await.unwrap()).unwrap(),
        before
    );
    sqlx::query("DROP TRIGGER reject_category_update")
        .execute(&admin.pool)
        .await
        .unwrap();
    admin.update_quick_category("新分类", None).await.unwrap();
    let release = admin.publish().await.unwrap();
    assert!(release.quick_categories.is_empty());
    assert_eq!(release.nodes[0].category, "未分类");
    sales.apply(&release, "").await.unwrap();
    let snapshot = sales.snapshot().await.unwrap();
    assert_eq!(snapshot.nodes[0].node, personal);
    assert!(snapshot.quick_categories.contains(&"原分类".into()));
    sales.create_quick_category("个人分类").await.unwrap();
    personal.category = "个人分类".into();
    sales.save_node(personal.clone()).await.unwrap();
    sales
        .update_quick_category("个人分类", Some("我的分类"))
        .await
        .unwrap();
    assert!(
        sales
            .snapshot()
            .await
            .unwrap()
            .manageable_quick_categories
            .contains(&"我的分类".into())
    );
    sales.update_quick_category("我的分类", None).await.unwrap();
    let snapshot = sales.snapshot().await.unwrap();
    assert_eq!(snapshot.nodes[0].node.replies["ko"], "내 답변");
    assert_eq!(snapshot.nodes[0].node.category, "未分类");
    assert_eq!(
        admin.snapshot().await.unwrap().nodes[0].node.replies["ko"],
        "원래 답변"
    );
}

#[tokio::test]
async fn foreign_backup_import_merges_drafts_remaps_references_and_keeps_identity() {
    let dir = tempfile::tempdir().unwrap();
    let source = create_admin(&dir.path().join("source")).await;
    let material = binary_material(&source, "manual.txt", b"shared content").await;
    source.create_quick_category("空快捷分类").await.unwrap();
    source.create_material_category("空素材分类").await.unwrap();
    let mut root = node("same-id", "백업 답변");
    root.position = Some(Position { x: 55.0, y: 70.0 });
    root.material_ids = vec![material.id.clone(), "removed-material".into()];
    root.branches = vec![Branch {
        id: "branch".into(),
        label: "继续".into(),
        target_id: "child".into(),
    }];
    let mut child = node("child", "다음 답변");
    child.is_entry = false;
    source.save_nodes(vec![root.clone(), child]).await.unwrap();
    source.publish().await.unwrap();
    root.replies.insert("ko".into(), "최신 초안 답변".into());
    source.save_node(root.clone()).await.unwrap();
    let saved = backup::backup(&source, None).await.unwrap();
    let target = create_admin(&dir.path().join("target")).await;
    target
        .save_node(node("same-id", "我的原有话术"))
        .await
        .unwrap();
    binary_material(&target, "existing.txt", b"shared content").await;
    target.publish().await.unwrap();
    let identity = target.required_workspace().await.unwrap();
    let published = serde_json::to_value(target.latest().await.unwrap()).unwrap();
    assert!(backup::restore(&target, &saved.path).await.is_err());
    assert_eq!(
        local(
            &target,
            "POST",
            "/backup/import",
            json!({"directory":saved.path})
        )
        .await
        .0,
        StatusCode::OK
    );
    let snapshot = target.snapshot().await.unwrap();
    assert_eq!(
        serde_json::to_value(snapshot.workspace.as_ref().unwrap()).unwrap(),
        serde_json::to_value(&identity).unwrap()
    );
    assert_eq!(
        serde_json::to_value(target.latest().await.unwrap()).unwrap(),
        published
    );
    assert!(snapshot.draft_dirty);
    assert!(snapshot.quick_categories.contains(&"空快捷分类".into()));
    assert!(snapshot.material_categories.contains(&"空素材分类".into()));
    assert_eq!(
        snapshot
            .nodes
            .iter()
            .find(|r| r.node.id == "same-id")
            .unwrap()
            .node
            .replies["ko"],
        "我的原有话术"
    );
    let imported = &snapshot
        .nodes
        .iter()
        .find(|r| {
            r.node
                .replies
                .get("ko")
                .is_some_and(|s| s == "최신 초안 답변")
        })
        .unwrap()
        .node;
    assert_ne!(imported.id, root.id);
    assert_ne!(imported.flow_id, root.flow_id);
    assert_eq!(imported.position, root.position);
    assert_ne!(imported.branches[0].id, "branch");
    let next = &snapshot
        .nodes
        .iter()
        .find(|r| r.node.id == imported.branches[0].target_id)
        .unwrap()
        .node;
    assert_eq!(next.flow_id, imported.flow_id);
    assert_eq!(next.replies["ko"], "다음 답변");
    let linked = snapshot
        .materials
        .iter()
        .find(|m| m.id == imported.material_ids[0])
        .unwrap();
    assert_ne!(linked.id, material.id);
    assert_eq!(linked.hash, material.hash);
    assert_ne!(imported.material_ids[1], "removed-material");
    assert!(
        !snapshot
            .materials
            .iter()
            .any(|m| m.id == imported.material_ids[1])
    );
    assert_eq!(
        snapshot
            .flows
            .iter()
            .find(|f| Some(&f.id) == imported.flow_id.as_ref())
            .unwrap()
            .name,
        "测试流程（导入 2）"
    );
    let blob_count = std::fs::read_dir(target.root.join("blobs"))
        .unwrap()
        .count();
    assert_eq!(blob_count, 1);
    // A failed database write rolls back the whole import.
    let before = serde_json::to_value(target.snapshot().await.unwrap()).unwrap();
    sqlx::query("CREATE TRIGGER reject_backup_node BEFORE INSERT ON draft_nodes BEGIN SELECT RAISE(ABORT,'test rollback'); END").execute(&target.pool).await.unwrap();
    assert!(backup::import_as_draft(&target, &saved.path).await.is_err());
    assert_eq!(
        serde_json::to_value(target.snapshot().await.unwrap()).unwrap(),
        before
    );
    sqlx::query("DROP TRIGGER reject_backup_node")
        .execute(&target.pool)
        .await
        .unwrap();
    // File corruption also leaves all current records and releases intact.
    let hash = material.hash.as_ref().unwrap();
    tokio::fs::write(
        std::path::Path::new(&saved.path).join("blobs").join(hash),
        b"corrupt",
    )
    .await
    .unwrap();
    assert!(backup::import_as_draft(&target, &saved.path).await.is_err());
    assert_eq!(
        serde_json::to_value(target.snapshot().await.unwrap()).unwrap(),
        before
    );
    let sales = AppState::open(dir.path().join("sales")).await.unwrap();
    sales
        .apply(&source.latest().await.unwrap(), "")
        .await
        .unwrap();
    assert_eq!(
        local(
            &sales,
            "POST",
            "/backup/import",
            json!({"directory":saved.path})
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    let release = target.publish().await.unwrap();
    assert_eq!(release.group_id, identity.group_id);
    assert_eq!(release.version, identity.version + 1);
    assert_eq!(release.nodes.len(), 3);
}

#[tokio::test]
async fn foreign_single_canvas_backup_import_upgrades_without_adopting_identity() {
    let dir = tempfile::tempdir().unwrap();
    let source = create_admin(&dir.path().join("source")).await;
    source
        .save_node(node("legacy-root", "이전 내용"))
        .await
        .unwrap();
    source.publish().await.unwrap();
    let saved = backup::backup(&source, None).await.unwrap();
    let path = std::path::Path::new(&saved.path).join("backup.json");
    let mut data: Value = serde_json::from_slice(&tokio::fs::read(&path).await.unwrap()).unwrap();
    data["format_version"] = json!(1);
    for field in [
        "draft_flows",
        "official_flows",
        "draft_quick_categories",
        "official_quick_categories",
    ] {
        data.as_object_mut().unwrap().remove(field);
    }
    for field in ["draft_nodes", "official_nodes"] {
        for node in data[field].as_array_mut().unwrap() {
            node.as_object_mut().unwrap().remove("flowId");
        }
    }
    for release in data["releases"].as_array_mut().unwrap() {
        release["formatVersion"] = json!(1);
        release.as_object_mut().unwrap().remove("flows");
        release.as_object_mut().unwrap().remove("quickCategories");
        for node in release["nodes"].as_array_mut().unwrap() {
            node.as_object_mut().unwrap().remove("flowId");
        }
    }
    tokio::fs::write(&path, serde_json::to_vec(&data).unwrap())
        .await
        .unwrap();
    let target = create_admin(&dir.path().join("target")).await;
    let identity = serde_json::to_value(target.required_workspace().await.unwrap()).unwrap();
    backup::import_as_draft(&target, &saved.path).await.unwrap();
    let snapshot = target.snapshot().await.unwrap();
    assert_eq!(serde_json::to_value(snapshot.workspace).unwrap(), identity);
    assert_eq!(snapshot.nodes.len(), 1);
    let imported = &snapshot.nodes[0].node;
    assert_ne!(imported.id, "legacy-root");
    assert!(imported.is_entry);
    assert_eq!(imported.replies["ko"], "이전 내용");
    assert_eq!(
        snapshot
            .flows
            .iter()
            .find(|f| Some(&f.id) == imported.flow_id.as_ref())
            .unwrap()
            .name,
        "原有流程"
    );
}

#[tokio::test]
async fn excel_preview_preserves_original_columns_sheet_selection_and_pagination() {
    let dir = tempfile::tempdir().unwrap();
    let state = create_admin(dir.path()).await;
    let material = binary_material(
        &state,
        "grid.xlsx",
        include_bytes!("../../web/tests/fixtures/sales-grid.xlsx"),
    )
    .await;
    let page = knowtosay::search::rows(&state, &material.id, "", 0, Some("产品资料"))
        .await
        .unwrap();
    assert_eq!(page.sheets, vec!["产品资料", "售后"]);
    assert_eq!(page.total, 163);
    assert_eq!(page.rows.len(), 150);
    assert_eq!(page.column_count, 27);
    assert_eq!(page.rows[0].row, 3);
    assert_eq!(&page.rows[0].cells[..4], &["", "", "产品", "韩语回复"]);
    assert_eq!(page.rows[0].cells[26], "远列");
    let next = knowtosay::search::rows(&state, &material.id, "", 150, Some("产品资料"))
        .await
        .unwrap();
    assert_eq!(next.rows.len(), 13);
    assert_eq!(next.rows[0].row, 153);
    let (_, json) = local(
        &state,
        "GET",
        &format!("/materials/{}/rows?sheet=%E5%94%AE%E5%90%8E", material.id),
        Value::Null,
    )
    .await;
    assert_eq!(json["total"], 1);
    assert_eq!(json["rows"][0]["row"], 2);
    assert_eq!(json["rows"][0]["cells"][1], "售后内容");
    let matches = knowtosay::search::rows(&state, &material.id, "KOREA", 0, None)
        .await
        .unwrap();
    assert_eq!(matches.total, 1);
    assert_eq!(matches.rows[0].row, 4);
    let mut stored = material;
    stored.category = "产品".into();
    state
        .put("draft_materials", &stored.id, &stored)
        .await
        .unwrap();
    assert!(
        knowtosay::search::search_filtered(&state, "KOREA", Some("售后"))
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        knowtosay::search::search_filtered(&state, "KOREA", Some("产品"))
            .await
            .unwrap()
            .len(),
        1
    );
}

#[tokio::test]
async fn multiple_quick_replies_survive_storage_sync_backups_and_personal_updates() {
    let dir = tempfile::tempdir().unwrap();
    let admin = create_admin(&dir.path().join("admin")).await;
    let mut quick = node("multi", "기본 답변");
    quick.kind = ScriptKind::Quick;
    quick.flow_id = None;
    quick.is_entry = false;
    let mut legacy = serde_json::to_value(&quick).unwrap();
    legacy.as_object_mut().unwrap().remove("replyOptions");
    assert!(
        serde_json::from_value::<ScriptNode>(legacy)
            .unwrap()
            .reply_options
            .is_empty()
    );
    quick.reply_options = vec![ReplyOption {
        id: "option-a".into(),
        name: "追问".into(),
        replies: BTreeMap::from([
            ("zh-CN".into(), "具体情况".into()),
            ("ko".into(), "상세 상황".into()),
        ]),
    }];
    let mut invalid = quick.clone();
    invalid.reply_options.push(invalid.reply_options[0].clone());
    assert!(admin.save_node(invalid).await.is_err());
    let mut invalid = node("invalid-flow", "flow");
    invalid.reply_options = quick.reply_options.clone();
    assert!(admin.save_node(invalid).await.is_err());
    admin.save_node(quick.clone()).await.unwrap();
    let reopened = AppState::open(&*admin.root).await.unwrap();
    assert_eq!(reopened.snapshot().await.unwrap().nodes[0].node, quick);
    admin.publish().await.unwrap();
    let address = server(admin.clone(), None).await;
    let sales = AppState::open(dir.path().join("sales")).await.unwrap();
    sync::synchronize(&sales, &address).await.unwrap();
    assert_eq!(sales.snapshot().await.unwrap().nodes[0].node, quick);
    let mut personal = quick.clone();
    personal.reply_options[0]
        .replies
        .insert("ko".into(), "개인 답변".into());
    sales.save_node(personal.clone()).await.unwrap();
    let personal_backup = knowtosay::personal::export(&sales, None).await.unwrap();
    quick.reply_options[0].name = "管理员更名".into();
    admin.save_node(quick.clone()).await.unwrap();
    admin.publish().await.unwrap();
    sync::synchronize(&sales, &address).await.unwrap();
    let snapshot = sales.snapshot().await.unwrap();
    assert_eq!(snapshot.nodes[0].node, personal);
    assert_eq!(snapshot.nodes[0].official.as_ref(), Some(&quick));
    let package = backup::export_package(&admin, None).await.unwrap();
    let offline = AppState::open(dir.path().join("offline")).await.unwrap();
    backup::import_package(&offline, &package.path)
        .await
        .unwrap();
    assert_eq!(offline.snapshot().await.unwrap().nodes[0].node, quick);
    knowtosay::personal::import(&offline, &personal_backup.path)
        .await
        .unwrap();
    assert_eq!(offline.snapshot().await.unwrap().nodes[0].node, personal);
    let saved = backup::backup(&admin, None).await.unwrap();
    let other = create_admin(&dir.path().join("other")).await;
    backup::import_as_draft(&other, &saved.path).await.unwrap();
    assert_eq!(
        other.snapshot().await.unwrap().nodes[0].node.reply_options,
        quick.reply_options
    );
    assert_eq!(
        local(&admin, "DELETE", "/scripts/multi", json!(null))
            .await
            .0,
        StatusCode::OK
    );
    backup::restore(&admin, &saved.path).await.unwrap();
    assert_eq!(admin.snapshot().await.unwrap().nodes[0].node, quick);
}

#[tokio::test]
async fn case_images_and_folders_survive_moves_sync_and_foreign_backup_import() {
    let dir = tempfile::tempdir().unwrap();
    let admin = create_admin(&dir.path().join("admin")).await;
    let root = Folder {
        id: "cases".into(),
        name: "案例".into(),
        parent_id: None,
    };
    let child = Folder {
        id: "child".into(),
        name: "治疗方案".into(),
        parent_id: Some(root.id.clone()),
    };
    admin.save_folder(root.clone()).await.unwrap();
    admin.save_folder(child.clone()).await.unwrap();
    assert!(
        admin
            .save_folder(Folder {
                parent_id: Some(child.id.clone()),
                ..root.clone()
            })
            .await
            .is_err()
    );
    assert!(
        admin
            .save_folder(Folder {
                id: "duplicate".into(),
                ..child.clone()
            })
            .await
            .is_err()
    );
    let image = binary_material(
        &admin,
        "case.png",
        include_bytes!("../../web/tests/fixtures/preview.png"),
    )
    .await;
    let content = json!({"id":"case", "name":"案例组合", "kind":"case", "text":"案例文案", "notes":"详情备注", "category":"", "folderId":child.id, "imageIds":[image.id], "tags":["案例标签"], "hash":null, "size":0, "mime":"text/plain", "rows":[]});
    assert_eq!(
        local(&admin, "POST", "/materials", content.clone()).await.0,
        StatusCode::OK
    );
    let case = admin
        .snapshot()
        .await
        .unwrap()
        .materials
        .into_iter()
        .find(|m| m.id == "case")
        .unwrap();
    assert_eq!(case.size, image.size);
    let mut invalid = content;
    invalid["imageIds"] = json!([image.id, image.id]);
    assert_eq!(
        local(&admin, "POST", "/materials", invalid).await.0,
        StatusCode::BAD_REQUEST
    );
    admin
        .save_folder(Folder {
            name: "调整后的方案".into(),
            ..child.clone()
        })
        .await
        .unwrap();
    assert_eq!(
        admin
            .snapshot()
            .await
            .unwrap()
            .materials
            .into_iter()
            .find(|m| m.id == "case")
            .unwrap()
            .folder_id
            .as_deref(),
        Some("child")
    );
    assert_eq!(
        knowtosay::search::search_folder(&admin, "案例文案", None, Some("cases"))
            .await
            .unwrap()
            .len(),
        1
    );
    assert!(
        knowtosay::search::search_folder(&admin, "案例文案", None, Some("missing"))
            .await
            .unwrap()
            .is_empty()
    );
    admin.publish().await.unwrap();
    let address = server(admin.clone(), None).await;
    let sales = AppState::open(dir.path().join("sales")).await.unwrap();
    sync::synchronize(&sales, &address).await.unwrap();
    assert_eq!(sales.snapshot().await.unwrap().folders.len(), 2);
    assert!(
        sales
            .root
            .join("blobs")
            .join(image.hash.as_ref().unwrap())
            .is_file()
    );
    assert_eq!(
        sales
            .snapshot()
            .await
            .unwrap()
            .materials
            .into_iter()
            .find(|m| m.id == "case")
            .unwrap(),
        case
    );
    let package = backup::export_package(&admin, None).await.unwrap();
    let offline = AppState::open(dir.path().join("offline")).await.unwrap();
    backup::import_package(&offline, &package.path)
        .await
        .unwrap();
    assert_eq!(offline.snapshot().await.unwrap().folders.len(), 2);
    let saved = backup::backup(&admin, None).await.unwrap();
    let other = create_admin(&dir.path().join("other")).await;
    other
        .save_folder(Folder {
            id: "existing".into(),
            ..root.clone()
        })
        .await
        .unwrap();
    backup::import_as_draft(&other, &saved.path).await.unwrap();
    let imported = other.snapshot().await.unwrap();
    let imported_case = imported
        .materials
        .iter()
        .find(|m| m.kind == "case")
        .unwrap();
    assert_ne!(imported_case.image_ids[0], image.id);
    let imported_image = imported
        .materials
        .iter()
        .find(|m| m.id == imported_case.image_ids[0])
        .unwrap();
    assert_eq!(imported_image.hash, image.hash);
    let imported_child = imported
        .folders
        .iter()
        .find(|f| Some(&f.id) == imported_case.folder_id.as_ref())
        .unwrap();
    let imported_root = imported
        .folders
        .iter()
        .find(|f| Some(&f.id) == imported_child.parent_id.as_ref())
        .unwrap();
    assert_eq!(imported_root.name, "案例（导入 2）");
    other.publish().await.unwrap();
    admin.delete_folder("child").await.unwrap();
    assert_eq!(
        admin
            .snapshot()
            .await
            .unwrap()
            .materials
            .iter()
            .find(|m| m.id == "case")
            .unwrap()
            .folder_id
            .as_deref(),
        Some("cases")
    );
    backup::restore(&admin, &saved.path).await.unwrap();
    assert_eq!(admin.snapshot().await.unwrap().folders.len(), 2);
}

#[tokio::test]
async fn legacy_categories_and_release_manifests_migrate_to_stable_folders() {
    let dir = tempfile::tempdir().unwrap();
    let migration_dir = dir.path().join("previous-migrations");
    tokio::fs::create_dir_all(&migration_dir).await.unwrap();
    for entry in std::fs::read_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/migrations")).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name();
        if name.to_string_lossy().starts_with("0006") {
            continue;
        }
        tokio::fs::copy(entry.path(), migration_dir.join(name))
            .await
            .unwrap();
    }
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .connect_with(
            sqlx::sqlite::SqliteConnectOptions::new()
                .filename(dir.path().join("workspace.sqlite"))
                .create_if_missing(true),
        )
        .await
        .unwrap();
    sqlx::migrate::Migrator::new(migration_dir.as_path())
        .await
        .unwrap()
        .run(&pool)
        .await
        .unwrap();
    let group_id = uuid::Uuid::new_v4().to_string();
    let material = json!({"id":"legacy-file","name":"原有文字","kind":"text","category":"原分类","tags":[],"notes":"旧备注","text":"旧正文","mime":"text/plain","hash":null,"size":0,"rows":[]});
    for table in ["draft_materials", "official_materials"] {
        sqlx::query(&format!(
            "INSERT INTO {table}(id,json) VALUES('legacy-file',?)"
        ))
        .bind(material.to_string())
        .execute(&pool)
        .await
        .unwrap();
    }
    sqlx::query("INSERT INTO workspace(id,json) VALUES(1,?)").bind(json!({"groupId":group_id,"name":"旧工作组","role":"admin","version":1,"adminAddress":null}).to_string()).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO releases(version,json) VALUES(1,?)").bind(json!({"formatVersion":2,"groupId":group_id,"groupName":"旧工作组","version":1,"nodes":[],"flows":[],"materials":[material],"materialCategories":["原分类"],"quickCategories":[]}).to_string()).execute(&pool).await.unwrap();
    pool.close().await;
    let state = AppState::open(dir.path()).await.unwrap();
    let first = state.latest().await.unwrap();
    let second = state.latest().await.unwrap();
    assert_eq!(first.folders, second.folders);
    assert_eq!(first.folders.len(), 1);
    assert_eq!(
        first.materials[0].folder_id.as_ref(),
        Some(&first.folders[0].id)
    );
    let snapshot = state.snapshot().await.unwrap();
    assert_eq!(snapshot.folders[0].id, first.folders[0].id);
    assert_eq!(
        snapshot.materials[0].folder_id.as_ref(),
        Some(&snapshot.folders[0].id)
    );
    state.delete_folder(&snapshot.folders[0].id).await.unwrap();
    let saved = backup::backup(&state, None).await.unwrap();
    state
        .save_folder(Folder {
            id: "temporary".into(),
            name: "临时文件夹".into(),
            parent_id: None,
        })
        .await
        .unwrap();
    backup::restore(&state, &saved.path).await.unwrap();
    assert!(state.snapshot().await.unwrap().folders.is_empty());
}
