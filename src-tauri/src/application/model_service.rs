use std::sync::Arc;

use anyhow::{anyhow, Result};
use tauri::AppHandle;

use crate::managers::model::ModelManager;
use crate::managers::transcription::TranscriptionManager;
use crate::settings::{get_settings, write_settings};

pub struct ModelService<'a> {
    app_handle: &'a AppHandle,
    model_manager: &'a Arc<ModelManager>,
    transcription_manager: &'a Arc<TranscriptionManager>,
}

impl<'a> ModelService<'a> {
    pub fn new(
        app_handle: &'a AppHandle,
        model_manager: &'a Arc<ModelManager>,
        transcription_manager: &'a Arc<TranscriptionManager>,
    ) -> Self {
        Self {
            app_handle,
            model_manager,
            transcription_manager,
        }
    }

    pub fn set_active_model(&self, model_id: &str) -> Result<()> {
        let model_info = self
            .model_manager
            .get_model_info(model_id)
            .ok_or_else(|| anyhow!("Model not found: {}", model_id))?;

        if !model_info.is_downloaded {
            return Err(anyhow!("Model not downloaded: {}", model_id));
        }

        self.transcription_manager.load_model(model_id)?;

        let mut settings = get_settings(self.app_handle);
        settings.selected_model = model_id.to_string();
        write_settings(self.app_handle, settings);
        Ok(())
    }

    pub fn delete_model(&self, model_id: &str) -> Result<()> {
        let settings = get_settings(self.app_handle);
        if settings.selected_model == model_id {
            self.transcription_manager.unload_model()?;
            let mut settings = get_settings(self.app_handle);
            settings.selected_model = String::new();
            write_settings(self.app_handle, settings);
        }

        self.model_manager.delete_model(model_id)?;
        Ok(())
    }
}
