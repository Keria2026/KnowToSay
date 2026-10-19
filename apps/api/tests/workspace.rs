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
        kind: ScriptKind::Flow,
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
    assert_eq!(
        state.list::<Material>("draft_materials").await.unwrap(),
        vec![material.clone()]
    );
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
