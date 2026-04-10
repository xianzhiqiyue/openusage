use tauri::{AppHandle, Manager, Position, Size};

#[cfg(target_os = "macos")]
use tauri_nspanel::{
    CollectionBehavior, ManagerExt, PanelLevel, StyleMask, WebviewWindowExt, tauri_panel,
};

#[cfg(target_os = "macos")]
macro_rules! get_or_init_panel {
    ($app_handle:expr) => {
        match $app_handle.get_webview_panel("main") {
            Ok(panel) => Some(panel),
            Err(_) => {
                if let Err(err) = crate::panel::init($app_handle) {
                    log::error!("Failed to init panel: {}", err);
                    None
                } else {
                    match $app_handle.get_webview_panel("main") {
                        Ok(panel) => Some(panel),
                        Err(err) => {
                            log::error!("Panel missing after init: {:?}", err);
                            None
                        }
                    }
                }
            }
        }
    };
}

fn with_main_window<F>(app_handle: &AppHandle, f: F)
where
    F: FnOnce(tauri::WebviewWindow),
{
    match app_handle.get_webview_window("main") {
        Some(window) => f(window),
        None => log::error!("Main window not found"),
    }
}

pub fn show_panel(app_handle: &AppHandle) {
    #[cfg(target_os = "macos")]
    {
        if let Some(panel) = get_or_init_panel!(app_handle) {
            panel.show_and_make_key();
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        with_main_window(app_handle, |window| {
            let _ = window.unminimize();
            let _ = window.show();
            let _ = window.set_focus();
        });
    }
}

pub fn hide_panel(app_handle: &AppHandle) {
    #[cfg(target_os = "macos")]
    {
        use tauri_nspanel::ManagerExt;

        if let Ok(panel) = app_handle.get_webview_panel("main") {
            panel.hide();
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        with_main_window(app_handle, |window| {
            let _ = window.hide();
        });
    }
}

pub fn toggle_panel(app_handle: &AppHandle) {
    #[cfg(target_os = "macos")]
    {
        let Some(panel) = get_or_init_panel!(app_handle) else {
            return;
        };

        if panel.is_visible() {
            log::debug!("toggle_panel: hiding panel");
            panel.hide();
        } else {
            log::debug!("toggle_panel: showing panel");
            panel.show_and_make_key();
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        with_main_window(app_handle, |window| {
            let is_visible = window.is_visible().unwrap_or(false);
            if is_visible {
                log::debug!("toggle_panel: hiding window");
                let _ = window.hide();
            } else {
                log::debug!("toggle_panel: showing window");
                let _ = window.unminimize();
                let _ = window.show();
                let _ = window.set_focus();
            }
        });
    }
}

pub fn handle_tray_click(app_handle: &AppHandle, icon_position: Position, icon_size: Size) {
    #[cfg(target_os = "macos")]
    {
        let Some(panel) = get_or_init_panel!(app_handle) else {
            return;
        };

        if panel.is_visible() {
            log::debug!("tray click: hiding panel");
            panel.hide();
            return;
        }

        log::debug!("tray click: showing panel");
        panel.show_and_make_key();
        position_panel_at_tray_icon(app_handle, icon_position, icon_size);
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = (icon_position, icon_size);
        with_main_window(app_handle, |window| {
            let is_visible = window.is_visible().unwrap_or(false);
            if is_visible {
                log::debug!("tray click: hiding window");
                let _ = window.hide();
            } else {
                log::debug!("tray click: showing window");
                let _ = window.unminimize();
                let _ = window.show();
                let _ = window.center();
                let _ = window.set_focus();
            }
        });
    }
}

#[cfg(target_os = "macos")]
tauri_panel! {
    panel!(OpenUsagePanel {
        config: {
            can_become_key_window: true,
            is_floating_panel: true
        }
    })

    panel_event!(OpenUsagePanelEventHandler {
        window_did_resign_key(notification: &NSNotification) -> ()
    })
}

pub fn init(app_handle: &tauri::AppHandle) -> tauri::Result<()> {
    #[cfg(target_os = "macos")]
    {
        if app_handle.get_webview_panel("main").is_ok() {
            return Ok(());
        }

        let window = app_handle.get_webview_window("main").unwrap();

        let panel = window.to_panel::<OpenUsagePanel>()?;

        // Disable native shadow - it causes gray border on transparent windows
        // Let CSS handle shadow via shadow-xl class
        panel.set_has_shadow(false);
        panel.set_opaque(false);

        panel.set_level(PanelLevel::MainMenu.value() + 1);
        panel.set_collection_behavior(
            CollectionBehavior::new()
                .move_to_active_space()
                .full_screen_auxiliary()
                .value(),
        );
        panel.set_style_mask(StyleMask::empty().nonactivating_panel().value());

        let event_handler = OpenUsagePanelEventHandler::new();
        let handle = app_handle.clone();
        event_handler.window_did_resign_key(move |_notification| {
            if let Ok(panel) = handle.get_webview_panel("main") {
                panel.hide();
            }
        });
        panel.set_event_handler(Some(event_handler.as_ref()));
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = app_handle.get_webview_window("main");
    }

    Ok(())
}

#[cfg(target_os = "macos")]
pub fn position_panel_at_tray_icon(
    app_handle: &tauri::AppHandle,
    icon_position: Position,
    icon_size: Size,
) {
    let window = app_handle.get_webview_window("main").unwrap();

    // Tray icon events on macOS report coordinates in a hybrid physical space where
    // each monitor region uses its own scale (logical_pos × scale = physical origin).
    // On mixed-DPI setups this creates overlapping regions, making it impossible to
    // reliably determine the correct monitor from tray coordinates alone.
    //
    // Instead, we use NSEvent::mouseLocation() which returns the cursor position in
    // macOS's unified logical (point) coordinate space — always unambiguous regardless
    // of how many monitors or scale factors are involved. We find which monitor
    // contains the cursor, then convert the tray icon's physical coordinates to
    // logical coordinates within that monitor.

    let (icon_phys_x, icon_phys_y) = match &icon_position {
        Position::Physical(pos) => (pos.x as f64, pos.y as f64),
        Position::Logical(pos) => (pos.x, pos.y),
    };
    let (icon_phys_w, icon_phys_h) = match &icon_size {
        Size::Physical(s) => (s.width as f64, s.height as f64),
        Size::Logical(s) => (s.width, s.height),
    };

    let mouse_logical = objc2_app_kit::NSEvent::mouseLocation();
    let monitors = window.available_monitors().expect("failed to get monitors");
    let primary_logical_h = window
        .primary_monitor()
        .ok()
        .flatten()
        .map(|m| m.size().height as f64 / m.scale_factor())
        .unwrap_or(0.0);

    let mouse_x = mouse_logical.x;
    let mouse_y = primary_logical_h - mouse_logical.y;

    let mut found_monitor = None;
    for m in &monitors {
        let pos = m.position();
        let scale = m.scale_factor();
        let logical_w = m.size().width as f64 / scale;
        let logical_h = m.size().height as f64 / scale;

        let logical_x = pos.x as f64 / scale;
        let logical_y = pos.y as f64 / scale;
        let x_in = mouse_x >= logical_x && mouse_x < logical_x + logical_w;
        let y_in = mouse_y >= logical_y && mouse_y < logical_y + logical_h;

        if x_in && y_in {
            found_monitor = Some(m.clone());
            break;
        }
    }

    let monitor = match found_monitor {
        Some(m) => m,
        None => {
            log::warn!(
                "No monitor found for cursor at ({:.0}, {:.0}), using primary",
                mouse_x,
                mouse_y
            );
            match window.primary_monitor() {
                Ok(Some(m)) => m,
                _ => return,
            }
        }
    };

    let target_scale = monitor.scale_factor();
    let mon_logical_x = monitor.position().x as f64;
    let mon_logical_y = monitor.position().y as f64;
    let phys_origin_x = mon_logical_x * target_scale;
    let phys_origin_y = mon_logical_y * target_scale;

    let icon_logical_x = mon_logical_x + (icon_phys_x - phys_origin_x) / target_scale;
    let icon_logical_y = mon_logical_y + (icon_phys_y - phys_origin_y) / target_scale;
    let icon_logical_w = icon_phys_w / target_scale;
    let icon_logical_h = icon_phys_h / target_scale;

    let panel_width = match (window.outer_size(), window.scale_factor()) {
        (Ok(s), Ok(win_scale)) => s.width as f64 / win_scale,
        _ => {
            let conf: serde_json::Value = serde_json::from_str(include_str!("../tauri.conf.json"))
                .expect("tauri.conf.json must be valid JSON");
            conf["app"]["windows"][0]["width"]
                .as_f64()
                .expect("width must be set in tauri.conf.json")
        }
    };

    let icon_center_x = icon_logical_x + (icon_logical_w / 2.0);
    let panel_x = icon_center_x - (panel_width / 2.0);
    let nudge_up: f64 = 6.0;
    let panel_y = icon_logical_y + icon_logical_h - nudge_up;

    let _ = window.set_position(tauri::LogicalPosition::new(panel_x, panel_y));
}
