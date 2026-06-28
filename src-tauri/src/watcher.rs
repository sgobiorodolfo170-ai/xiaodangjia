use notify::{Watcher, RecursiveMode, Event};
use std::sync::mpsc::channel;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::path::Path;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

// File watcher event types for frontend
#[derive(Clone, serde::Serialize)]
pub struct FileChangeEvent {
    pub event_type: String,
    pub path: String,
    pub is_directory: bool,
}

/// Start file watcher for a project directory.
/// Accepts an external stop_flag so the caller can signal the thread to exit.
pub fn start_file_watcher(
    app_handle: AppHandle,
    _project_id: String,
    path: String,
    stop_flag: Arc<AtomicBool>,
) -> Result<std::thread::JoinHandle<()>, String> {
    let path_clone = path.clone();

    let handle = std::thread::spawn(move || {
        let (tx, rx) = channel();

        let mut watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
            if let Ok(event) = res {
                let _ = tx.send(event);
            }
        }).expect("Failed to create watcher");

        watcher.watch(Path::new(&path_clone), RecursiveMode::Recursive)
            .expect("Failed to start watching");

        log::info!("Started watching: {}", path_clone);

        loop {
            if stop_flag.load(Ordering::Relaxed) {
                log::info!("Stopping watcher for: {}", path_clone);
                break;
            }
            match rx.recv_timeout(Duration::from_secs(1)) {
                Ok(event) => {
                    if stop_flag.load(Ordering::Relaxed) {
                        break;
                    }
                    for path in event.paths {
                        let event_type = match event.kind {
                            notify::EventKind::Create(_) => "created",
                            notify::EventKind::Modify(_) => "modified",
                            notify::EventKind::Remove(_) => "deleted",
                            _ => continue,
                        };

                        let is_dir = path.is_dir();

                        let change_event = FileChangeEvent {
                            event_type: event_type.to_string(),
                            path: path.to_string_lossy().to_string(),
                            is_directory: is_dir,
                        };

                        let _ = app_handle.emit("file-change", &change_event);
                        log::info!("File change: {} - {}", event_type, path.display());
                    }
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
                Err(_) => break,
            }
        }

        drop(watcher);
        log::info!("Watcher stopped for: {}", path_clone);
    });

    Ok(handle)
}
