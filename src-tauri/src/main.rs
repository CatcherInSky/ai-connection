mod models;
mod probe;
mod storage;

use crate::models::{ProbeResult, Service, Settings};
use crate::probe::run_probe_once;
use crate::storage::Storage;
use chrono::Utc;
use rusqlite::Error as SqlError;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio::time::sleep;
#[cfg(not(target_os = "linux"))]
use tauri::{image::Image, menu::{MenuBuilder, MenuItem}};
use tauri::{Emitter, Manager, State};

#[derive(Debug)]
struct AppState {
    services: Vec<Service>,
    settings: Settings,
}

type SharedState<'a> = State<'a, Arc<Mutex<AppState>>>;
type SharedStorage<'a> = State<'a, Arc<Storage>>;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FrontendSettings {
    probe_interval_ms: u64,
}

#[tauri::command]
async fn get_settings(state: SharedState<'_>) -> Result<FrontendSettings, String> {
    let guard = state.lock().unwrap();
    Ok(FrontendSettings {
        probe_interval_ms: guard.settings.probe_interval_ms,
    })
}

#[tauri::command]
async fn get_traffic_state(state: SharedState<'_>) -> Result<Settings, String> {
    let guard = state.lock().unwrap();
    Ok(guard.settings.clone())
}

#[tauri::command]
async fn get_services(state: SharedState<'_>) -> Result<Vec<Service>, String> {
    let guard = state.lock().unwrap();
    Ok(guard.services.clone())
}

#[tauri::command]
async fn run_single_probe(
    storage: SharedStorage<'_>,
    state: SharedState<'_>,
    service_id: String,
) -> Result<ProbeResult, String> {
    let service = {
        let guard = state.lock().unwrap();
        guard
            .services
            .iter()
            .find(|s| s.id == service_id && s.enabled)
            .cloned()
    };

    let Some(service) = service else {
        return Err("service not found or disabled".into());
    };

    let timestamp = Utc::now().timestamp_millis();
    let result = run_probe_once(&service, timestamp)
        .await
        .map_err(|e| e.to_string())?;
    let _ = storage.insert_probe(&result);
    Ok(result)
}

/// 对所有启用的服务并发探测，返回每个服务的探测结果。
#[tauri::command]
async fn run_probe_all(
    storage: SharedStorage<'_>,
    state: SharedState<'_>,
    app: tauri::AppHandle,
) -> Result<Vec<ProbeResult>, String> {
    let services: Vec<Service> = {
        let guard = state.lock().unwrap();
        guard
            .services
            .iter()
            .filter(|s| s.enabled)
            .cloned()
            .collect()
    };

    let timestamp = Utc::now().timestamp_millis();
    let handles: Vec<_> = services
        .iter()
        .map(|service| {
            let svc = service.clone();
            tokio::spawn(async move { run_probe_once(&svc, timestamp).await })
        })
        .collect();

    let mut results = Vec::with_capacity(handles.len());
    for (i, handle) in handles.into_iter().enumerate() {
        let service_id = services
            .get(i)
            .map(|s| s.id.clone())
            .unwrap_or_else(|| "unknown".into());

        match handle.await {
            Ok(Ok(result)) => {
                let _ = storage.insert_probe(&result);
                let _ = app.emit("probe_result", &result);

                let used_kb = (result.estimated_bytes.max(0) as u64 + 1023) / 1024;
                {
                    let mut guard = state.lock().unwrap();
                    guard.settings.traffic_used_today_kb =
                        guard.settings.traffic_used_today_kb.saturating_add(used_kb);
                }
                results.push(result);
            }
            Ok(Err(_)) => {
                let fail =
                    ProbeResult::new_failure(service_id, "unknown");
                let _ = storage.insert_probe(&fail);
                let _ = app.emit("probe_result", &fail);
                results.push(fail);
            }
            Err(_) => {
                let fail = ProbeResult::new_failure(service_id, "unknown");
                let _ = storage.insert_probe(&fail);
                let _ = app.emit("probe_result", &fail);
                results.push(fail);
            }
        }
    }
    Ok(results)
}

#[tauri::command]
async fn get_recent_probes(
    storage: SharedStorage<'_>,
    service_id: String,
    since_ms: i64,
) -> Result<Vec<ProbeResult>, String> {
    storage
        .load_recent_probes(&service_id, since_ms)
        .map_err(|e: SqlError| e.to_string())
}

/// 获取所有服务在指定时间范围内的探测记录，用于同时展示多服务图表。
#[tauri::command]
async fn get_recent_probes_all(
    storage: SharedStorage<'_>,
    since_ms: i64,
) -> Result<Vec<ProbeResult>, String> {
    storage
        .load_recent_probes_all(since_ms)
        .map_err(|e: SqlError| e.to_string())
}

#[tauri::command]
async fn update_probe_interval(state: SharedState<'_>, interval_ms: u64) -> Result<(), String> {
    let mut guard = state.lock().unwrap();
    guard.settings.probe_interval_ms = interval_ms.max(5_000);
    Ok(())
}

fn default_services() -> Vec<Service> {
    vec![
        Service::new(
            "claude",
            "Claude",
            "https://api.anthropic.com/",
            "/",
            5_000,
        ),
        Service::new(
            "cursor",
            "Cursor",
            "https://api.cursor.com/",
            "/",
            5_000,
        ),
        Service::new(
            "gemini",
            "Gemini",
            "https://generativelanguage.googleapis.com/",
            "/",
            5_000,
        ),
    ]
}

