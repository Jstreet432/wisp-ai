use super::Host;
use serde_json::{json, Value};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Mutex,
};
use std::time::Duration;
use tauri::{Emitter, Manager};
use tauri_plugin_updater::{Update, UpdaterExt};

const STABLE_FEED: &str = "https://github.com/fedeya/wisp-ai/releases/latest/download/latest.json";
const NIGHTLY_FEED: &str =
    "https://github.com/fedeya/wisp-ai/releases/download/nightly/latest.json";

fn default_channel() -> &'static str {
    if env!("CARGO_PKG_VERSION").contains("-nightly.") {
        "nightly"
    } else {
        "stable"
    }
}
fn channel_allows(nightly: bool, current: &semver::Version, remote: &semver::Version) -> bool {
    if nightly {
        remote.pre.as_str().starts_with("nightly.") && remote > current
    } else {
        remote.pre.is_empty()
            && (remote > current
                || (current.pre.as_str().starts_with("nightly.") && remote != current))
    }
}
fn read_channel(data: &std::path::Path) -> Result<String, String> {
    match std::fs::read_to_string(data.join("update-channel.txt")) {
        Ok(value) if value == "stable" || value == "nightly" => Ok(value),
        Ok(_) => Err("Invalid saved update channel.".into()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(default_channel().into()),
        Err(error) => Err(error.to_string()),
    }
}
const AUTO_INSTALL_FILE: &str = "update-auto-install.txt";
/// Left by an automatic install that ran while Wisp was in the tray, so the
/// relaunched app goes back to the tray instead of opening on screen.
const RELAUNCH_MINIMIZED_FILE: &str = "update-relaunch-minimized";
const BUSY_TURN: &str = "Wait for the active turn before stopping Wisp";
/// Returned when an automatic install finds a reply running; the app retries later.
const AUTOMATIC_BUSY: &str = "Wisp is busy; the update will install later.";

fn read_auto_install(data: &std::path::Path) -> Result<bool, String> {
    match std::fs::read_to_string(data.join(AUTO_INSTALL_FILE)) {
        Ok(value) if value == "on" => Ok(true),
        Ok(value) if value == "off" => Ok(false),
        Ok(_) => Err("Invalid saved automatic update setting.".into()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(true),
        Err(error) => Err(error.to_string()),
    }
}

/// Reads or saves whether downloaded updates install by themselves while Wisp is idle.
#[tauri::command]
pub fn update_auto_install(
    host: tauri::State<'_, Host>,
    enabled: Option<bool>,
) -> Result<bool, String> {
    if let Some(enabled) = enabled {
        std::fs::create_dir_all(&host.data).map_err(|e| e.to_string())?;
        std::fs::write(
            host.data.join(AUTO_INSTALL_FILE),
            if enabled { "on" } else { "off" },
        )
        .map_err(|e| e.to_string())?;
        return Ok(enabled);
    }
    read_auto_install(&host.data)
}

/// Whether the previous run asked to reopen in the tray. Consumes the request.
pub fn take_relaunch_minimized(data: &std::path::Path) -> bool {
    std::fs::remove_file(data.join(RELAUNCH_MINIMIZED_FILE)).is_ok()
}

#[tauri::command]
pub fn update_channel(
    host: tauri::State<'_, Host>,
    updates: tauri::State<'_, Updates>,
    channel: Option<String>,
) -> Result<String, String> {
    let _operation = updates.begin()?;
    if let Some(channel) = channel {
        if channel != "stable" && channel != "nightly" {
            return Err("Unknown update channel.".into());
        }
        *updates
            .pending
            .lock()
            .map_err(|_| "Update state is unavailable.")? = None;
        updates.clear_download();
        std::fs::create_dir_all(&host.data).map_err(|e| e.to_string())?;
        std::fs::write(host.data.join("update-channel.txt"), &channel)
            .map_err(|e| e.to_string())?;
        return Ok(channel);
    }
    read_channel(&host.data)
}

#[derive(Default)]
pub struct Updates {
    pending: Mutex<Option<Update>>,
    /// The verified installer of a background download, by version.
    downloaded: Mutex<Option<(String, Vec<u8>)>>,
    /// Serializes background downloads: a repeated request waits for the one in
    /// flight and then reuses its installer instead of being refused.
    downloading: tauri::async_runtime::Mutex<()>,
    busy: AtomicBool,
}

struct Operation<'a>(&'a AtomicBool);
impl Drop for Operation<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}
impl Updates {
    fn begin(&self) -> Result<Operation<'_>, String> {
        self.busy
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| "An update operation is already in progress.".to_string())?;
        Ok(Operation(&self.busy))
    }

    fn clear_download(&self) {
        if let Ok(mut downloaded) = self.downloaded.lock() {
            *downloaded = None;
        }
    }

    fn downloaded_bytes(&self, version: &str) -> Option<Vec<u8>> {
        self.downloaded
            .lock()
            .ok()?
            .as_ref()
            .filter(|(saved, _)| saved == version)
            .map(|(_, bytes)| bytes.clone())
    }

    fn pending_update(&self) -> Result<Update, String> {
        self.pending
            .lock()
            .map_err(|_| "Update state is unavailable.")?
            .clone()
            .ok_or_else(|| "Check for updates before installing.".to_string())
    }
}

