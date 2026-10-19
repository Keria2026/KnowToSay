#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use knowtosay::{db::AppState, error::Error, models::ApiError};
use tauri::Manager;
use tower::ServiceExt;
mod desktop_support;

fn api_error(error: Error) -> ApiError {
    ApiError {
        code: error.1,
        message: error.2,
        request_id: uuid::Uuid::new_v4().to_string(),
        details: None,
    }
}

#[tauri::command]
async fn api_request(
    state: tauri::State<'_, AppState>,
    method: String,
    path: String,
    payload: Option<serde_json::Value>,
) -> Result<serde_json::Value, ApiError> {
    if !path.starts_with("/api/v1/") {
        return Err(api_error(Error::invalid("不支持的本地操作")));
    }
    let method = axum::http::Method::from_bytes(method.as_bytes())
        .map_err(|_| api_error(Error::invalid("请求方法无效")))?;
    let request = axum::http::Request::builder()
        .method(method)
        .uri(path)
        .header("content-type", "application/json")
        .header("x-knowtosay-local", "1")
        .body(axum::body::Body::from(
            payload.map(|p| p.to_string()).unwrap_or_default(),
        ))
        .map_err(|e| api_error(Error::internal(e)))?;
    let response = knowtosay::http::local_router(state.inner().clone())
        .oneshot(request)
        .await
        .map_err(|e| api_error(Error::internal(e)))?;
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), 64 * 1024 * 1024)
        .await
        .map_err(|e| api_error(Error::internal(e)))?;
    if !status.is_success() {
        return Err(serde_json::from_slice(&bytes)
            .unwrap_or_else(|_| api_error(Error::invalid("本地操作失败"))));
    }
    serde_json::from_slice(&bytes).map_err(|e| api_error(Error::internal(e)))
}

#[tauri::command]
async fn import_files(
    state: tauri::State<'_, AppState>,
    paths: Vec<String>,
) -> Result<serde_json::Value, ApiError> {
    let materials = knowtosay::files::import_paths(&state, paths)
        .await
        .map_err(api_error)?;
    serde_json::to_value(materials).map_err(|e| api_error(Error::internal(e)))
}

#[tauri::command]
async fn material_path(state: tauri::State<'_, AppState>, id: String) -> Result<String, ApiError> {
    let snapshot = state.snapshot().await.map_err(api_error)?;
    let m = snapshot
        .materials
        .into_iter()
        .find(|m| m.id == id)
        .ok_or_else(|| api_error(Error::missing("素材已移除")))?;
    let hash = m
        .hash
        .ok_or_else(|| api_error(Error::invalid("文字素材没有文件")))?;
    if !knowtosay::db::valid_hash(&hash) {
        return Err(api_error(Error::invalid("素材引用无效")));
    }
    Ok(state.root.join("blobs").join(hash).to_string_lossy().into())
}

#[tauri::command]
async fn export_material(
    state: tauri::State<'_, AppState>,
    id: String,
    destination: String,
) -> Result<(), ApiError> {
    let snapshot = state.snapshot().await.map_err(api_error)?;
    let m = snapshot
        .materials
        .into_iter()
        .find(|m| m.id == id)
        .ok_or_else(|| api_error(Error::missing("素材已移除")))?;
    let hash = m
        .hash
        .ok_or_else(|| api_error(Error::invalid("文字素材请使用复制")))?;
    if !knowtosay::db::valid_hash(&hash) {
        return Err(api_error(Error::invalid("素材引用无效")));
    }
    if !std::path::Path::new(&destination).is_absolute() {
        return Err(api_error(Error::invalid("请选择导出位置")));
    }
    tokio::fs::copy(state.root.join("blobs").join(hash), destination)
        .await
        .map_err(|e| api_error(Error::from(e)))?;
    Ok(())
}

#[tauri::command]
async fn reveal_material(
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
    id: String,
) -> Result<(), ApiError> {
    use tauri_plugin_opener::OpenerExt;
    let snapshot = state.snapshot().await.map_err(api_error)?;
    let m = snapshot
        .materials
        .into_iter()
        .find(|m| m.id == id)
        .ok_or_else(|| api_error(Error::missing("素材已移除")))?;
    let hash = m
        .hash
        .ok_or_else(|| api_error(Error::invalid("文字素材请直接复制")))?;
    if !knowtosay::db::valid_hash(&hash) {
        return Err(api_error(Error::invalid("素材引用无效")));
    }
    app.opener()
        .reveal_item_in_dir(state.root.join("blobs").join(hash))
        .map_err(|e| api_error(Error::internal(e)))?;
    Ok(())
}

#[tauri::command]
async fn window_mode(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    compact: bool,
) -> Result<(), String> {
    let control = app.state::<desktop_support::DesktopController>();
    let mut saved = control.main_bounds.lock().map_err(|e| e.to_string())?;
    if compact {
        *saved = Some(desktop_support::MainWindowBounds {
            size: window.inner_size().map_err(|e| e.to_string())?,
            position: window.outer_position().map_err(|e| e.to_string())?,
            maximized: window.is_maximized().map_err(|e| e.to_string())?,
        });
        window.unmaximize().map_err(|e| e.to_string())?;
        window
            .set_size(desktop_support::fitted_size(&window, (420.0, 740.0)))
            .map_err(|e| e.to_string())?;
    } else if let Some(bounds) = saved.take() {
        window.set_size(bounds.size).map_err(|e| e.to_string())?;
        window
            .set_position(bounds.position)
            .map_err(|e| e.to_string())?;
        if bounds.maximized {
            window.maximize().map_err(|e| e.to_string())?;
        }
    }
    window
        .set_always_on_top(compact)
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            desktop_support::show_main(app)
        }))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _, event| {
                    if event.state() == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                        desktop_support::toggle_search(app)
                    }
                })
                .build(),
        )
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let root = app.path().app_data_dir()?;
            let state = tauri::async_runtime::block_on(AppState::open(root))
                .map_err(|e| std::io::Error::other(e.2))?;
            tauri::async_runtime::spawn({
                let s = state.clone();
                async move {
                    knowtosay::sync::background(s);
                }
            });
            app.manage(state);
            desktop_support::setup(app)
        })
        .on_window_event(|window, event| match event {
            tauri::WindowEvent::CloseRequested { api, .. } => {
                api.prevent_close();
                let _ = window.hide();
            }
            tauri::WindowEvent::Resized(_) | tauri::WindowEvent::Focused(false)
                if window.label() == "main" && window.is_minimized().unwrap_or(false) =>
            {
                let _ = window.hide();
            }
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            api_request,
            import_files,
            material_path,
            export_material,
            reveal_material,
            window_mode,
            desktop_support::open_search,
            desktop_support::open_quick_replies,
            desktop_support::close_search,
            desktop_support::resize_search,
            desktop_support::open_script,
            desktop_support::desktop_status,
            desktop_support::save_desktop_preferences
        ])
        .run(tauri::generate_context!())
        .unwrap_or_else(|e| {
            eprintln!("KnowToSay 启动失败：{e}");
            std::process::exit(1)
        });
}
