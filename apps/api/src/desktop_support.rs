use knowtosay::{
    db::AppState,
    models::{DesktopPreferences, DesktopStatus, SearchPanelMode},
};
use std::sync::{
    Mutex,
    atomic::{AtomicBool, AtomicIsize, Ordering},
};
use tauri::{Emitter, Manager, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_global_shortcut::GlobalShortcutExt;

pub struct MainWindowBounds {
    pub size: tauri::PhysicalSize<u32>,
    pub position: tauri::PhysicalPosition<i32>,
    pub maximized: bool,
}

pub struct DesktopController {
    pub preferences: Mutex<DesktopPreferences>,
    pub shortcut_registered: AtomicBool,
    pub warning: Mutex<Option<String>>,
    pub previous_window: AtomicIsize,
    pub search_mode: Mutex<SearchPanelMode>,
    pub search_expanded: AtomicBool,
    pub main_bounds: Mutex<Option<MainWindowBounds>>,
}

pub fn show_main(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

pub fn show_search(app: &tauri::AppHandle) {
    show_panel(app, SearchPanelMode::Search)
}

pub fn show_quick_replies(app: &tauri::AppHandle) {
    show_panel(app, SearchPanelMode::Quick)
}

pub fn toggle_search(app: &tauri::AppHandle) {
    let quick = app.try_state::<DesktopController>().is_some_and(|c| {
        c.search_mode
            .lock()
            .is_ok_and(|m| *m == SearchPanelMode::Quick)
    });
    if !quick
        && app
            .get_webview_window("search")
            .is_some_and(|w| w.is_visible().unwrap_or(false))
    {
        hide_search(app);
    } else {
        show_search(app);
    }
}

pub fn fitted_size(window: &tauri::WebviewWindow, target: (f64, f64)) -> tauri::LogicalSize<f64> {
    let screen = window
        .current_monitor()
        .ok()
        .flatten()
        .map(|m| {
            (
                m.size().width as f64 / m.scale_factor(),
                m.size().height as f64 / m.scale_factor(),
            )
        })
        .unwrap_or((1920.0, 1080.0));
    let (width, height) = knowtosay::desktop_behavior::fit_window_size(target, screen);
    tauri::LogicalSize::new(width, height)
}

fn show_panel(app: &tauri::AppHandle, mode: SearchPanelMode) {
    let Some(window) = app.get_webview_window("search") else {
        return;
    };
    if let Some(control) = app.try_state::<DesktopController>() {
        if let Ok(mut current) = control.search_mode.lock() {
            *current = mode;
        }
        #[cfg(windows)]
        {
            // Store the previous foreground HWND as an integer across the UI thread boundary.
            let previous =
                unsafe { windows_sys::Win32::UI::WindowsAndMessaging::GetForegroundWindow() }
                    as isize;
            if !window.is_visible().unwrap_or(false) {
                control.previous_window.store(previous, Ordering::Relaxed);
            }
        }
        #[cfg(not(windows))]
        let _ = &control;
    }
    if let Some(control) = app.try_state::<DesktopController>() {
        control
            .search_expanded
            .store(mode == SearchPanelMode::Quick, Ordering::Relaxed);
    }
    let _ = window.set_size(fitted_size(
        &window,
        (
            720.0,
            if mode == SearchPanelMode::Quick {
                560.0
            } else {
                190.0
            },
        ),
    ));
    #[cfg(windows)]
    if let Some(rect) = cursor_monitor() {
        let width = window.outer_size().map_or(720, |s| s.width) as i32;
        let _ = window.set_position(tauri::PhysicalPosition::new(
            rect.left + ((rect.right - rect.left - width) / 2).max(0),
            rect.top + (rect.bottom - rect.top) / 5,
        ));
    }
    #[cfg(not(windows))]
    let _ = window.center();
    let _ = window.show();
    let _ = window.set_focus();
    let _ = window.emit("quick-search-open", mode);
}

pub fn hide_search(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("search") {
        let _ = window.hide();
    }
    #[cfg(not(windows))]
    if let Some(control) = app.try_state::<DesktopController>() {
        control.previous_window.store(0, Ordering::Relaxed);
    }
    #[cfg(windows)]
    if let Some(control) = app.try_state::<DesktopController>() {
        let hwnd = control.previous_window.swap(0, Ordering::Relaxed);
        if hwnd != 0 {
            unsafe {
                windows_sys::Win32::UI::WindowsAndMessaging::SetForegroundWindow(hwnd as _);
            }
        }
    }
}

#[tauri::command]
pub fn open_search(app: tauri::AppHandle) {
    show_search(&app)
}
#[tauri::command]
pub fn open_quick_replies(app: tauri::AppHandle) {
    show_quick_replies(&app)
}
#[tauri::command]
pub fn close_search(app: tauri::AppHandle) {
    hide_search(&app)
}
#[tauri::command]
pub fn resize_search(app: tauri::AppHandle, expanded: bool) -> Result<(), String> {
    let window = app.get_webview_window("search").ok_or("搜索窗口未创建")?;
    let control = app.state::<DesktopController>();
    if control.search_expanded.swap(expanded, Ordering::Relaxed) == expanded {
        return Ok(());
    }
    let scale = window.scale_factor().map_err(|e| e.to_string())?;
    let current_width = window.inner_size().map_err(|e| e.to_string())?.width as f64 / scale;
    window
        .set_size(fitted_size(
            &window,
            (current_width, if expanded { 560.0 } else { 190.0 }),
        ))
        .map_err(|e| e.to_string())
}
#[tauri::command]
pub fn open_script(app: tauri::AppHandle, id: String) {
    hide_search(&app);
    show_main(&app);
    let _ = app.emit_to("main", "open-script", id);
}
#[tauri::command]
pub fn desktop_status(app: tauri::AppHandle) -> Result<DesktopStatus, String> {
    let control = app.state::<DesktopController>();
    Ok(DesktopStatus {
        preferences: control
            .preferences
            .lock()
            .map_err(|e| e.to_string())?
            .clone(),
        shortcut_registered: control.shortcut_registered.load(Ordering::Relaxed),
        warning: control.warning.lock().map_err(|e| e.to_string())?.clone(),
        search_mode: *control.search_mode.lock().map_err(|e| e.to_string())?,
    })
}
#[tauri::command]
pub fn save_desktop_preferences(
    app: tauri::AppHandle,
    preferences: DesktopPreferences,
) -> Result<DesktopStatus, String> {
    preferences.validate()?;
    let control = app.state::<DesktopController>();
    let mut current = control.preferences.lock().map_err(|e| e.to_string())?;
    let old = current.shortcut.clone();
    let registered = control.shortcut_registered.load(Ordering::Relaxed);
    if old != preferences.shortcut || (!registered && !preferences.shortcut.is_empty()) {
        if !preferences.shortcut.is_empty() {
            app.global_shortcut()
                .register(preferences.shortcut.as_str())
                .map_err(|e| format!("无法注册此快捷键，可能已被占用：{e}"))?;
        }
        if registered
            && !old.is_empty()
            && let Err(e) = app.global_shortcut().unregister(old.as_str())
        {
            let _ = app
                .global_shortcut()
                .unregister(preferences.shortcut.as_str());
            return Err(e.to_string());
        }
    }
    let path = app.state::<AppState>().root.join("desktop-settings.json");
    let json = serde_json::to_vec_pretty(&preferences).map_err(|e| e.to_string())?;
    if let Err(e) = std::fs::write(path, json) {
        if old != preferences.shortcut {
            let _ = app
                .global_shortcut()
                .unregister(preferences.shortcut.as_str());
            if registered {
                let _ = app.global_shortcut().register(old.as_str());
            }
        }
        return Err(e.to_string());
    }
    control
        .shortcut_registered
        .store(!preferences.shortcut.is_empty(), Ordering::Relaxed);
    *current = preferences;
    *control.warning.lock().map_err(|e| e.to_string())? = None;
    drop(current);
    desktop_status(app)
}

pub fn setup(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let root = app.state::<AppState>().root.clone();
    let preferences = std::fs::read(root.join("desktop-settings.json"))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<DesktopPreferences>(&bytes).ok())
        .filter(|p| p.validate().is_ok())
        .unwrap_or_default();
    let registration = if preferences.shortcut.is_empty() {
        Ok(())
    } else {
        app.global_shortcut()
            .register(preferences.shortcut.as_str())
    };
    let warning = registration
        .as_ref()
        .err()
        .map(|e| format!("快捷键暂不可用，请在设置中修改：{e}"));
    app.manage(DesktopController {
        preferences: Mutex::new(preferences.clone()),
        shortcut_registered: AtomicBool::new(
            registration.is_ok() && !preferences.shortcut.is_empty(),
        ),
        warning: Mutex::new(warning),
        previous_window: AtomicIsize::new(0),
        search_mode: Mutex::new(SearchPanelMode::Search),
        search_expanded: AtomicBool::new(false),
        main_bounds: Mutex::new(None),
    });
    WebviewWindowBuilder::new(
        app,
        "search",
        WebviewUrl::App(knowtosay::desktop_behavior::QUICK_SEARCH_PATH.into()),
    )
    .title("KnowToSay 搜索")
    .decorations(false)
    .resizable(true)
    .always_on_top(true)
    .skip_taskbar(true)
    .visible(false)
    .inner_size(720., 190.)
    .min_inner_size(340., 90.)
    .build()?;
    use tauri::{
        menu::{Menu, MenuItem},
        tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    };
    let search = MenuItem::with_id(app, "search", "搜索话术和素材", true, None::<&str>)?;
    let show = MenuItem::with_id(app, "show", "打开工作台", true, None::<&str>)?;
    let exit = MenuItem::with_id(app, "exit", "退出 KnowToSay", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&search, &show, &exit])?;
    let icon = app.default_window_icon().cloned().ok_or("缺少托盘图标")?;
    TrayIconBuilder::with_id("knowtosay-tray")
        .icon(icon)
        .tooltip("KnowToSay · 话术与知识库")
        .show_menu_on_left_click(false)
        .menu(&menu)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "search" => show_search(app),
            "show" => show_main(app),
            "exit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main(tray.app_handle())
            }
        })
        .build(app)?;
    #[cfg(windows)]
    start_edge_watcher(app.handle().clone());
    Ok(())
}