#[tauri::command]
pub async fn check_update(
    app: tauri::AppHandle,
    updates: tauri::State<'_, Updates>,
) -> Result<Value, String> {
    // Source builds and unsupported platforms must not replace themselves with a Windows release.
    if cfg!(debug_assertions) || !cfg!(all(windows, target_arch = "x86_64")) {
        return Ok(json!({"supported": false, "update": null}));
    }
    let _operation = updates.begin()?;
    let channel = read_channel(&app.state::<Host>().data)?;
    let nightly = channel == "nightly";
    let update = app
        .updater_builder()
        .endpoints(vec![if nightly { NIGHTLY_FEED } else { STABLE_FEED }
            .parse()
            .map_err(|_| "Invalid update endpoint.")?])
        .map_err(|e| e.to_string())?
        .version_comparator(move |current, remote| {
            channel_allows(nightly, &current, &remote.version)
        })
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?
        .check()
        .await
        .map_err(|e| e.to_string())?;
    let details = update
        .as_ref()
        .map(|u| json!({"version": u.version, "notes": u.body}));
    let version = update.as_ref().map(|u| u.version.as_str());
    if version.is_none_or(|version| updates.downloaded_bytes(version).is_none()) {
        updates.clear_download();
    }
    *updates
        .pending
        .lock()
        .map_err(|_| "Update state is unavailable.")? = update;
    Ok(json!({"supported": true, "update": details}))
}

/// Downloads and verifies the update found by the last check without installing it,
/// so an automatic install later only has to run the installer. Returns its version.
#[tauri::command]
pub async fn download_update(updates: tauri::State<'_, Updates>) -> Result<String, String> {
    let _downloading = updates.downloading.lock().await;
    // Read the update only after any earlier download finished, so a request that
    // waited reuses that installer or downloads what the last check found.
    let mut update = updates.pending_update()?;
    if updates.downloaded_bytes(&update.version).is_some() {
        return Ok(update.version);
    }
    update.timeout = Some(Duration::from_secs(30 * 60));
    let bytes = update
        .download(|_, _| {}, || {})
        .await
        .map_err(|e| e.to_string())?;
    // A check or channel change during the download may have replaced the update.
    if updates.pending_update().map(|u| u.version) != Ok(update.version.clone()) {
        return Err("The update changed while downloading.".into());
    }
    *updates
        .downloaded
        .lock()
        .map_err(|_| "Update state is unavailable.")? = Some((update.version.clone(), bytes));
    Ok(update.version)
}

fn main_window_hidden(app: &tauri::AppHandle) -> bool {
    app.get_webview_window("main")
        .is_some_and(|window| !window.is_visible().unwrap_or(true))
}