fn default_settings() -> Settings {
    Settings {
        probe_interval_ms: 30_000,
        daily_traffic_budget_kb: 50_000,
        traffic_used_today_kb: 0,
        traffic_day_start_ms: Utc::now().timestamp_millis(),
    }
}

fn build_initial_state() -> Arc<Mutex<AppState>> {
    Arc::new(Mutex::new(AppState {
        services: default_services(),
        settings: default_settings(),
    }))
}

/// 创建系统托盘图标及菜单，支持常驻后台。
/// Linux 需安装 libayatana-appindicator3-1，否则跳过托盘（避免崩溃）。
#[cfg(not(target_os = "linux"))]
fn setup_tray(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    use tauri::tray::TrayIconBuilder;

    let show_item = MenuItem::with_id(app, "show", "显示主窗口", true, None::<&str>)?;
    let probe_item = MenuItem::with_id(app, "probe", "立即探测全部", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;

    let menu = MenuBuilder::new(app)
        .item(&show_item)
        .item(&probe_item)
        .item(&quit_item)
        .build()?;

    let icon_path = std::path::Path::new("icons").join("icon.png");
    let icon = if icon_path.exists() {
        Image::from_path(icon_path)?
    } else {
        Image::from_bytes(include_bytes!("../icons/icon.png"))?
    };

    let _tray = TrayIconBuilder::with_id("main")
        .icon(icon)
        .menu(&menu)
        .tooltip("AI Connection Monitor - 点击显示主窗口")
        .on_menu_event(move |app, event| {
            match event.id.as_ref() {
                "show" => {
                    if let Some(w) = app.get_webview_window("main") {
                        let _ = w.show();
                        let _ = w.set_focus();
                    }
                }
                "probe" => {
                    let _ = app.emit("tray_probe_now", ());
                }
                "quit" => {
                    app.exit(0);
                }
                _ => {}
            }
        })
        .build(app)?;

    Ok(())
}

#[cfg(target_os = "linux")]
fn setup_tray(_app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}

fn main() {
    let app_state = build_initial_state();
    let state_for_task = app_state.clone();

    tauri::Builder::default()
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            get_services,
            run_single_probe,
            run_probe_all,
            get_recent_probes,
            get_recent_probes_all,
            get_settings,
            get_traffic_state,
            update_probe_interval
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .setup(move |app| {
            if let Err(e) = setup_tray(app) {
                eprintln!("托盘初始化失败: {}", e);
            }

            let app_handle = app.handle().clone();
            let db_path = app
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join("probes.db");
            if let Some(parent) = db_path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }

            let storage = Storage::new(db_path).expect("failed to open probes database");
            let storage = Arc::new(storage);
            let storage_for_task = storage.clone();

            app.manage(storage);

            tauri::async_runtime::spawn(async move {
                use std::time::Duration;
                sleep(Duration::from_secs(3)).await;
                loop {
                    let _ = app_handle.emit("probe_started", ());

                    let (services, mut settings) = {
                        let guard = state_for_task.lock().unwrap();
                        (guard.services.clone(), guard.settings.clone())
                    };

                    let now = Utc::now().timestamp_millis();
                    let day_ms = 24_i64 * 60 * 60 * 1000;
                    if now - settings.traffic_day_start_ms >= day_ms {
                        let mut guard = state_for_task.lock().unwrap();
                        guard.settings.traffic_used_today_kb = 0;
                        guard.settings.traffic_day_start_ms = now;
                        settings.traffic_used_today_kb = 0;
                        settings.traffic_day_start_ms = now;
                    }

                    let enabled: Vec<_> = services
                        .into_iter()
                        .filter(|s| s.enabled)
                        .collect();
                    let timestamp = Utc::now().timestamp_millis();
                    let handles: Vec<_> = enabled
                        .iter()
                        .map(|service| {
                            let svc = service.clone();
                            tokio::spawn(async move { run_probe_once(&svc, timestamp).await })
                        })
                        .collect();

                    let mut batch: Vec<ProbeResult> = Vec::with_capacity(handles.len());
                    for (i, handle) in handles.into_iter().enumerate() {
                        let service_id = enabled
                            .get(i)
                            .map(|s| s.id.clone())
                            .unwrap_or_else(|| "unknown".into());
                        let result = match handle.await {
                            Ok(Ok(r)) => r,
                            Ok(Err(_)) => ProbeResult::new_failure(service_id.clone(), "unknown"),
                            Err(_) => ProbeResult::new_failure(service_id.clone(), "unknown"),
                        };
                        let _ = storage_for_task.insert_probe(&result);
                        batch.push(result.clone());

                        if result.reachable {
                            let used_kb = (result.estimated_bytes.max(0) as u64 + 1023) / 1024;
                            {
                                let mut guard = state_for_task.lock().unwrap();
                                guard.settings.traffic_used_today_kb =
                                    guard.settings.traffic_used_today_kb.saturating_add(used_kb);
                            }
                            settings.traffic_used_today_kb =
                                settings.traffic_used_today_kb.saturating_add(used_kb);

                            if settings.traffic_used_today_kb
                                > settings.daily_traffic_budget_kb.saturating_mul(9) / 10
                            {
                                settings.probe_interval_ms =
                                    (settings.probe_interval_ms as f64 * 1.5).round() as u64;
                                let mut guard = state_for_task.lock().unwrap();
                                guard.settings.probe_interval_ms = settings.probe_interval_ms;
                            }
                        }
                    }
                    let _ = app_handle.emit("probe_batch", &batch);

                    let interval_ms = {
                        let guard = state_for_task.lock().unwrap();
                        guard.settings.probe_interval_ms
                    };
                    sleep(Duration::from_millis(interval_ms)).await;
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

