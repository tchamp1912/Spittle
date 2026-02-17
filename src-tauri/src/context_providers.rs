use crate::settings::AppSettings;
use std::path::PathBuf;

#[cfg(target_os = "macos")]
use log::debug;

#[cfg(target_os = "macos")]
use std::process::Command;

#[cfg(target_os = "macos")]
fn get_frontmost_app_bundle_id() -> Option<String> {
    let output = Command::new("osascript")
        .arg("-e")
        .arg("tell application \"System Events\" to get bundle identifier of first application process whose frontmost is true")
        .output()
        .ok()?;

    if output.status.success() {
        let id = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !id.is_empty() {
            return Some(id);
        }
    }
    None
}

#[cfg(target_os = "macos")]
fn get_cursor_workspace() -> Option<PathBuf> {
    let home = dirs_or_home()?;
    let context_file = home
        .join("Library")
        .join("Caches")
        .join("spittle")
        .join("cursor_context.json");

    let content = std::fs::read_to_string(&context_file).ok()?;
    let parsed: serde_json::Value = serde_json::from_str(&content).ok()?;

    let roots = parsed.get("workspaceRoots")?.as_array()?;
    let first = roots.first()?.as_str()?;
    let path = PathBuf::from(first);

    if path.is_dir() {
        Some(path)
    } else {
        None
    }
}

#[cfg(target_os = "macos")]
fn get_iterm2_cwd() -> Option<PathBuf> {
    let output = Command::new("osascript")
        .arg("-e")
        .arg("tell application \"iTerm2\" to tell current session of current window to get variable named \"path\"")
        .output()
        .ok()?;

    if output.status.success() {
        let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !path_str.is_empty() {
            let path = PathBuf::from(&path_str);
            if path.is_dir() {
                return Some(path);
            }
        }
    }
    None
}

#[cfg(target_os = "macos")]
fn get_terminal_context_cwd() -> Option<PathBuf> {
    let home = dirs_or_home()?;
    let context_file = home
        .join("Library")
        .join("Caches")
        .join("spittle")
        .join("terminal_context.json");

    let content = std::fs::read_to_string(&context_file).ok()?;
    let parsed: serde_json::Value = serde_json::from_str(&content).ok()?;
    let cwd = parsed.get("cwd")?.as_str()?;
    let path = PathBuf::from(cwd);
    if path.is_dir() {
        Some(path)
    } else {
        None
    }
}

#[cfg(target_os = "macos")]
fn is_terminal_bundle_id(id: &str) -> bool {
    let id_lower = id.to_ascii_lowercase();
    id_lower.contains("iterm2")
        || id_lower.contains("terminal")
        || id_lower.contains("warp")
        || id_lower.contains("wezterm")
        || id_lower.contains("alacritty")
        || id_lower.contains("kitty")
}

#[cfg(target_os = "macos")]
fn dirs_or_home() -> Option<PathBuf> {
    std::env::var("HOME").ok().map(PathBuf::from)
}

#[cfg(target_os = "macos")]
pub fn get_workspace_root(settings: &AppSettings) -> Option<PathBuf> {
    let bundle_id = get_frontmost_app_bundle_id();
    debug!("Frontmost app bundle ID: {:?}", bundle_id);

    if let Some(ref id) = bundle_id {
        // Cursor / VS Code
        if id.contains("Cursor") || id.contains("VSCode") || id.contains("vscode") {
            if let Some(root) = get_cursor_workspace() {
                debug!("Resolved workspace from Cursor context: {:?}", root);
                return Some(root);
            }
        }

        // Terminal apps (iTerm2/Terminal/Warp/others)
        if is_terminal_bundle_id(id) {
            if let Some(cwd) = get_iterm2_cwd() {
                debug!("Resolved workspace from iTerm2 CWD: {:?}", cwd);
                return Some(cwd);
            }
            if let Some(cwd) = get_terminal_context_cwd() {
                debug!("Resolved workspace from terminal context file: {:?}", cwd);
                return Some(cwd);
            }
        }
    }

    // Fallback: most recent workspace root from settings
    for root_str in &settings.recent_workspace_roots {
        let path = PathBuf::from(root_str);
        if path.is_dir() {
            debug!("Falling back to MRU workspace root: {:?}", path);
            return Some(path);
        }
    }

    None
}

