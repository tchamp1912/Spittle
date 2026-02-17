use serde::Serialize;

#[derive(Clone, Copy, Debug)]
pub enum ModelStateKind {
    LoadingStarted,
    LoadingFailed,
    Loaded,
    Unloaded,
}

impl ModelStateKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::LoadingStarted => "loading_started",
            Self::LoadingFailed => "loading_failed",
            Self::Loaded => "loaded",
            Self::Unloaded => "unloaded",
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct ModelStateEvent {
    pub event_type: String,
    pub model_id: Option<String>,
    pub model_name: Option<String>,
    pub error: Option<String>,
}

impl ModelStateEvent {
    pub fn new(
        kind: ModelStateKind,
        model_id: Option<String>,
        model_name: Option<String>,
        error: Option<String>,
    ) -> Self {
        Self {
            event_type: kind.as_str().to_string(),
            model_id,
            model_name,
            error,
        }
    }
}
