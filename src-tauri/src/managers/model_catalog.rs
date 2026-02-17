use std::collections::HashMap;

use anyhow::{anyhow, Context, Result};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CatalogEngineType {
    Whisper,
    Parakeet,
    Moonshine,
    SenseVoice,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CatalogModel {
    pub id: String,
    pub name: String,
    pub description: String,
    pub filename: String,
    pub url: Option<String>,
    pub size_mb: u64,
    pub is_directory: bool,
    pub engine_type: CatalogEngineType,
    pub accuracy_score: f32,
    pub speed_score: f32,
    pub supports_translation: bool,
    pub is_recommended: bool,
    pub language_group: String,
}

#[derive(Debug, Clone)]
pub struct ResolvedCatalogModel {
    pub id: String,
    pub name: String,
    pub description: String,
    pub filename: String,
    pub url: Option<String>,
    pub size_mb: u64,
    pub is_directory: bool,
    pub engine_type: CatalogEngineType,
    pub accuracy_score: f32,
    pub speed_score: f32,
    pub supports_translation: bool,
    pub is_recommended: bool,
    pub supported_languages: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ModelCatalogFile {
    language_groups: HashMap<String, Vec<String>>,
    models: Vec<CatalogModel>,
}

pub fn load_model_catalog() -> Result<Vec<ResolvedCatalogModel>> {
    let raw = include_str!("../../resources/model_catalog.json");
    let parsed: ModelCatalogFile = serde_json::from_str(raw).context("parse model catalog JSON")?;

    let mut resolved = Vec::with_capacity(parsed.models.len());
    for model in parsed.models {
        let supported_languages =
            if let Some(group) = parsed.language_groups.get(&model.language_group) {
                group.clone()
            } else {
                return Err(anyhow!(
                    "model '{}' references unknown language group '{}'",
                    model.id,
                    model.language_group
                ));
            };

        resolved.push(ResolvedCatalogModel {
            id: model.id,
            name: model.name,
            description: model.description,
            filename: model.filename,
            url: model.url,
            size_mb: model.size_mb,
            is_directory: model.is_directory,
            engine_type: model.engine_type,
            accuracy_score: model.accuracy_score,
            speed_score: model.speed_score,
            supports_translation: model.supports_translation,
            is_recommended: model.is_recommended,
            supported_languages,
        });
    }

    Ok(resolved)
}