#[cfg(target_os = "macos")]
pub fn update_mru(app: &tauri::AppHandle, workspace_root: &std::path::Path) {
    let root_str = workspace_root.to_string_lossy().to_string();
    let mut settings = crate::settings::get_settings(app);

    // Remove if already present, then push to front
    settings.recent_workspace_roots.retain(|r| r != &root_str);
    settings.recent_workspace_roots.insert(0, root_str);
    settings.recent_workspace_roots.truncate(5);

    crate::settings::write_settings(app, settings);
}

#[cfg(not(target_os = "macos"))]
pub fn get_workspace_root(_settings: &AppSettings) -> Option<PathBuf> {
    None
}

#[cfg(not(target_os = "macos"))]
pub fn update_mru(_app: &tauri::AppHandle, _workspace_root: &std::path::Path) {
    // No-op on non-macOS platforms
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;
    use once_cell::sync::Lazy;
    use std::process::Command;
    use std::sync::Mutex;
    use tempfile::TempDir;

    static ENV_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

    struct EnvVarGuard {
        key: &'static str,
        previous: Option<String>,
    }

    struct ItermWindowGuard {
        window_id: String,
    }

    impl EnvVarGuard {
        fn set(key: &'static str, value: &str) -> Self {
            let previous = std::env::var(key).ok();
            std::env::set_var(key, value);
            Self { key, previous }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            if let Some(prev) = self.previous.as_ref() {
                std::env::set_var(self.key, prev);
            } else {
                std::env::remove_var(self.key);
            }
        }
    }

    impl Drop for ItermWindowGuard {
        fn drop(&mut self) {
            close_iterm_window(&self.window_id);
        }
    }

    fn run_osascript(script: &str) -> Option<String> {
        let out = Command::new("osascript")
            .arg("-e")
            .arg(script)
            .output()
            .ok()?;
        if !out.status.success() {
            return None;
        }
        Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
    }

    fn close_iterm_window(window_id: &str) {
        let script = format!(
            r#"tell application "iTerm2"
if exists (first window whose id is {}) then
close (first window whose id is {})
end if
end tell"#,
            window_id, window_id
        );
        let _ = run_osascript(&script);
    }

    #[test]
    fn test_get_terminal_context_cwd_reads_cache_file() {
        let _lock = ENV_LOCK.lock().unwrap();
        let home = TempDir::new().unwrap();
        let cache_dir = home.path().join("Library").join("Caches").join("spittle");
        std::fs::create_dir_all(&cache_dir).unwrap();

        let workspace = TempDir::new().unwrap();
        let context = format!(r#"{{"cwd":"{}"}}"#, workspace.path().display());
        std::fs::write(cache_dir.join("terminal_context.json"), context).unwrap();

        let _home_guard = EnvVarGuard::set("HOME", &home.path().to_string_lossy());
        let result = get_terminal_context_cwd();
        assert_eq!(result, Some(workspace.path().to_path_buf()));
    }

    #[test]
    #[ignore = "requires iTerm2 + macOS Automation permissions; run locally"]
    fn test_get_workspace_root_with_real_iterm2_process() {
        let _lock = ENV_LOCK.lock().unwrap();

        // Skip if iTerm2 is not installed.
        if run_osascript(r#"id of application "iTerm2""#).is_none() {
            eprintln!("Skipping: iTerm2 is not installed");
            return;
        }

        let workspace = TempDir::new().unwrap();
        let workspace_path = workspace.path().to_string_lossy().replace('\'', "'\"'\"'");
        let launch_script = format!(
            r#"tell application "iTerm2"
activate
set w to (create window with default profile command "cd '{}'; exec $SHELL -l")
return id of w
end tell"#,
            workspace_path
        );
        let window_id = run_osascript(&launch_script).expect("failed to launch iTerm2 window");
        let _window_guard = ItermWindowGuard { window_id };

        // Give iTerm2 time to become frontmost and update session path.
        std::thread::sleep(std::time::Duration::from_millis(1500));

        let mut settings = crate::settings::get_default_settings();
        settings.recent_workspace_roots.clear();

        let resolved = get_workspace_root(&settings)
            .expect("expected workspace root to resolve from active iTerm2 session");
        assert_eq!(resolved, workspace.path().to_path_buf());
    }
}
