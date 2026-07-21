use serde_json::{json, Map, Value};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
};
use tauri::{AppHandle, Manager, State, WebviewUrl, WebviewWindow};
use tauri_plugin_autostart::ManagerExt as AutostartManagerExt;

const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";

struct Store {
    path: PathBuf,
    log_path: PathBuf,
    data: Mutex<Map<String, Value>>,
    last_heartbeat: std::sync::atomic::AtomicU64,
}

impl Store {
    fn load(app: &AppHandle) -> Result<Self, String> {
        let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let path = dir.join("storage.json");
        let log_dir = app.path().app_log_dir().map_err(|e| e.to_string())?;
        fs::create_dir_all(&log_dir).map_err(|e| e.to_string())?;
        let log_path = log_dir.join("viewer.log");
        let mut data = read_map(&path).unwrap_or_default();
        decrypt_profile_passwords(&mut data);

        let store = Self {
            path,
            log_path,
            data: Mutex::new(data),
            last_heartbeat: std::sync::atomic::AtomicU64::new(unix_timestamp()),
        };
        store.persist()?;
        Ok(store)
    }

    fn persist(&self) -> Result<(), String> {
        let data = self.data.lock().map_err(|e| e.to_string())?;
        let mut persisted = data.clone();
        encrypt_profile_passwords(&mut persisted)?;
        let temp = self.path.with_extension("json.tmp");
        fs::write(
            &temp,
            serde_json::to_vec_pretty(&persisted).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        fs::rename(temp, &self.path).map_err(|e| e.to_string())
    }
}

fn unix_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn transform_profile_passwords(
    data: &mut Map<String, Value>,
    transform: impl Fn(&str) -> Result<String, String>,
) -> Result<(), String> {
    if let Some(items) = data.get_mut("profiles").and_then(Value::as_array_mut) {
        for profile in items {
            if let Some(password) = profile.get_mut("password") {
                if let Some(raw) = password.as_str() {
                    *password = Value::String(transform(raw)?);
                }
            }
        }
    }
    Ok(())
}

fn encrypt_profile_passwords(data: &mut Map<String, Value>) -> Result<(), String> {
    transform_profile_passwords(data, protect_secret)
}

fn decrypt_profile_passwords(data: &mut Map<String, Value>) {
    let _ = transform_profile_passwords(data, unprotect_secret);
}

#[cfg(target_os = "windows")]
fn protect_secret(secret: &str) -> Result<String, String> {
    use base64::Engine;
    use windows_sys::Win32::{
        Foundation::LocalFree,
        Security::Cryptography::{CryptProtectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB},
    };
    if secret.is_empty() || secret.starts_with("dpapi:") {
        return Ok(secret.to_owned());
    }
    let mut input = CRYPT_INTEGER_BLOB {
        cbData: secret.len() as u32,
        pbData: secret.as_ptr() as *mut u8,
    };
    let mut output = CRYPT_INTEGER_BLOB {
        cbData: 0,
        pbData: std::ptr::null_mut(),
    };
    let ok = unsafe {
        CryptProtectData(
            &mut input,
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut output,
        )
    };
    if ok == 0 {
        return Err(std::io::Error::last_os_error().to_string());
    }
    let bytes = unsafe { std::slice::from_raw_parts(output.pbData, output.cbData as usize) };
    let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
    unsafe { LocalFree(output.pbData.cast()) };
    Ok(format!("dpapi:{encoded}"))
}

#[cfg(target_os = "windows")]
fn unprotect_secret(secret: &str) -> Result<String, String> {
    use base64::Engine;
    use windows_sys::Win32::{
        Foundation::LocalFree,
        Security::Cryptography::{
            CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
        },
    };
    let Some(encoded) = secret.strip_prefix("dpapi:") else {
        return Ok(secret.to_owned());
    };
    let encrypted = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .map_err(|e| e.to_string())?;
    let mut input = CRYPT_INTEGER_BLOB {
        cbData: encrypted.len() as u32,
        pbData: encrypted.as_ptr() as *mut u8,
    };
    let mut output = CRYPT_INTEGER_BLOB {
        cbData: 0,
        pbData: std::ptr::null_mut(),
    };
    let ok = unsafe {
        CryptUnprotectData(
            &mut input,
            std::ptr::null_mut(),
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut output,
        )
    };
    if ok == 0 {
        return Err(std::io::Error::last_os_error().to_string());
    }
    let bytes = unsafe { std::slice::from_raw_parts(output.pbData, output.cbData as usize) };
    let result = String::from_utf8(bytes.to_vec()).map_err(|e| e.to_string())?;
    unsafe { LocalFree(output.pbData.cast()) };
    Ok(result)
}

#[cfg(not(target_os = "windows"))]
fn protect_secret(secret: &str) -> Result<String, String> {
    Ok(secret.to_owned())
}

#[cfg(not(target_os = "windows"))]
fn unprotect_secret(secret: &str) -> Result<String, String> {
    Ok(secret.to_owned())
}

fn append_log(store: &Store, source: &str, message: &str) -> Result<(), String> {
    use std::io::Write;
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs();
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&store.log_path)
        .map_err(|e| e.to_string())?;
    writeln!(file, "{timestamp} [{source}] {message}").map_err(|e| e.to_string())
}

