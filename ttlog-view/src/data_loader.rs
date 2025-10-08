use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tokio::task;

use crate::{logs::{Logs, LogsInfo, ResolvedLog}, snapshots::{Snapshots, SnapshotFile}};

#[derive(Debug, Clone)]
pub enum LoadingState<T> {
    NotStarted,
    Loading,
    Loaded(T),
    Error(String),
}

impl<T> LoadingState<T> {
    pub fn is_loading(&self) -> bool {
        matches!(self, LoadingState::Loading)
    }

    pub fn is_loaded(&self) -> bool {
        matches!(self, LoadingState::Loaded(_))
    }

    pub fn is_error(&self) -> bool {
        matches!(self, LoadingState::Error(_))
    }

    pub fn get_data(&self) -> Option<&T> {
        match self {
            LoadingState::Loaded(data) => Some(data),
            _ => None,
        }
    }

    pub fn get_error(&self) -> Option<&str> {
        match self {
            LoadingState::Error(err) => Some(err),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AppData {
    pub logs_info: LoadingState<LogsInfo>,
    pub log_events: LoadingState<Vec<ResolvedLog>>,
    pub snapshots: LoadingState<Vec<SnapshotFile>>,
}

impl Default for AppData {
    fn default() -> Self {
        Self {
            logs_info: LoadingState::NotStarted,
            log_events: LoadingState::NotStarted,
            snapshots: LoadingState::NotStarted,
        }
    }
}

pub enum DataMessage {
    LogsInfoLoaded(Result<LogsInfo, String>),
    LogEventsLoaded(Result<Vec<ResolvedLog>, String>),
    SnapshotsLoaded(Result<Vec<SnapshotFile>, String>),
}

pub struct DataLoader {
    pub data: Arc<Mutex<AppData>>,
    pub receiver: mpsc::UnboundedReceiver<DataMessage>,
    sender: mpsc::UnboundedSender<DataMessage>,
}

impl DataLoader {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        
        Self {
            data: Arc::new(Mutex::new(AppData::default())),
            receiver,
            sender,
        }
    }

    pub async fn start_loading(&self, dir_path: String, log_file_path: String) {
        let data = Arc::clone(&self.data);
        let sender = self.sender.clone();

        // Update states to loading
        {
            let mut data_guard = data.lock().await;
            data_guard.logs_info = LoadingState::Loading;
            data_guard.log_events = LoadingState::Loading;
            data_guard.snapshots = LoadingState::Loading;
        }

        // Load logs info
        let sender_clone = sender.clone();
        let dir_path_clone = dir_path.clone();
        task::spawn(async move {
            let result = task::spawn_blocking(move || {
                Logs::get_logs_info(&dir_path_clone)
                    .map_err(|e| e.to_string())
            }).await;

            match result {
                Ok(logs_info_result) => {
                    let _ = sender_clone.send(DataMessage::LogsInfoLoaded(logs_info_result));
                }
                Err(e) => {
                    let _ = sender_clone.send(DataMessage::LogsInfoLoaded(Err(e.to_string())));
                }
            }
        });

        // Load log events
        let sender_clone = sender.clone();
        let log_file_path_clone = log_file_path.clone();
        task::spawn(async move {
            let result = task::spawn_blocking(move || {
                Logs::get_logs(&log_file_path_clone)
                    .map_err(|e| e.to_string())
            }).await;

            match result {
                Ok(log_events_result) => {
                    let _ = sender_clone.send(DataMessage::LogEventsLoaded(log_events_result));
                }
                Err(e) => {
                    let _ = sender_clone.send(DataMessage::LogEventsLoaded(Err(e.to_string())));
                }
            }
        });

        // Load snapshots
        let sender_clone = sender.clone();
        task::spawn(async move {
            let result = task::spawn_blocking(move || {
                Snapshots::read_snapshots(&dir_path)
                    .map_err(|e| e.to_string())
            }).await;

            match result {
                Ok(snapshots_result) => {
                    let _ = sender_clone.send(DataMessage::SnapshotsLoaded(snapshots_result));
                }
                Err(e) => {
                    let _ = sender_clone.send(DataMessage::SnapshotsLoaded(Err(e.to_string())));
                }
            }
        });
    }

    pub async fn process_message(&self, message: DataMessage) {
        let mut data_guard = self.data.lock().await;
        
        match message {
            DataMessage::LogsInfoLoaded(result) => {
                data_guard.logs_info = match result {
                    Ok(info) => LoadingState::Loaded(info),
                    Err(err) => LoadingState::Error(err),
                };
            }
            DataMessage::LogEventsLoaded(result) => {
                data_guard.log_events = match result {
                    Ok(events) => LoadingState::Loaded(events),
                    Err(err) => LoadingState::Error(err),
                };
            }
            DataMessage::SnapshotsLoaded(result) => {
                data_guard.snapshots = match result {
                    Ok(snapshots) => LoadingState::Loaded(snapshots),
                    Err(err) => LoadingState::Error(err),
                };
            }
        }
    }

    pub async fn get_data(&self) -> AppData {
        self.data.lock().await.clone()
    }
}
