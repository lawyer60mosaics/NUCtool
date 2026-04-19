#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]
use std::{
    env,
    thread,
    sync::{Arc, Mutex}
};
use tauri_plugin_autostart::MacosLauncher;
use tauri_plugin_autostart::ManagerExt;

#[cfg(windows)]
mod win_plug;
#[cfg(unix)]
mod linux_plug;
mod plug;
mod tests;

use plug::{
    setup,
    struct_set::FanControlState,
    config::{
        export_monitor_log_csv,
        list_fan_profiles,
        load_fan_config,
        load_fan_profile,
        save_fan_config,
        save_fan_profile,
    },
    fan::{fan_reset, get_current_fan_mode, get_fan_speeds, start_fan_control, stop_fan_control},
    tdp::{get_tdp, set_tdp, set_rgb, get_rgb, set_rgb_color_y, set_rgb_color_n, get_rgb_color},
};

#[cfg(windows)]
use win_plug::{
    permissions::privilege_escalation,
    wmi::wmi_security,
};
#[cfg(unix)]
use linux_plug::{
    sysfs::sys_init,
};

fn main() {
    #[cfg(debug_assertions)]
    let devtools = tauri_plugin_devtools::init();
    #[cfg(debug_assertions)]
    let mut builder = tauri::Builder::default();
    #[cfg(not(debug_assertions))]
    let builder = tauri::Builder::default();
    #[cfg(debug_assertions)]
    {
        builder = builder.plugin(devtools);
    }
    #[cfg(windows)]
    privilege_escalation();
    #[cfg(windows)]
    thread::spawn(move || {
        wmi_security();
        fan_reset();
    });
    #[cfg(unix)]
    sys_init();
    let fan_control_state = FanControlState {
        is_running: Arc::new(Mutex::new(false)),
        active_mode: Arc::new(Mutex::new("office".to_string())),
    };
    builder
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, Some(vec![]), ))
        .manage(fan_control_state)
        .setup(|app| setup::init(app))
        .invoke_handler(tauri::generate_handler![
            start_fan_control,
            stop_fan_control,
            get_current_fan_mode,
            save_fan_config,
            load_fan_config,
            save_fan_profile,
            load_fan_profile,
            list_fan_profiles,
            export_monitor_log_csv,
            get_fan_speeds,
            get_tdp,
            set_tdp,
            set_rgb,
            get_rgb,
            set_rgb_color_y,
            set_rgb_color_n,
            get_rgb_color,
            autostart_status,
            autostart_set
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                window.hide().unwrap();
                api.prevent_close();
            }
        })
        .run(tauri::generate_context!())
        .unwrap();
}

#[tauri::command]
fn autostart_status(app: tauri::AppHandle) -> Result<bool, String> {
    let mgr = app.autolaunch();
    mgr.is_enabled().map_err(|e| e.to_string())
}

#[tauri::command]
fn autostart_set(app: tauri::AppHandle, enable: bool) -> Result<bool, String> {
    let mgr = app.autolaunch();
    let res = if enable { mgr.enable() } else { mgr.disable() };
    res.map_err(|e| e.to_string())?;
    mgr.is_enabled().map_err(|e| e.to_string())
}