fn startup_settings(data: &Map<String, Value>) -> Value {
    let mut defaults = json!({
        "profileId": null,
        "fullscreen": true,
        "displayIndex": 0,
        "startWithWindows": true,
        "autoReconnect": true
    });
    if let (Some(target), Some(saved)) = (
        defaults.as_object_mut(),
        data.get("startupSettings").and_then(Value::as_object),
    ) {
        target.extend(saved.clone());
    }
    defaults
}

fn read_map(path: &Path) -> Option<Map<String, Value>> {
    serde_json::from_slice::<Value>(&fs::read(path).ok()?)
        .ok()?
        .as_object()
        .cloned()
}

fn local_url(page: &str) -> Result<url::Url, String> {
    url::Url::parse(&format!("http://tauri.localhost/html/{page}")).map_err(|e| e.to_string())
}

fn profiles(data: &Map<String, Value>) -> Vec<Value> {
    data.get("profiles")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

fn active_profile(data: &Map<String, Value>) -> Option<Value> {
    let list = profiles(data);
    let id = data.get("activeProfileId").and_then(Value::as_str);
    list.iter()
        .find(|p| p.get("id").and_then(Value::as_str) == id)
        .cloned()
        .or_else(|| list.first().cloned())
}

fn navigate(window: &WebviewWindow, target: &str) -> Result<(), String> {
    let url = if target.starts_with("http://") || target.starts_with("https://") {
        url::Url::parse(target).map_err(|e| e.to_string())?
    } else {
        local_url(target)?
    };
    window.navigate(url).map_err(|e| e.to_string())
}

fn is_local_url(url: &url::Url) -> bool {
    matches!(url.host_str(), Some("tauri.localhost") | Some("localhost"))
}

fn is_configured_origin(store: &Store, url: &url::Url) -> bool {
    if is_local_url(url) {
        return true;
    }
    let data = match store.data.lock() {
        Ok(data) => data,
        Err(_) => return false,
    };
    profiles(&data).iter().any(|profile| {
        profile
            .get("url")
            .and_then(Value::as_str)
            .and_then(|saved| url::Url::parse(saved).ok())
            .is_some_and(|saved| {
                let exact_origin = saved.scheme() == url.scheme()
                    && saved.host_str() == url.host_str()
                    && saved.port_or_known_default() == url.port_or_known_default();
                let unifi_cloud_redirect = saved.host_str() == Some("unifi.ui.com")
                    && url.scheme() == "https"
                    && url
                        .host_str()
                        .is_some_and(|host| host == "ui.com" || host.ends_with(".ui.com"));
                exact_origin || unifi_cloud_redirect
            })
    })
}

fn run_native_action(
    action: String,
    args: Vec<Value>,
    app: AppHandle,
    window: WebviewWindow,
    store: State<'_, Store>,
) -> Result<Value, String> {
    let first = args.first().cloned().unwrap_or(Value::Null);
    let page_url = window.url().map_err(|e| e.to_string())?;
    if !is_configured_origin(&store, &page_url) {
        return Err("This page is not an authorized UniFi console".into());
    }
    if !is_local_url(&page_url)
        && !matches!(
            action.as_str(),
            "configLoad"
                | "startupSettingsGet"
                | "upv:log"
                | "upv:heartbeat"
                | "toggleFullscreen"
                | "switchNextProfile"
                | "openConfig"
                | "restart"
        )
    {
        return Err(format!(
            "Native action {action} is unavailable from the camera page"
        ));
    }
    match action.as_str() {
        "configLoad" => {
            let data = store.data.lock().map_err(|e| e.to_string())?;
            Ok(active_profile(&data).unwrap_or(Value::Null))
        }
        "profilesLoad" => {
            let data = store.data.lock().map_err(|e| e.to_string())?;
            Ok(Value::Array(profiles(&data)))
        }
        "activeProfileGet" => Ok(store
            .data
            .lock()
            .map_err(|e| e.to_string())?
            .get("activeProfileId")
            .cloned()
            .unwrap_or(Value::Null)),
        "startupProfileGet" => Ok(store
            .data
            .lock()
            .map_err(|e| e.to_string())?
            .get("startupSettings")
            .and_then(|v| v.get("profileId"))
            .cloned()
            .unwrap_or(Value::Null)),
        "startupSettingsGet" => {
            let data = store.data.lock().map_err(|e| e.to_string())?;
            Ok(startup_settings(&data))
        }
        "displaysGet" => {
            let primary = window
                .primary_monitor()
                .map_err(|e| e.to_string())?
                .map(|m| m.position().to_owned());
            let displays = window.available_monitors().map_err(|e| e.to_string())?.iter().enumerate().map(|(i, m)| {
                let size=m.size(); let pos=m.position(); let is_primary=primary.as_ref()==Some(pos);
                json!({"index":i,"id":format!("{}:{}",pos.x,pos.y),"isPrimary":is_primary,"label":if is_primary {format!("Primary ({}×{})",size.width,size.height)} else {format!("Display {} ({}×{})",i+1,size.width,size.height)},"width":size.width,"height":size.height,"x":pos.x,"y":pos.y})
            }).collect();
            Ok(Value::Array(displays))
        }
        "connectionTest" => test_connection(first.as_str().unwrap_or_default()),
        "diagnosticsGet" => {
            let data = store.data.lock().map_err(|e| e.to_string())?;
            Ok(json!({
                "version": env!("CARGO_PKG_VERSION"),
                "profiles": profiles(&data).len(),
                "settings": startup_settings(&data),
                "storageProtected": cfg!(target_os = "windows"),
                "logPath": store.log_path.to_string_lossy(),
                "autostartEnabled": app.autolaunch().is_enabled().unwrap_or(false)
            }))
        }
        "supportBundleCreate" => {
            let data = store.data.lock().map_err(|e| e.to_string())?;
            let report_path = store.log_path.with_file_name("support-report.txt");
            let report = format!(
                "Unifi Protect Viewer support report\nVersion: {}\nProfiles: {}\nStartup settings: {}\nLog: {}\nPasswords: excluded\n",
                env!("CARGO_PKG_VERSION"),
                profiles(&data).len(),
                startup_settings(&data),
                store.log_path.display()
            );
            fs::write(&report_path, report).map_err(|e| e.to_string())?;
            std::process::Command::new("explorer.exe")
                .arg("/select,")
                .arg(&report_path)
                .spawn()
                .map_err(|e| e.to_string())?;
            Ok(Value::String(report_path.to_string_lossy().into_owned()))
        }
        "profilesSave" => {
            store
                .data
                .lock()
                .map_err(|e| e.to_string())?
                .insert("profiles".into(), first);
            store.persist()?;
            Ok(Value::Null)
        }
        "activeProfileSet" => {
            store
                .data
                .lock()
                .map_err(|e| e.to_string())?
                .insert("activeProfileId".into(), first);
            store.persist()?;
            Ok(Value::Null)
        }
        "startupProfileSet" => update_startup(&app, &store, json!({"profileId":first})),
        "startupSettingsSet" => update_startup(&app, &store, first),
        "configSave" => {
            save_config(&store, first)?;
            Ok(Value::Null)
        }
        "reset" => {
            store.data.lock().map_err(|e| e.to_string())?.clear();
            store.persist()?;
            Ok(Value::Null)
        }
        "restart" => {
            std::process::Command::new(std::env::current_exe().map_err(|e| e.to_string())?)
                .spawn()
                .map_err(|e| e.to_string())?;
            app.exit(0);
            Ok(Value::Null)
        }
        "openConfig" => {
            navigate(&window, "config.html")?;
            Ok(Value::Null)
        }
        "switchNextProfile" => {
            let data = store.data.lock().map_err(|e| e.to_string())?;
            if profiles(&data).len() <= 1 {
                return Ok(Value::Null);
            }
            drop(data);
            navigate(&window, "profile-select.html")?;
            Ok(Value::Null)
        }
        "launchProfile" => {
            navigate_to_profile(&store, &window, first.as_str().unwrap_or_default())?;
            Ok(Value::Null)
        }
        "toggleFullscreen" => {
            window
                .set_fullscreen(!window.is_fullscreen().map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
            Ok(Value::Null)
        }
        "openDevTools" => {
            window.open_devtools();
            Ok(Value::Null)
        }
        "openExternal" => {
            let u = first.as_str().unwrap_or_default();
            let parsed = url::Url::parse(u).map_err(|e| e.to_string())?;
            if !matches!(parsed.scheme(), "http" | "https") {
                return Err("only http(s) URLs are allowed".into());
            }
            std::process::Command::new("explorer.exe")
                .arg(u)
                .spawn()
                .map_err(|e| e.to_string())?;
            Ok(Value::Null)
        }
        "openLogFile" => {
            if !store.log_path.exists() {
                append_log(&store, "app", "log opened")?;
            }
            std::process::Command::new("explorer.exe")
                .arg(&store.log_path)
                .spawn()
                .map_err(|e| e.to_string())?;
            Ok(Value::Null)
        }
        "upv:log" => {
            if let Some(msg) = first.as_str() {
                append_log(&store, "window", msg)?;
            }
            Ok(Value::Null)
        }
        "upv:heartbeat" => {
            store
                .last_heartbeat
                .store(unix_timestamp(), std::sync::atomic::Ordering::Relaxed);
            Ok(Value::Null)
        }
        other => Err(format!("unknown native action: {other}")),
    }
}

// Each frontend operation is an explicit Tauri command. The dispatcher below
// remains an internal implementation detail so renderer pages never exchange
// generic channels or emulate another desktop runtime.
#[tauri::command]
fn config_load(
    app: AppHandle,
    window: WebviewWindow,
    store: State<'_, Store>,
) -> Result<Value, String> {
    run_native_action("configLoad".into(), vec![], app, window, store)
}
#[tauri::command]
fn profiles_load(
    app: AppHandle,
    window: WebviewWindow,
    store: State<'_, Store>,
) -> Result<Value, String> {
    run_native_action("profilesLoad".into(), vec![], app, window, store)
}
#[tauri::command]
fn active_profile_get(
    app: AppHandle,
    window: WebviewWindow,
    store: State<'_, Store>,
) -> Result<Value, String> {
    run_native_action("activeProfileGet".into(), vec![], app, window, store)
}
#[tauri::command]
fn startup_profile_get(
    app: AppHandle,
    window: WebviewWindow,
    store: State<'_, Store>,
) -> Result<Value, String> {
    run_native_action("startupProfileGet".into(), vec![], app, window, store)
}
#[tauri::command]
fn startup_settings_get(
    app: AppHandle,
    window: WebviewWindow,
    store: State<'_, Store>,
) -> Result<Value, String> {
    run_native_action("startupSettingsGet".into(), vec![], app, window, store)
}
#[tauri::command]
fn displays_get(
    app: AppHandle,
    window: WebviewWindow,
    store: State<'_, Store>,
) -> Result<Value, String> {
    run_native_action("displaysGet".into(), vec![], app, window, store)
}
#[tauri::command]
fn connection_test(
    url: String,
    app: AppHandle,
    window: WebviewWindow,
    store: State<'_, Store>,
) -> Result<Value, String> {
    run_native_action(
        "connectionTest".into(),
        vec![Value::String(url)],
        app,
        window,
        store,
    )
}
#[tauri::command]
fn diagnostics_get(
    app: AppHandle,
    window: WebviewWindow,
    store: State<'_, Store>,
) -> Result<Value, String> {
    run_native_action("diagnosticsGet".into(), vec![], app, window, store)
}
#[tauri::command]
fn support_bundle_create(
    app: AppHandle,
    window: WebviewWindow,
    store: State<'_, Store>,
) -> Result<Value, String> {
    run_native_action("supportBundleCreate".into(), vec![], app, window, store)
}
#[tauri::command]
fn profiles_save(
    profiles: Value,
    app: AppHandle,
    window: WebviewWindow,
    store: State<'_, Store>,
) -> Result<Value, String> {
    run_native_action("profilesSave".into(), vec![profiles], app, window, store)
}
#[tauri::command]
fn active_profile_set(
    id: Value,
    app: AppHandle,
    window: WebviewWindow,
    store: State<'_, Store>,
) -> Result<Value, String> {
    run_native_action("activeProfileSet".into(), vec![id], app, window, store)
}
#[tauri::command]
fn startup_profile_set(
    id: Value,
    app: AppHandle,
    window: WebviewWindow,
    store: State<'_, Store>,
) -> Result<Value, String> {
    run_native_action("startupProfileSet".into(), vec![id], app, window, store)
}
#[tauri::command]
fn startup_settings_set(
    settings: Value,
    app: AppHandle,
    window: WebviewWindow,
    store: State<'_, Store>,
) -> Result<Value, String> {
    run_native_action(
        "startupSettingsSet".into(),
        vec![settings],
        app,
        window,
        store,
    )
}
#[tauri::command]
fn config_save(
    config: Value,
    app: AppHandle,
    window: WebviewWindow,
    store: State<'_, Store>,
) -> Result<Value, String> {
    run_native_action("configSave".into(), vec![config], app, window, store)
}
#[tauri::command]
fn reset(app: AppHandle, window: WebviewWindow, store: State<'_, Store>) -> Result<Value, String> {
    run_native_action("reset".into(), vec![], app, window, store)
}
#[tauri::command]
fn restart(
    app: AppHandle,
    window: WebviewWindow,
    store: State<'_, Store>,
) -> Result<Value, String> {
    run_native_action("restart".into(), vec![], app, window, store)
}
#[tauri::command]
fn open_config(
    app: AppHandle,
    window: WebviewWindow,
    store: State<'_, Store>,
) -> Result<Value, String> {
    run_native_action("openConfig".into(), vec![], app, window, store)
}
#[tauri::command]
fn switch_next_profile(
    app: AppHandle,
    window: WebviewWindow,
    store: State<'_, Store>,
) -> Result<Value, String> {
    run_native_action("switchNextProfile".into(), vec![], app, window, store)
}
#[tauri::command]
fn launch_profile(
    id: String,
    app: AppHandle,
    window: WebviewWindow,
    store: State<'_, Store>,
) -> Result<Value, String> {
    run_native_action(
        "launchProfile".into(),
        vec![Value::String(id)],
        app,
        window,
        store,
    )
}
#[tauri::command]
fn toggle_fullscreen(
    app: AppHandle,
    window: WebviewWindow,
    store: State<'_, Store>,
) -> Result<Value, String> {
    run_native_action("toggleFullscreen".into(), vec![], app, window, store)
}
#[tauri::command]
fn set_fullscreen(window: WebviewWindow, fullscreen: bool) -> Result<(), String> {
    window.set_fullscreen(fullscreen).map_err(|e| e.to_string())
}
#[cfg(target_os = "windows")]
fn system_idle_seconds() -> u32 {
    use windows_sys::Win32::{
        System::SystemInformation::GetTickCount,
        UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO},
    };
    let mut info = LASTINPUTINFO {
        cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32,
        dwTime: 0,
    };
    if unsafe { GetLastInputInfo(&mut info) } == 0 {
        return 0;
    }
    unsafe { GetTickCount() }.wrapping_sub(info.dwTime) / 1_000
}

#[cfg(not(target_os = "windows"))]
fn system_idle_seconds() -> u32 {
    0
}
#[tauri::command]
fn open_devtools(
    app: AppHandle,
    window: WebviewWindow,
    store: State<'_, Store>,
) -> Result<Value, String> {
    run_native_action("openDevTools".into(), vec![], app, window, store)
}
#[tauri::command]
fn open_external(
    url: String,
    app: AppHandle,
    window: WebviewWindow,
    store: State<'_, Store>,
) -> Result<Value, String> {
    run_native_action(
        "openExternal".into(),
        vec![Value::String(url)],
        app,
        window,
        store,
    )
}
#[tauri::command]
fn open_log_file(
    app: AppHandle,
    window: WebviewWindow,
    store: State<'_, Store>,
) -> Result<Value, String> {
    run_native_action("openLogFile".into(), vec![], app, window, store)
}
#[tauri::command]
fn viewer_log(
    message: String,
    app: AppHandle,
    window: WebviewWindow,
    store: State<'_, Store>,
) -> Result<Value, String> {
    run_native_action(
        "upv:log".into(),
        vec![Value::String(message)],
        app,
        window,
        store,
    )
}
#[tauri::command]
fn heartbeat(
    app: AppHandle,
    window: WebviewWindow,
    store: State<'_, Store>,
) -> Result<Value, String> {
    run_native_action("upv:heartbeat".into(), vec![], app, window, store)
}

fn test_connection(target: &str) -> Result<Value, String> {
    use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
    use std::time::Duration;
    let parsed = url::Url::parse(target).map_err(|_| "Enter a valid http:// or https:// URL")?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err("Only http:// and https:// addresses are supported".into());
    }
    let host = parsed.host_str().ok_or("The URL does not contain a host")?;
    let port = parsed
        .port_or_known_default()
        .ok_or("The URL does not contain a usable port")?;
    let addresses: Vec<SocketAddr> = (host, port)
        .to_socket_addrs()
        .map_err(|_| format!("Could not resolve {host}"))?
        .collect();
    if addresses.is_empty() {
        return Err(format!("Could not resolve {host}"));
    }
    let started = std::time::Instant::now();
    let reachable = addresses
        .iter()
        .take(4)
        .any(|address| TcpStream::connect_timeout(address, Duration::from_secs(3)).is_ok());
    let elapsed_ms = started.elapsed().as_millis();
    let looks_like_liveview = parsed.path().contains("/protect/");
    if reachable {
        Ok(json!({
            "ok": true,
            "host": host,
            "port": port,
            "elapsedMs": elapsed_ms,
            "looksLikeLiveview": looks_like_liveview,
            "message": if looks_like_liveview {
                format!("Network connection successful. Reached {host}:{port} in {elapsed_ms} ms. Save and launch to verify the account and live view.")
            } else {
                format!("Reached {host}:{port} in {elapsed_ms} ms, but this does not look like a Protect live-view URL. Open the desired live view in Protect and copy its complete address.")
            }
        }))
    } else {
        Ok(json!({
            "ok": false,
            "host": host,
            "port": port,
            "elapsedMs": elapsed_ms,
            "looksLikeLiveview": looks_like_liveview,
            "message": format!("Could not reach {host}:{port}. Confirm this PC is on the camera network, the address is correct, and the UniFi console is powered on.")
        }))
    }
}