#[cfg(windows)]
fn cursor_monitor() -> Option<windows_sys::Win32::Foundation::RECT> {
    use windows_sys::Win32::{
        Foundation::POINT,
        Graphics::Gdi::{GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromPoint},
        UI::WindowsAndMessaging::GetCursorPos,
    };
    let mut point = POINT { x: 0, y: 0 };
    unsafe {
        if GetCursorPos(&mut point) == 0 {
            return None;
        }
        let monitor = MonitorFromPoint(point, MONITOR_DEFAULTTONEAREST);
        let mut info: MONITORINFO = std::mem::zeroed();
        info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
        if GetMonitorInfoW(monitor, &mut info) == 0 {
            return None;
        }
        Some(info.rcWork)
    }
}

#[cfg(windows)]
fn start_edge_watcher(app: tauri::AppHandle) {
    use windows_sys::Win32::{
        Foundation::POINT,
        Graphics::Gdi::{GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromPoint},
        UI::{
            Input::KeyboardAndMouse::{GetAsyncKeyState, VK_LBUTTON, VK_RBUTTON},
            WindowsAndMessaging::{
                GetClassNameW, GetCursorPos, GetForegroundWindow, GetWindowRect,
                GetWindowThreadProcessId,
            },
        },
    };
    std::thread::spawn(move || {
        let mut pending = String::new();
        let mut since = std::time::Instant::now();
        let mut fired = false;
        loop {
            std::thread::sleep(std::time::Duration::from_millis(60));
            let Some(main) = app.get_webview_window("main") else {
                return;
            };
            if main.is_visible().unwrap_or(true) && !main.is_minimized().unwrap_or(false) {
                pending.clear();
                fired = false;
                continue;
            }
            let Some(control) = app.try_state::<DesktopController>() else {
                continue;
            };
            let preferences = match control.preferences.lock() {
                Ok(p) => p.clone(),
                Err(_) => continue,
            };
            if preferences.edges.is_empty() {
                continue;
            }
            let edge = unsafe {
                if GetAsyncKeyState(VK_LBUTTON as i32) < 0
                    || GetAsyncKeyState(VK_RBUTTON as i32) < 0
                {
                    pending.clear();
                    continue;
                }
                let mut point = POINT { x: 0, y: 0 };
                if GetCursorPos(&mut point) == 0 {
                    continue;
                }
                let monitor = MonitorFromPoint(point, MONITOR_DEFAULTTONEAREST);
                let mut info: MONITORINFO = std::mem::zeroed();
                info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
                if GetMonitorInfoW(monitor, &mut info) == 0 {
                    continue;
                }
                // Suppress full-screen foreground applications, excluding the Windows desktop.
                let foreground = GetForegroundWindow();
                let mut rect = std::mem::zeroed();
                let mut class = [0u16; 128];
                let len = GetClassNameW(foreground, class.as_mut_ptr(), 128);
                let name = String::from_utf16_lossy(&class[..len.max(0) as usize]);
                let mut pid = 0;
                GetWindowThreadProcessId(foreground, &mut pid);
                if pid != std::process::id()
                    && name != "Progman"
                    && name != "WorkerW"
                    && GetWindowRect(foreground, &mut rect) != 0
                    && rect.left <= info.rcMonitor.left
                    && rect.top <= info.rcMonitor.top
                    && rect.right >= info.rcMonitor.right
                    && rect.bottom >= info.rcMonitor.bottom
                {
                    pending.clear();
                    continue;
                }
                let r = info.rcMonitor;
                knowtosay::desktop_behavior::active_edge(
                    (point.x, point.y),
                    (r.left, r.top, r.right, r.bottom),
                    &preferences.edges,
                )
                .unwrap_or("")
            };
            if edge.is_empty() || !preferences.edges.iter().any(|e| e == edge) {
                pending.clear();
                fired = false;
                continue;
            }
            if pending != edge {
                pending = edge.into();
                since = std::time::Instant::now();
                fired = false;
            }
            if !fired && since.elapsed().as_millis() >= preferences.dwell_ms as u128 {
                fired = true;
                show_quick_replies(&app);
            }
        }
    });
}
