use std::sync::Arc;

use anyhow::{Context, Result};
use tauri::{AppHandle, Manager};

use crate::managers::audio::AudioRecordingManager;
use crate::managers::domain_selector::DomainSelectorManager;
use crate::managers::history::HistoryManager;
use crate::managers::model::ModelManager;
use crate::managers::transcription::TranscriptionManager;

#[derive(Clone)]
pub struct AppServices {
    pub recording_manager: Arc<AudioRecordingManager>,
    pub domain_selector_manager: Arc<DomainSelectorManager>,
    pub model_manager: Arc<ModelManager>,
    pub transcription_manager: Arc<TranscriptionManager>,
    pub history_manager: Arc<HistoryManager>,
}

impl AppServices {
    pub fn initialize(app_handle: &AppHandle) -> Result<Self> {
        let recording_manager = Arc::new(
            AudioRecordingManager::new(app_handle).context("initialize recording manager")?,
        );
        let model_manager =
            Arc::new(ModelManager::new(app_handle).context("initialize model manager")?);
        let domain_selector_manager = Arc::new(DomainSelectorManager::new());
        let transcription_manager = Arc::new(
            TranscriptionManager::new(app_handle, model_manager.clone())
                .context("initialize transcription manager")?,
        );
        let history_manager =
            Arc::new(HistoryManager::new(app_handle).context("initialize history manager")?);

        Ok(Self {
            recording_manager,
            domain_selector_manager,
            model_manager,
            transcription_manager,
            history_manager,
        })
    }

    pub fn register(self, app_handle: &AppHandle) {
        app_handle.manage(self.recording_manager);
        app_handle.manage(self.domain_selector_manager);
        app_handle.manage(self.model_manager);
        app_handle.manage(self.transcription_manager);
        app_handle.manage(self.history_manager);
    }
}