fn update_startup(app: &AppHandle, store: &Store, patch: Value) -> Result<Value, String> {
    let mut data = store.data.lock().map_err(|e| e.to_string())?;
    let mut settings = data
        .get("startupSettings")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_else(|| {
            json!({"profileId":null,"fullscreen":true,"displayIndex":0,"startWithWindows":true,"autoReconnect":true})
                .as_object()
                .unwrap()
                .clone()
        });
    if let Some(p) = patch.as_object() {
        for (k, v) in p {
            settings.insert(k.clone(), v.clone());
        }
    }
    data.insert("startupSettings".into(), Value::Object(settings));
    drop(data);
    store.persist()?;
    sync_autostart(app, store)?;
    Ok(Value::Null)
}

fn sync_autostart(app: &AppHandle, store: &Store) -> Result<(), String> {
    let enabled = {
        let data = store.data.lock().map_err(|e| e.to_string())?;
        startup_settings(&data)
            .get("startWithWindows")
            .and_then(Value::as_bool)
            .unwrap_or(true)
    };
    let manager = app.autolaunch();
    if enabled {
        manager.enable().map_err(|e| e.to_string())?;
    } else {
        manager.disable().map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn save_config(store: &Store, config: Value) -> Result<(), String> {
    let mut data = store.data.lock().map_err(|e| e.to_string())?;
    let mut list = profiles(&data);
    let id = data
        .get("activeProfileId")
        .and_then(Value::as_str)
        .map(str::to_owned);
    if let Some(item) = list
        .iter_mut()
        .find(|p| p.get("id").and_then(Value::as_str) == id.as_deref())
    {
        if let (Some(dst), Some(src)) = (item.as_object_mut(), config.as_object()) {
            dst.extend(src.clone());
        }
    }
    data.insert("profiles".into(), Value::Array(list));
    drop(data);
    store.persist()
}

fn navigate_to_profile(store: &Store, window: &WebviewWindow, id: &str) -> Result<(), String> {
    let mut data = store.data.lock().map_err(|e| e.to_string())?;
    let profile = profiles(&data)
        .into_iter()
        .find(|p| p.get("id").and_then(Value::as_str) == Some(id))
        .ok_or("profile not found")?;
    data.insert("activeProfileId".into(), Value::String(id.into()));
    let target = profile
        .get("url")
        .and_then(Value::as_str)
        .ok_or("profile URL missing")?
        .to_owned();
    drop(data);
    store.persist()?;
    let host = url::Url::parse(&target)
        .ok()
        .and_then(|url| url.host_str().map(str::to_owned))
        .unwrap_or_else(|| "invalid host".into());
    append_log(store, "app", &format!("launching profile on {host}"))?;
    navigate(window, &target).map_err(|error| {
        let _ = append_log(store, "app", &format!("profile launch failed: {error}"));
        error
    })
}

fn initial_page(store: &Store) -> WebviewUrl {
    let data = store.data.lock().expect("store lock");
    let list = profiles(&data);
    if list.is_empty() {
        return WebviewUrl::App("html/config.html".into());
    }
    let selected = selected_profile_url(&data);
    selected
        .and_then(|u| url::Url::parse(&u).ok())
        .map(WebviewUrl::External)
        .unwrap_or_else(|| WebviewUrl::App("html/profile-select.html".into()))
}

fn selected_profile_url(data: &Map<String, Value>) -> Option<String> {
    let list = profiles(data);
    let configured_id = startup_settings(data)
        .get("profileId")
        .and_then(Value::as_str)
        .map(str::to_owned);
    configured_id
        .and_then(|id| {
            list.iter()
                .find(|profile| profile.get("id").and_then(Value::as_str) == Some(id.as_str()))
                .cloned()
        })
        .or_else(|| (list.len() == 1).then(|| list[0].clone()))
        .and_then(|p| p.get("url").and_then(Value::as_str).map(str::to_owned))
}

pub fn run() {
    // UniFi appliances commonly use a locally issued/self-signed certificate.
    #[cfg(target_os = "windows")]
    std::env::set_var(
        "WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS",
        if cfg!(debug_assertions) {
            "--ignore-certificate-errors --remote-debugging-port=9222"
        } else {
            "--ignore-certificate-errors"
        },
    );
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_autostart::Builder::new().build())
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                // Keep the native idle watchdog alive when the operator clicks
                // X. The hidden viewer will return after the idle threshold.
                api.prevent_close();
                let _ = window.hide();
                if let Some(store) = window.try_state::<Store>() {
                    let _ = append_log(&store, "window", "viewer closed to background");
                }
            }
        })
        .setup(|app| {
            let store = Store::load(app.handle()).map_err(std::io::Error::other)?;
            // WebView2 keeps its encrypted UniFi cookies here, allowing valid
            // sessions to survive application and machine restarts.
            let webview_data_dir = app.path().app_local_data_dir()?.join("EBWebView");
            fs::create_dir_all(&webview_data_dir)?;
            let page = initial_page(&store);
            let settings = {
                let data = store
                    .data
                    .lock()
                    .map_err(|e| std::io::Error::other(e.to_string()))?;
                startup_settings(&data)
            };
            let has_profiles = {
                let data = store
                    .data
                    .lock()
                    .map_err(|e| std::io::Error::other(e.to_string()))?;
                !profiles(&data).is_empty()
            };
            let fullscreen = has_profiles
                && settings
                    .get("fullscreen")
                    .and_then(Value::as_bool)
                    .unwrap_or(true);
            let display_index = settings
                .get("displayIndex")
                .and_then(Value::as_u64)
                .unwrap_or(0) as usize;
            app.manage(store);
            let mut builder = tauri::WebviewWindowBuilder::new(app, "main", page)
                .title("Unifi Protect Viewer")
                .inner_size(1280.0, 760.0)
                .min_inner_size(800.0, 500.0)
                .user_agent(USER_AGENT)
                .data_directory(webview_data_dir);

            let navigation_handle = app.handle().clone();
            builder = builder.on_navigation(move |url| {
                let allowed = navigation_handle
                    .try_state::<Store>()
                    .is_some_and(|store| is_configured_origin(&store, url));
                if !allowed {
                    if let Some(store) = navigation_handle.try_state::<Store>() {
                        let host = url.host_str().unwrap_or("unknown host");
                        let _ = append_log(
                            &store,
                            "navigation",
                            &format!("blocked navigation to {}://{host}", url.scheme()),
                        );
                    }
                }
                allowed
            });
            let page_load_handle = app.handle().clone();
            let camera_automation = include_str!("../../src/js/preload.js").to_owned();
            builder = builder.on_page_load(move |window, payload| {
                if let Some(store) = page_load_handle.try_state::<Store>() {
                    let url = payload.url();
                    let host = url.host_str().unwrap_or("local viewer");
                    let _ = append_log(
                        &store,
                        "navigation",
                        &format!("page loaded: {}://{host}{}", url.scheme(), url.path()),
                    );
                }
                if payload.event() == tauri::webview::PageLoadEvent::Finished {
                    // Authentication may temporarily navigate through a normal
                    // login window. As soon as Protect loads, force the native
                    // window back into kiosk fullscreen before revealing video.
                    if payload.url().path().contains("/protect/") {
                        if let Err(error) = window.set_fullscreen(true) {
                            if let Some(store) = page_load_handle.try_state::<Store>() {
                                let _ = append_log(
                                    &store,
                                    "window",
                                    &format!("could not enter camera fullscreen: {error}"),
                                );
                            }
                        }
                    }
                    let profile = page_load_handle
                        .try_state::<Store>()
                        .and_then(|store| {
                            store
                                .data
                                .lock()
                                .ok()
                                .and_then(|data| active_profile(&data))
                        })
                        .unwrap_or(Value::Null);
                    let injected = format!(
                        "window.__UPV_PROFILE__ = {};\n{}",
                        serde_json::to_string(&profile).unwrap_or_else(|_| "null".into()),
                        camera_automation
                    );
                    if let Err(error) = window.eval(injected) {
                        if let Some(store) = page_load_handle.try_state::<Store>() {
                            let _ = append_log(
                                &store,
                                "navigation",
                                &format!("camera automation injection failed: {error}"),
                            );
                        }
                    } else if let Err(error) = window.eval("startCameraPage()") {
                        if let Some(store) = page_load_handle.try_state::<Store>() {
                            let _ = append_log(
                                &store,
                                "navigation",
                                &format!("camera automation start failed: {error}"),
                            );
                        }
                    }
                }
            });

            let monitors = app.available_monitors()?;
            if let Some(monitor) = monitors.get(display_index).or_else(|| monitors.first()) {
                let position = monitor.position();
                builder = builder.position(position.x as f64, position.y as f64);
            }
            let window = builder.build()?;
            if fullscreen {
                window.set_fullscreen(true)?;
            }
            if let Some(store) = app.try_state::<Store>() {
                let _ = append_log(&store, "app", "viewer started");
                let _ = sync_autostart(app.handle(), &store);
            }
            let watchdog_app = app.handle().clone();
            std::thread::spawn(move || loop {
                std::thread::sleep(std::time::Duration::from_secs(30));
                let Some(store) = watchdog_app.try_state::<Store>() else {
                    continue;
                };
                let last = store
                    .last_heartbeat
                    .load(std::sync::atomic::Ordering::Relaxed);
                let configured = store
                    .data
                    .lock()
                    .map(|data| !profiles(&data).is_empty())
                    .unwrap_or(false);
                let is_local_page = watchdog_app
                    .get_webview_window("main")
                    .and_then(|window| window.url().ok())
                    .is_some_and(|url| is_local_url(&url));
                if configured && is_local_page && unix_timestamp().saturating_sub(last) > 120 {
                    let _ =
                        append_log(&store, "watchdog", "renderer heartbeat stopped; restarting");
                    if let Ok(exe) = std::env::current_exe() {
                        let _ = std::process::Command::new(exe).spawn();
                    }
                    watchdog_app.exit(2);
                    break;
                }
            });

            // This must be native: background WebViews throttle JavaScript
            // timers. Restore the unattended camera wall once per idle period.
            let idle_app = app.handle().clone();
            std::thread::spawn(move || {
                let mut restored_for_current_idle = false;
                loop {
                    std::thread::sleep(std::time::Duration::from_secs(5));
                    if system_idle_seconds() < 60 {
                        restored_for_current_idle = false;
                        continue;
                    }
                    if restored_for_current_idle {
                        continue;
                    }
                    let Some(window) = idle_app.get_webview_window("main") else {
                        continue;
                    };
                    let is_camera_page = window
                        .url()
                        .ok()
                        .is_some_and(|url| url.path().contains("/protect/"));
                    if !is_camera_page {
                        continue;
                    }

                    let _ = window.show();
                    let _ = window.unminimize();
                    let _ = window.set_fullscreen(true);
                    let _ = window.set_focus();
                    let _ = window.eval("enterUniFiFullscreen().catch(() => {})");
                    if let Some(store) = idle_app.try_state::<Store>() {
                        let _ = append_log(
                            &store,
                            "idle",
                            "60 seconds idle; brought camera viewer to foreground and fullscreen",
                        );
                    }
                    restored_for_current_idle = true;
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            config_load,
            profiles_load,
            active_profile_get,
            startup_profile_get,
            startup_settings_get,
            displays_get,
            connection_test,
            diagnostics_get,
            support_bundle_create,
            profiles_save,
            active_profile_set,
            startup_profile_set,
            startup_settings_set,
            config_save,
            reset,
            restart,
            open_config,
            switch_next_profile,
            launch_profile,
            toggle_fullscreen,
            set_fullscreen,
            open_devtools,
            open_external,
            open_log_file,
            viewer_log,
            heartbeat
        ])
        .run(tauri::generate_context!())
        .expect("error while running application");
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_non_http_navigation() {
        assert!(url::Url::parse("not a url").is_err());
    }
    #[test]
    fn profile_selection_prefers_active() {
        let data = serde_json::from_value::<Map<String, Value>>(
            json!({"activeProfileId":"b","profiles":[{"id":"a"},{"id":"b","name":"B"}]}),
        )
        .unwrap();
        assert_eq!(active_profile(&data).unwrap()["name"], "B");
    }

    #[test]
    fn kiosk_defaults_are_enabled() {
        let data = Map::new();
        let settings = startup_settings(&data);
        assert_eq!(settings["fullscreen"], true);
        assert_eq!(settings["startWithWindows"], true);
        assert_eq!(settings["autoReconnect"], true);
    }

    #[test]
    fn startup_profile_wins_over_active_profile() {
        let data = serde_json::from_value::<Map<String, Value>>(json!({
            "activeProfileId":"a",
            "startupSettings":{"profileId":"b"},
            "profiles":[
                {"id":"a","url":"https://one.local/protect"},
                {"id":"b","url":"https://two.local/protect"}
            ]
        }))
        .unwrap();
        assert_eq!(
            selected_profile_url(&data).as_deref(),
            Some("https://two.local/protect")
        );
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn windows_password_protection_round_trips() {
        let encrypted = protect_secret("store-camera-password").unwrap();
        assert!(encrypted.starts_with("dpapi:"));
        assert_ne!(encrypted, "store-camera-password");
        assert_eq!(
            unprotect_secret(&encrypted).unwrap(),
            "store-camera-password"
        );
    }

    #[test]
    fn connection_test_rejects_non_web_urls() {
        assert!(test_connection("file:///camera").is_err());
        assert!(test_connection("not a url").is_err());
    }

    #[test]
    fn connection_test_returns_structured_failure() {
        let result = test_connection("http://127.0.0.1:9/protect/dashboard/test").unwrap();
        assert_eq!(result["ok"], false);
        assert_eq!(result["host"], "127.0.0.1");
        assert_eq!(result["port"], 9);
        assert_eq!(result["looksLikeLiveview"], true);
        assert!(result["elapsedMs"].is_number());
    }
}
