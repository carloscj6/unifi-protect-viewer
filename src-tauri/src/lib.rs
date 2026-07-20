use serde_json::{json, Map, Value};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
};
use tauri::{AppHandle, Manager, State, WebviewUrl, WebviewWindow};

const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";

struct Store {
    path: PathBuf,
    data: Mutex<Map<String, Value>>,
}

impl Store {
    fn load(app: &AppHandle) -> Result<Self, String> {
        let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let path = dir.join("storage.json");
        let mut data = read_map(&path).unwrap_or_default();

        // One-time, non-destructive import of the Electron store.
        if data.is_empty() {
            if let Some(old) = electron_store_path() {
                if let Some(imported) = read_map(&old) {
                    data = imported;
                }
            }
        }
        let store = Self {
            path,
            data: Mutex::new(data),
        };
        store.persist()?;
        Ok(store)
    }

    fn persist(&self) -> Result<(), String> {
        let data = self.data.lock().map_err(|e| e.to_string())?;
        let temp = self.path.with_extension("json.tmp");
        fs::write(
            &temp,
            serde_json::to_vec_pretty(&*data).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        fs::rename(temp, &self.path).map_err(|e| e.to_string())
    }
}

fn read_map(path: &Path) -> Option<Map<String, Value>> {
    serde_json::from_slice::<Value>(&fs::read(path).ok()?)
        .ok()?
        .as_object()
        .cloned()
}

fn electron_store_path() -> Option<PathBuf> {
    std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .map(|p| p.join("unifi-protect-viewer").join("config.json"))
        .filter(|p| p.is_file())
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

#[tauri::command]
fn ipc(
    channel: String,
    args: Vec<Value>,
    app: AppHandle,
    window: WebviewWindow,
    store: State<'_, Store>,
) -> Result<Value, String> {
    let first = args.first().cloned().unwrap_or(Value::Null);
    match channel.as_str() {
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
        "startupSettingsGet" => Ok(store
            .data
            .lock()
            .map_err(|e| e.to_string())?
            .get("startupSettings")
            .cloned()
            .unwrap_or(json!({"profileId":null,"fullscreen":false,"displayIndex":0}))),
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
        "startupProfileSet" => update_startup(&store, json!({"profileId":first})),
        "startupSettingsSet" => update_startup(&store, first),
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
            let page = if profiles(&data).len() > 1 {
                "profile-select.html"
            } else {
                "config.html"
            };
            drop(data);
            navigate(&window, page)?;
            Ok(Value::Null)
        }
        "launchProfile" => {
            launch_profile(&store, &window, first.as_str().unwrap_or_default())?;
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
        "openLogFile" => Ok(Value::Null),
        "upv:log" => {
            if let Some(msg) = first.as_str() {
                println!("{msg}");
            }
            Ok(Value::Null)
        }
        other => Err(format!("unknown IPC channel: {other}")),
    }
}

fn update_startup(store: &Store, patch: Value) -> Result<Value, String> {
    let mut data = store.data.lock().map_err(|e| e.to_string())?;
    let mut settings = data
        .get("startupSettings")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_else(|| {
            json!({"profileId":null,"fullscreen":false,"displayIndex":0})
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
    Ok(Value::Null)
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

fn launch_profile(store: &Store, window: &WebviewWindow, id: &str) -> Result<(), String> {
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
    navigate(window, &target)
}

fn initial_page(store: &Store) -> WebviewUrl {
    let data = store.data.lock().expect("store lock");
    let list = profiles(&data);
    if list.is_empty() {
        return WebviewUrl::App("html/config.html".into());
    }
    let selected =
        active_profile(&data).and_then(|p| p.get("url").and_then(Value::as_str).map(str::to_owned));
    selected
        .and_then(|u| url::Url::parse(&u).ok())
        .map(WebviewUrl::External)
        .unwrap_or_else(|| WebviewUrl::App("html/profile-select.html".into()))
}

pub fn run() {
    // UniFi appliances commonly use a locally issued/self-signed certificate.
    #[cfg(target_os = "windows")]
    std::env::set_var(
        "WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS",
        "--ignore-certificate-errors",
    );
    tauri::Builder::default()
        .setup(|app| {
            let store = Store::load(app.handle()).map_err(std::io::Error::other)?;
            let page = initial_page(&store);
            app.manage(store);
            let init = format!(
                "{}\n{}",
                include_str!("bridge.js"),
                include_str!("../../src/js/preload.js")
            );
            tauri::WebviewWindowBuilder::new(app, "main", page)
                .title("Unifi Protect Viewer")
                .inner_size(1280.0, 760.0)
                .min_inner_size(800.0, 500.0)
                .user_agent(USER_AGENT)
                .initialization_script(&init)
                .build()?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![ipc])
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
}
