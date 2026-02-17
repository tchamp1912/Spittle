pub mod audio;
pub mod domain_selector;
pub mod history;
pub mod model;
pub mod model_catalog;
pub mod recording_pipeline;

#[cfg(not(feature = "mock_transcription"))]
pub mod transcription;
#[cfg(feature = "mock_transcription")]
#[path = "transcription_mock.rs"]
pub mod transcription;