/// Installs the update found by the last check. An `automatic` install never waits
/// for a reply: it gives up with [`AUTOMATIC_BUSY`] so the app can retry when idle.
#[tauri::command]
pub async fn install_update(
    app: tauri::AppHandle,
    updates: tauri::State<'_, Updates>,
    automatic: Option<bool>,
) -> Result<(), String> {
    let automatic = automatic.unwrap_or(false);
    let _operation = updates.begin()?;
    let mut update = updates.pending_update()?;
    update.timeout = Some(Duration::from_secs(30 * 60));
    let host = app.state::<Host>();
    super::update_trace::begin(&host.data);
    if automatic {
        super::update_trace::append(&host.data, "automatic-install");
    }
    let bytes = match updates.downloaded_bytes(&update.version) {
        Some(bytes) => bytes,
        None => {
            // Download and verify the signature while Wisp is still running.
            let mut downloaded = 0u64;
            update
                .download(
                    |chunk, total| {
                        downloaded += chunk as u64;
                        let _ = app.emit(
                            "update-progress",
                            json!({"phase": "downloading", "downloaded": downloaded, "total": total}),
                        );
                    },
                    || {},
                )
                .await
                .map_err(|e| e.to_string())?
        }
    };
    super::update_trace::append(&host.data, "download-verified");
    let _ = app.emit("update-progress", json!({"phase": "waiting"}));
    let _lifecycle = host.lifecycle.lock().await;
    // /stop atomically refuses active turns. Retry only that refusal, preserving replies.
    loop {
        match super::stop_host(&host).await {
            Ok(()) => break,
            Err(error) if error == BUSY_TURN && automatic => return Err(AUTOMATIC_BUSY.into()),
            Err(error) if error == BUSY_TURN => {
                tauri::async_runtime::spawn_blocking(|| std::thread::sleep(Duration::from_secs(1)))
                    .await
                    .map_err(|e| e.to_string())?;
            }
            Err(error) => return Err(error),
        }
    }
    super::update_trace::append(&host.data, "bridge-stopped-before-install");
    let _ = app.emit("update-progress", json!({"phase": "installing"}));
    let relaunch_minimized = automatic && main_window_hidden(&app);
    if relaunch_minimized {
        let _ = std::fs::write(host.data.join(RELAUNCH_MINIMIZED_FILE), "");
    }
    // The NSIS hook also waits for the bridge process to finish releasing its files.
    if let Err(error) = update.install(bytes) {
        super::update_trace::append(&host.data, "installer-launch-failed");
        if relaunch_minimized {
            take_relaunch_minimized(&host.data);
        }
        let _ = super::start_host(&host).await;
        return Err(error.to_string());
    }
    app.restart();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channels_are_isolated_and_only_returning_to_stable_allows_downgrades() {
        let allowed = |nightly, current: &str, remote: &str| {
            channel_allows(nightly, &current.parse().unwrap(), &remote.parse().unwrap())
        };
        assert!(!allowed(false, "0.1.0", "0.1.1-nightly.100.1"));
        assert!(!allowed(true, "0.1.1-nightly.100.1", "0.2.0"));
        assert!(allowed(true, "0.1.0", "0.1.1-nightly.100.1"));
        assert!(allowed(true, "0.1.1-nightly.9.1", "0.1.1-nightly.10.1"));
        assert!(!allowed(true, "0.1.1-nightly.10.1", "0.1.1-nightly.9.1"));
        assert!(allowed(false, "0.1.1-nightly.10.1", "0.1.0"));
        assert!(!allowed(false, "0.1.1", "0.1.0"));
        assert!(!allowed(false, "0.1.0", "0.1.0"));
    }

    #[test]
    fn checks_and_installs_are_serialized_and_errors_release_the_operation() {
        let updates = Updates::default();
        let operation = updates.begin().unwrap();
        assert!(updates.begin().is_err());
        drop(operation);
        assert!(updates.begin().is_ok());
    }

    #[test]
    fn automatic_install_defaults_on_and_relaunch_marker_is_consumed_once() {
        let dir = std::env::temp_dir().join(format!("wisp-auto-install-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        assert!(read_auto_install(&dir).unwrap());
        std::fs::write(dir.join(AUTO_INSTALL_FILE), "off").unwrap();
        assert!(!read_auto_install(&dir).unwrap());
        std::fs::write(dir.join(AUTO_INSTALL_FILE), "maybe").unwrap();
        assert!(read_auto_install(&dir).is_err());

        assert!(!take_relaunch_minimized(&dir));
        std::fs::write(dir.join(RELAUNCH_MINIMIZED_FILE), "").unwrap();
        assert!(take_relaunch_minimized(&dir));
        assert!(!take_relaunch_minimized(&dir));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn downloaded_installers_only_match_their_version() {
        let updates = Updates::default();
        *updates.downloaded.lock().unwrap() = Some(("0.4.0".into(), vec![1, 2, 3]));
        assert_eq!(updates.downloaded_bytes("0.4.0"), Some(vec![1, 2, 3]));
        assert_eq!(updates.downloaded_bytes("0.4.1"), None);
        updates.clear_download();
        assert_eq!(updates.downloaded_bytes("0.4.0"), None);
    }
}
