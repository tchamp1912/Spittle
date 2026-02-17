use log::{debug, warn};
use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use specta::Type;
use std::collections::HashMap;
use tauri::AppHandle;
use tauri_plugin_store::StoreExt;

pub const APPLE_INTELLIGENCE_PROVIDER_ID: &str = "apple_intelligence";
pub const APPLE_INTELLIGENCE_DEFAULT_MODEL_ID: &str = "Apple Intelligence";

#[derive(Serialize, Debug, Clone, Copy, PartialEq, Eq, Type)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

// Custom deserializer to handle both old numeric format (1-5) and new string format ("trace", "debug", etc.)
impl<'de> Deserialize<'de> for LogLevel {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct LogLevelVisitor;

        impl<'de> Visitor<'de> for LogLevelVisitor {
            type Value = LogLevel;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a string or integer representing log level")
            }

            fn visit_str<E: de::Error>(self, value: &str) -> Result<LogLevel, E> {
                match value.to_lowercase().as_str() {
                    "trace" => Ok(LogLevel::Trace),
                    "debug" => Ok(LogLevel::Debug),
                    "info" => Ok(LogLevel::Info),
                    "warn" => Ok(LogLevel::Warn),
                    "error" => Ok(LogLevel::Error),
                    _ => Err(E::unknown_variant(
                        value,
                        &["trace", "debug", "info", "warn", "error"],
                    )),
                }
            }

            fn visit_u64<E: de::Error>(self, value: u64) -> Result<LogLevel, E> {
                match value {
                    1 => Ok(LogLevel::Trace),
                    2 => Ok(LogLevel::Debug),
                    3 => Ok(LogLevel::Info),
                    4 => Ok(LogLevel::Warn),
                    5 => Ok(LogLevel::Error),
                    _ => Err(E::invalid_value(de::Unexpected::Unsigned(value), &"1-5")),
                }
            }
        }

        deserializer.deserialize_any(LogLevelVisitor)
    }
}

impl From<LogLevel> for tauri_plugin_log::LogLevel {
    fn from(level: LogLevel) -> Self {
        match level {
            LogLevel::Trace => tauri_plugin_log::LogLevel::Trace,
            LogLevel::Debug => tauri_plugin_log::LogLevel::Debug,
            LogLevel::Info => tauri_plugin_log::LogLevel::Info,
            LogLevel::Warn => tauri_plugin_log::LogLevel::Warn,
            LogLevel::Error => tauri_plugin_log::LogLevel::Error,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Type)]
pub struct ShortcutBinding {
    pub id: String,
    pub name: String,
    pub description: String,
    pub default_binding: String,
    pub current_binding: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Type)]
pub struct LLMPrompt {
    pub id: String,
    pub name: String,
    pub prompt: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Type)]
pub struct JargonPack {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub terms: Vec<String>,
    #[serde(default)]
    pub corrections: Vec<crate::jargon::JargonCorrection>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Type)]
pub struct PostProcessProvider {
    pub id: String,
    pub label: String,
    pub base_url: String,
    #[serde(default)]
    pub allow_base_url_edit: bool,
    #[serde(default)]
    pub models_endpoint: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Type)]
#[serde(rename_all = "lowercase")]
pub enum OverlayPosition {
    None,
    Top,
    Bottom,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Type)]
#[serde(rename_all = "snake_case")]
pub enum ModelUnloadTimeout {
    Never,
    Immediately,
    Min2,
    Min5,
    Min10,
    Min15,
    Hour1,
    Sec5, // Debug mode only
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Type)]
#[serde(rename_all = "snake_case")]
pub enum PasteMethod {
    CtrlV,
    Direct,
    None,
    ShiftInsert,
    CtrlShiftV,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Type)]
#[serde(rename_all = "snake_case")]
pub enum ClipboardHandling {
    DontModify,
    CopyToClipboard,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Type)]
#[serde(rename_all = "snake_case")]
pub enum AutoSubmitKey {
    Enter,
    CtrlEnter,
    CmdEnter,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Type)]
#[serde(rename_all = "snake_case")]
pub enum RecordingRetentionPeriod {
    Never,
    PreserveLimit,
    Days3,
    Weeks2,
    Months3,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Type)]
#[serde(rename_all = "snake_case")]
pub enum KeyboardImplementation {
    Tauri,
    HandyKeys,
}

impl Default for KeyboardImplementation {
    fn default() -> Self {
        // Default to HandyKeys only on macOS where it's well-tested.
        // Windows and Linux use Tauri by default (handy-keys not sufficiently tested yet).
        #[cfg(target_os = "macos")]
        return KeyboardImplementation::HandyKeys;
        #[cfg(not(target_os = "macos"))]
        return KeyboardImplementation::Tauri;
    }
}

impl Default for ModelUnloadTimeout {
    fn default() -> Self {
        ModelUnloadTimeout::Never
    }
}

impl Default for PasteMethod {
    fn default() -> Self {
        // Default to CtrlV for macOS and Windows, Direct for Linux
        #[cfg(target_os = "linux")]
        return PasteMethod::Direct;
        #[cfg(not(target_os = "linux"))]
        return PasteMethod::CtrlV;
    }
}

impl Default for ClipboardHandling {
    fn default() -> Self {
        ClipboardHandling::DontModify
    }
}

impl Default for AutoSubmitKey {
    fn default() -> Self {
        AutoSubmitKey::Enter
    }
}

impl ModelUnloadTimeout {
    pub fn to_minutes(self) -> Option<u64> {
        match self {
            ModelUnloadTimeout::Never => None,
            ModelUnloadTimeout::Immediately => Some(0), // Special case for immediate unloading
            ModelUnloadTimeout::Min2 => Some(2),
            ModelUnloadTimeout::Min5 => Some(5),
            ModelUnloadTimeout::Min10 => Some(10),
            ModelUnloadTimeout::Min15 => Some(15),
            ModelUnloadTimeout::Hour1 => Some(60),
            ModelUnloadTimeout::Sec5 => Some(0), // Special case for debug - handled separately
        }
    }

    pub fn to_seconds(self) -> Option<u64> {
        match self {
            ModelUnloadTimeout::Never => None,
            ModelUnloadTimeout::Immediately => Some(0), // Special case for immediate unloading
            ModelUnloadTimeout::Sec5 => Some(5),
            _ => self.to_minutes().map(|m| m * 60),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Type)]
#[serde(rename_all = "snake_case")]
pub enum SoundTheme {
    Marimba,
    Pop,
    Custom,
}

impl SoundTheme {
    fn as_str(&self) -> &'static str {
        match self {
            SoundTheme::Marimba => "marimba",
            SoundTheme::Pop => "pop",
            SoundTheme::Custom => "custom",
        }
    }

    pub fn to_start_path(&self) -> String {
        format!("resources/{}_start.wav", self.as_str())
    }

    pub fn to_stop_path(&self) -> String {
        format!("resources/{}_stop.wav", self.as_str())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Type)]
#[serde(rename_all = "snake_case")]
pub enum TypingTool {
    Auto,
    Wtype,
    Kwtype,
    Dotool,
    Ydotool,
    Xdotool,
}

impl Default for TypingTool {
    fn default() -> Self {
        TypingTool::Auto
    }
}

/* still spittle for composing the initial JSON in the store ------------- */
#[derive(Serialize, Deserialize, Debug, Clone, Type)]
pub struct AppSettings {
    pub bindings: HashMap<String, ShortcutBinding>,
    pub push_to_talk: bool,
    pub audio_feedback: bool,
    #[serde(default = "default_audio_feedback_volume")]
    pub audio_feedback_volume: f32,
    #[serde(default = "default_sound_theme")]
    pub sound_theme: SoundTheme,
    #[serde(default = "default_start_hidden")]
    pub start_hidden: bool,
    #[serde(default = "default_autostart_enabled")]
    pub autostart_enabled: bool,
    #[serde(default = "default_update_checks_enabled")]
    pub update_checks_enabled: bool,
    #[serde(default = "default_model")]
    pub selected_model: String,
    #[serde(default = "default_always_on_microphone")]
    pub always_on_microphone: bool,
    #[serde(default)]
    pub selected_microphone: Option<String>,
    #[serde(default)]
    pub clamshell_microphone: Option<String>,
    #[serde(default)]
    pub selected_output_device: Option<String>,
    #[serde(default = "default_translate_to_english")]
    pub translate_to_english: bool,
    #[serde(default = "default_selected_language")]
    pub selected_language: String,
    #[serde(default = "default_overlay_position")]
    pub overlay_position: OverlayPosition,
    #[serde(default = "default_debug_mode")]
    pub debug_mode: bool,
    #[serde(default = "default_log_level")]
    pub log_level: LogLevel,
    #[serde(default)]
    pub custom_words: Vec<String>,
    #[serde(default)]
    pub model_unload_timeout: ModelUnloadTimeout,
    #[serde(default = "default_word_correction_threshold")]
    pub word_correction_threshold: f64,
    #[serde(default = "default_history_limit")]
    pub history_limit: usize,
    #[serde(default = "default_recording_retention_period")]
    pub recording_retention_period: RecordingRetentionPeriod,
    #[serde(default)]
    pub paste_method: PasteMethod,
    #[serde(default)]
    pub clipboard_handling: ClipboardHandling,
    #[serde(default = "default_auto_submit")]
    pub auto_submit: bool,
    #[serde(default)]
    pub auto_submit_key: AutoSubmitKey,
    #[serde(default = "default_post_process_enabled")]
    pub post_process_enabled: bool,
    #[serde(default = "default_post_process_auto_prompt_selection")]
    pub post_process_auto_prompt_selection: bool,
    #[serde(default = "default_post_process_provider_id")]
    pub post_process_provider_id: String,
    #[serde(default = "default_post_process_providers")]
    pub post_process_providers: Vec<PostProcessProvider>,
    #[serde(default = "default_post_process_api_keys")]
    pub post_process_api_keys: HashMap<String, String>,
    #[serde(default = "default_post_process_models")]
    pub post_process_models: HashMap<String, String>,
    #[serde(default = "default_post_process_prompts")]
    pub post_process_prompts: Vec<LLMPrompt>,
    #[serde(default)]
    pub post_process_selected_prompt_id: Option<String>,
    #[serde(default)]
    pub mute_while_recording: bool,
    #[serde(default = "default_audio_segment_size_seconds")]
    pub audio_segment_size_seconds: f64,
    #[serde(default)]
    pub append_trailing_space: bool,
    #[serde(default = "default_app_language")]
    pub app_language: String,
    #[serde(default)]
    pub experimental_enabled: bool,
    #[serde(default)]
    pub keyboard_implementation: KeyboardImplementation,
    #[serde(default = "default_show_tray_icon")]
    pub show_tray_icon: bool,
    #[serde(default = "default_paste_delay_ms")]
    pub paste_delay_ms: u64,
    #[serde(default = "default_typing_tool")]
    pub typing_tool: TypingTool,
    #[serde(default)]
    pub at_file_expansion_enabled: bool,
    #[serde(default)]
    pub recent_workspace_roots: Vec<String>,
    #[serde(default)]
    pub jargon_enabled_profiles: Vec<String>,
    #[serde(default)]
    pub jargon_custom_terms: Vec<String>,
    #[serde(default)]
    pub jargon_custom_corrections: Vec<crate::jargon::JargonCorrection>,
    #[serde(default = "default_domain_selector_enabled")]
    pub domain_selector_enabled: bool,
    #[serde(default = "default_domain_selector_timeout_ms")]
    pub domain_selector_timeout_ms: u64,
    #[serde(default = "default_domain_selector_top_k")]
    pub domain_selector_top_k: usize,
    #[serde(default = "default_domain_selector_min_score")]
    pub domain_selector_min_score: f32,
    #[serde(default = "default_domain_selector_hysteresis")]
    pub domain_selector_hysteresis: f32,
    #[serde(default = "default_domain_selector_blend_manual_profiles")]
    pub domain_selector_blend_manual_profiles: bool,
    #[serde(default)]
    pub jargon_packs: Vec<JargonPack>,
}

fn default_model() -> String {
    "".to_string()
}

fn default_always_on_microphone() -> bool {
    false
}

fn default_translate_to_english() -> bool {
    false
}

fn default_audio_segment_size_seconds() -> f64 {
    0.0
}

fn default_start_hidden() -> bool {
    false
}

fn default_autostart_enabled() -> bool {
    false
}

fn default_update_checks_enabled() -> bool {
    true
}

fn default_selected_language() -> String {
    "auto".to_string()
}

fn default_overlay_position() -> OverlayPosition {
    #[cfg(target_os = "linux")]
    return OverlayPosition::None;
    #[cfg(not(target_os = "linux"))]
    return OverlayPosition::Bottom;
}

fn default_debug_mode() -> bool {
    false
}

fn default_log_level() -> LogLevel {
    LogLevel::Debug
}

fn default_word_correction_threshold() -> f64 {
    0.18
}

fn default_paste_delay_ms() -> u64 {
    60
}

fn default_auto_submit() -> bool {
    false
}

fn default_history_limit() -> usize {
    5
}

fn default_recording_retention_period() -> RecordingRetentionPeriod {
    RecordingRetentionPeriod::PreserveLimit
}

fn default_audio_feedback_volume() -> f32 {
    1.0
}

fn default_sound_theme() -> SoundTheme {
    SoundTheme::Marimba
}

fn default_post_process_enabled() -> bool {
    false
}

fn default_post_process_auto_prompt_selection() -> bool {
    false
}

fn default_app_language() -> String {
    tauri_plugin_os::locale()
        .map(|l| l.replace('_', "-"))
        .unwrap_or_else(|| "en".to_string())
}

fn default_show_tray_icon() -> bool {
    true
}

fn default_post_process_provider_id() -> String {
    "openai".to_string()
}

fn default_post_process_providers() -> Vec<PostProcessProvider> {
    let mut providers = vec![
        PostProcessProvider {
            id: "openai".to_string(),
            label: "OpenAI".to_string(),
            base_url: "https://api.openai.com/v1".to_string(),
            allow_base_url_edit: false,
            models_endpoint: Some("/models".to_string()),
        },
        PostProcessProvider {
            id: "openrouter".to_string(),
            label: "OpenRouter".to_string(),
            base_url: "https://openrouter.ai/api/v1".to_string(),
            allow_base_url_edit: false,
            models_endpoint: Some("/models".to_string()),
        },
        PostProcessProvider {
            id: "anthropic".to_string(),
            label: "Anthropic".to_string(),
            base_url: "https://api.anthropic.com/v1".to_string(),
            allow_base_url_edit: false,
            models_endpoint: Some("/models".to_string()),
        },
        PostProcessProvider {
            id: "groq".to_string(),
            label: "Groq".to_string(),
            base_url: "https://api.groq.com/openai/v1".to_string(),
            allow_base_url_edit: false,
            models_endpoint: Some("/models".to_string()),
        },
        PostProcessProvider {
            id: "cerebras".to_string(),
            label: "Cerebras".to_string(),
            base_url: "https://api.cerebras.ai/v1".to_string(),
            allow_base_url_edit: false,
            models_endpoint: Some("/models".to_string()),
        },
    ];

    // Note: We always include Apple Intelligence on macOS ARM64 without checking availability
    // at startup. The availability check is deferred to when the user actually tries to use it
    // (in actions.rs). This prevents crashes on macOS 26.x beta where accessing
    // SystemLanguageModel.default during early app initialization causes SIGABRT.
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        providers.push(PostProcessProvider {
            id: APPLE_INTELLIGENCE_PROVIDER_ID.to_string(),
            label: "Apple Intelligence".to_string(),
            base_url: "apple-intelligence://local".to_string(),
            allow_base_url_edit: false,
            models_endpoint: None,
        });
    }

    // Custom provider always comes last
    providers.push(PostProcessProvider {
        id: "custom".to_string(),
        label: "Custom".to_string(),
        base_url: "http://localhost:11434/v1".to_string(),
        allow_base_url_edit: true,
        models_endpoint: Some("/models".to_string()),
    });

    providers
}

fn default_post_process_api_keys() -> HashMap<String, String> {
    let mut map = HashMap::new();
    for provider in default_post_process_providers() {
        map.insert(provider.id, String::new());
    }
    map
}

fn default_model_for_provider(provider_id: &str) -> String {
    if provider_id == APPLE_INTELLIGENCE_PROVIDER_ID {
        return APPLE_INTELLIGENCE_DEFAULT_MODEL_ID.to_string();
    }
    String::new()
}

fn default_post_process_models() -> HashMap<String, String> {
    let mut map = HashMap::new();
    for provider in default_post_process_providers() {
        map.insert(
            provider.id.clone(),
            default_model_for_provider(&provider.id),
        );
    }
    map
}

fn builtin_post_process_prompts() -> Vec<LLMPrompt> {
    vec![
        LLMPrompt {
            id: "default_improve_transcriptions".to_string(),
            name: "Improve Transcriptions".to_string(),
            prompt: "Clean this transcript for readability while preserving meaning:\n1. Fix spelling, capitalization, punctuation, and spacing\n2. Convert spoken number words to digits when clear\n3. Remove obvious filler words and false starts only when confidence is high\n4. Keep technical terms and identifiers exact\n\nReturn only the cleaned transcript text.\n\nTranscript:\n${output}".to_string(),
        },
        LLMPrompt {
            id: "default_coding_assistant".to_string(),
            name: "Coding Assistant".to_string(),
            prompt: "Rewrite this transcript into an engineering update.\n\nOutput format:\n## Summary\n- 2-4 factual bullets\n## Tasks\n- [ ] Task\n## Notes\n- Optional short bullets\n\nTranscript:\n${output}".to_string(),
        },
        LLMPrompt {
            id: "default_slack_message".to_string(),
            name: "Slack Message".to_string(),
            prompt: "Convert this transcript into a concise Slack update.\n1. Keep it direct, friendly, and skimmable\n2. Preserve decisions, blockers, owners, and dates exactly\n3. Keep to 80-140 words unless source is shorter\n\nReturn only the final Slack message body.\n\nTranscript:\n${output}".to_string(),
        },
        LLMPrompt {
            id: "default_email_draft".to_string(),
            name: "Email Draft".to_string(),
            prompt: "Transform this transcript into a professional email draft.\n\nOutput format:\nSubject: <clear subject>\n<body paragraphs>\n\nTranscript:\n${output}".to_string(),
        },
        LLMPrompt {
            id: "default_document_writer".to_string(),
            name: "Document Writer".to_string(),
            prompt: "Turn this transcript into a structured document draft.\n\nOutput format:\n# Title\n## Context\n## Details\n## Decisions\n## Next Steps\n\nTranscript:\n${output}".to_string(),
        },
        LLMPrompt {
            id: "default_meeting_notes".to_string(),
            name: "Meeting Notes".to_string(),
            prompt: "Convert this transcript into clean meeting notes.\n\nOutput format:\n## Summary\n- Bullet points\n## Decisions\n- Bullet points\n## Open Questions\n- Bullet points\n## Action Items\n- [ ] Owner - Task (Due: date or TBA)\n\nTranscript:\n${output}".to_string(),
        },
        LLMPrompt {
            id: "default_action_items".to_string(),
            name: "Action Items".to_string(),
            prompt: "Extract only actionable tasks from this transcript.\n\nOutput format:\n- [ ] Owner - Task (Due: date or TBA)\n\nRules:\n- Use \"Unassigned\" when owner is unknown\n- Do not include non-actionable commentary\n\nTranscript:\n${output}".to_string(),
        },
        LLMPrompt {
            id: "default_standup_update".to_string(),
            name: "Standup Update".to_string(),
            prompt: "Rewrite this transcript into a daily standup update.\n\nOutput format:\nYesterday:\n- Bullet points\nToday:\n- Bullet points\nBlockers:\n- Bullet points or \"None\"\n\nRules:\n- Max 3 bullets per section\n- Keep under 120 words when possible\n- Do not add details not present in source\n\nTranscript:\n${output}".to_string(),
        },
        LLMPrompt {
            id: "default_pr_description".to_string(),
            name: "PR Description".to_string(),
            prompt: "Turn this transcript into a pull request description.\n\nOutput format:\n## Summary\n## Changes\n## Testing\n## Reviewer Checklist\n\nTranscript:\n${output}".to_string(),
        },
        LLMPrompt {
            id: "default_ticket_writer".to_string(),
            name: "Jira Ticket".to_string(),
            prompt: "Convert this transcript into a clear engineering ticket.\n\nOutput format:\nTitle: <specific title>\nDescription:\nAcceptance Criteria:\n- ...\n\nTranscript:\n${output}".to_string(),
        },
        LLMPrompt {
            id: "default_commit_message".to_string(),
            name: "Commit Message".to_string(),
            prompt: "Create a conventional commit message from this transcript.\n\nRules:\n- Use one type: feat, fix, chore, refactor, docs, test\n- Keep subject <= 72 chars\n- Add body only if needed\n\nReturn only the commit message.\n\nTranscript:\n${output}".to_string(),
        },
        LLMPrompt {
            id: "default_release_notes".to_string(),
            name: "Release Notes".to_string(),
            prompt: "Rewrite this transcript into end-user release notes.\n\nOutput format:\n## Added\n## Improved\n## Fixed\n\nTranscript:\n${output}".to_string(),
        },
        LLMPrompt {
            id: "default_customer_support_reply".to_string(),
            name: "Support Reply".to_string(),
            prompt: "Turn this transcript into a customer support response.\n\nRules:\n- Keep tone empathetic and concise\n- Clearly state next steps\n- Ask only necessary follow-up questions\n\nReturn only the final reply.\n\nTranscript:\n${output}".to_string(),
        },
        LLMPrompt {
            id: "default_brain_dump_to_outline".to_string(),
            name: "Brain Dump to Outline".to_string(),
            prompt: "Organize this transcript into a structured outline.\n\nOutput format:\n# Main Topic\n## Section\n- Bullet\n\nTranscript:\n${output}".to_string(),
        },
    ]
}

fn default_post_process_prompts() -> Vec<LLMPrompt> {
    builtin_post_process_prompts()
}

fn default_typing_tool() -> TypingTool {
    TypingTool::Auto
}

fn default_domain_selector_enabled() -> bool {
    false
}

fn default_domain_selector_timeout_ms() -> u64 {
    120
}

fn default_domain_selector_top_k() -> usize {
    2
}

fn default_domain_selector_min_score() -> f32 {
    0.1
}

fn default_domain_selector_hysteresis() -> f32 {
    0.08
}

fn default_domain_selector_blend_manual_profiles() -> bool {
    true
}

fn ensure_post_process_defaults(settings: &mut AppSettings) -> bool {
    let mut changed = false;
    for provider in default_post_process_providers() {
        if settings
            .post_process_providers
            .iter()
            .all(|existing| existing.id != provider.id)
        {
            settings.post_process_providers.push(provider.clone());
            changed = true;
        }

        if !settings.post_process_api_keys.contains_key(&provider.id) {
            settings
                .post_process_api_keys
                .insert(provider.id.clone(), String::new());
            changed = true;
        }

        let default_model = default_model_for_provider(&provider.id);
        match settings.post_process_models.get_mut(&provider.id) {
            Some(existing) => {
                if existing.is_empty() && !default_model.is_empty() {
                    *existing = default_model.clone();
                    changed = true;
                }
            }
            None => {
                settings
                    .post_process_models
                    .insert(provider.id.clone(), default_model);
                changed = true;
            }
        }
    }

    for prompt in builtin_post_process_prompts() {
        if settings
            .post_process_prompts
            .iter()
            .all(|existing| existing.id != prompt.id)
        {
            settings.post_process_prompts.push(prompt);
            changed = true;
        }
    }

    changed
}

fn ensure_jargon_pack_defaults(settings: &mut AppSettings) -> bool {
    let mut changed = false;
    let mut seen_ids = std::collections::HashSet::new();
    let mut filtered = Vec::new();

    for pack in &settings.jargon_packs {
        let id = pack.id.trim();
        let label = pack.label.trim();
        if id.is_empty() || label.is_empty() || !seen_ids.insert(id.to_string()) {
            changed = true;
            continue;
        }
        filtered.push(JargonPack {
            id: id.to_string(),
            label: label.to_string(),
            terms: pack
                .terms
                .iter()
                .map(|term| term.trim().to_string())
                .filter(|term| !term.is_empty())
                .collect(),
            corrections: pack
                .corrections
                .iter()
                .filter(|item| !item.from.trim().is_empty() && !item.to.trim().is_empty())
                .cloned()
                .collect(),
        });
    }

    if filtered.len() != settings.jargon_packs.len() {
        changed = true;
    }
    settings.jargon_packs = filtered;

    let clamped_top_k = settings.domain_selector_top_k.clamp(1, 5);
    if settings.domain_selector_top_k != clamped_top_k {
        settings.domain_selector_top_k = clamped_top_k;
        changed = true;
    }

    let clamped_timeout = settings.domain_selector_timeout_ms.clamp(25, 2000);
    if settings.domain_selector_timeout_ms != clamped_timeout {
        settings.domain_selector_timeout_ms = clamped_timeout;
        changed = true;
    }

    let clamped_min_score = settings.domain_selector_min_score.clamp(0.0, 1.0);
    if (settings.domain_selector_min_score - clamped_min_score).abs() > f32::EPSILON {
        settings.domain_selector_min_score = clamped_min_score;
        changed = true;
    }

    let clamped_hysteresis = settings.domain_selector_hysteresis.clamp(0.0, 1.0);
    if (settings.domain_selector_hysteresis - clamped_hysteresis).abs() > f32::EPSILON {
        settings.domain_selector_hysteresis = clamped_hysteresis;
        changed = true;
    }

    changed
}

pub const SETTINGS_STORE_PATH: &str = "settings_store.json";
const SETTINGS_SCHEMA_VERSION: u32 = 1;

#[derive(Serialize, Deserialize, Debug, Clone)]
struct PersistedSettings {
    schema_version: u32,
    settings: AppSettings,
}

fn decode_settings(value: serde_json::Value) -> Result<(AppSettings, bool), serde_json::Error> {
    if let Ok(persisted) = serde_json::from_value::<PersistedSettings>(value.clone()) {
        return Ok((persisted.settings, true));
    }
    serde_json::from_value::<AppSettings>(value).map(|settings| (settings, false))
}

fn encode_settings(settings: &AppSettings) -> serde_json::Value {
    serde_json::to_value(PersistedSettings {
        schema_version: SETTINGS_SCHEMA_VERSION,
        settings: settings.clone(),
    })
    .unwrap_or_else(|_| serde_json::json!({}))
}

fn merge_default_bindings(settings: &mut AppSettings) -> bool {
    let default_settings = get_default_settings();
    let mut updated = false;
    for (key, value) in default_settings.bindings {
        if !settings.bindings.contains_key(&key) {
            debug!("Adding missing binding: {}", key);
            settings.bindings.insert(key, value);
            updated = true;
        }
    }
    updated
}

fn migrate_settings(settings: &mut AppSettings) -> bool {
    let mut changed = false;
    if merge_default_bindings(settings) {
        changed = true;
    }
    if ensure_post_process_defaults(settings) {
        changed = true;
    }
    if ensure_jargon_pack_defaults(settings) {
        changed = true;
    }
    changed
}

pub fn get_default_settings() -> AppSettings {
    #[cfg(target_os = "windows")]
    let default_shortcut = "ctrl+space";
    #[cfg(target_os = "macos")]
    let default_shortcut = "option+space";
    #[cfg(target_os = "linux")]
    let default_shortcut = "ctrl+space";
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    let default_shortcut = "alt+space";

    let mut bindings = HashMap::new();
    bindings.insert(
        "transcribe".to_string(),
        ShortcutBinding {
            id: "transcribe".to_string(),
            name: "Transcribe".to_string(),
            description: "Converts your speech into text.".to_string(),
            default_binding: default_shortcut.to_string(),
            current_binding: default_shortcut.to_string(),
        },
    );
    #[cfg(target_os = "windows")]
    let default_post_process_shortcut = "ctrl+shift+space";
    #[cfg(target_os = "macos")]
    let default_post_process_shortcut = "option+shift+space";
    #[cfg(target_os = "linux")]
    let default_post_process_shortcut = "ctrl+shift+space";
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    let default_post_process_shortcut = "alt+shift+space";

    bindings.insert(
        "transcribe_with_post_process".to_string(),
        ShortcutBinding {
            id: "transcribe_with_post_process".to_string(),
            name: "Transcribe with Post-Processing".to_string(),
            description: "Converts your speech into text and applies AI post-processing."
                .to_string(),
            default_binding: default_post_process_shortcut.to_string(),
            current_binding: default_post_process_shortcut.to_string(),
        },
    );
    bindings.insert(
        "cancel".to_string(),
        ShortcutBinding {
            id: "cancel".to_string(),
            name: "Cancel".to_string(),
            description: "Cancels the current recording.".to_string(),
            default_binding: "escape".to_string(),
            current_binding: "escape".to_string(),
        },
    );

    AppSettings {
        bindings,
        push_to_talk: true,
        audio_feedback: false,
        audio_feedback_volume: default_audio_feedback_volume(),
        sound_theme: default_sound_theme(),
        start_hidden: default_start_hidden(),
        autostart_enabled: default_autostart_enabled(),
        update_checks_enabled: default_update_checks_enabled(),
        selected_model: "".to_string(),
        always_on_microphone: false,
        selected_microphone: None,
        clamshell_microphone: None,
        selected_output_device: None,
        translate_to_english: false,
        selected_language: "auto".to_string(),
        overlay_position: default_overlay_position(),
        debug_mode: false,
        log_level: default_log_level(),
        custom_words: Vec::new(),
        model_unload_timeout: ModelUnloadTimeout::Never,
        word_correction_threshold: default_word_correction_threshold(),
        history_limit: default_history_limit(),
        recording_retention_period: default_recording_retention_period(),
        paste_method: PasteMethod::default(),
        clipboard_handling: ClipboardHandling::default(),
        auto_submit: default_auto_submit(),
        auto_submit_key: AutoSubmitKey::default(),
        post_process_enabled: default_post_process_enabled(),
        post_process_auto_prompt_selection: default_post_process_auto_prompt_selection(),
        post_process_provider_id: default_post_process_provider_id(),
        post_process_providers: default_post_process_providers(),
        post_process_api_keys: default_post_process_api_keys(),
        post_process_models: default_post_process_models(),
        post_process_prompts: default_post_process_prompts(),
        post_process_selected_prompt_id: None,
        mute_while_recording: false,
        audio_segment_size_seconds: default_audio_segment_size_seconds(),
        append_trailing_space: false,
        app_language: default_app_language(),
        experimental_enabled: false,
        keyboard_implementation: KeyboardImplementation::default(),
        show_tray_icon: default_show_tray_icon(),
        paste_delay_ms: default_paste_delay_ms(),
        typing_tool: default_typing_tool(),
        at_file_expansion_enabled: false,
        recent_workspace_roots: Vec::new(),
        jargon_enabled_profiles: Vec::new(),
        jargon_custom_terms: Vec::new(),
        jargon_custom_corrections: Vec::new(),
        domain_selector_enabled: default_domain_selector_enabled(),
        domain_selector_timeout_ms: default_domain_selector_timeout_ms(),
        domain_selector_top_k: default_domain_selector_top_k(),
        domain_selector_min_score: default_domain_selector_min_score(),
        domain_selector_hysteresis: default_domain_selector_hysteresis(),
        domain_selector_blend_manual_profiles: default_domain_selector_blend_manual_profiles(),
        jargon_packs: Vec::new(),
    }
}

impl AppSettings {
    pub fn active_post_process_provider(&self) -> Option<&PostProcessProvider> {
        self.post_process_providers
            .iter()
            .find(|provider| provider.id == self.post_process_provider_id)
    }

    pub fn post_process_provider(&self, provider_id: &str) -> Option<&PostProcessProvider> {
        self.post_process_providers
            .iter()
            .find(|provider| provider.id == provider_id)
    }

    pub fn post_process_provider_mut(
        &mut self,
        provider_id: &str,
    ) -> Option<&mut PostProcessProvider> {
        self.post_process_providers
            .iter_mut()
            .find(|provider| provider.id == provider_id)
    }
}

pub fn load_or_create_app_settings(app: &AppHandle) -> AppSettings {
    // Initialize store
    let store = app
        .store(SETTINGS_STORE_PATH)
        .expect("Failed to initialize store");

    let (mut settings, was_versioned) = if let Some(settings_value) = store.get("settings") {
        match decode_settings(settings_value) {
            Ok((settings, was_versioned)) => (settings, was_versioned),
            Err(e) => {
                warn!("Failed to parse settings: {}", e);
                (get_default_settings(), false)
            }
        }
    } else {
        (get_default_settings(), false)
    };

    let migrated = migrate_settings(&mut settings);
    if migrated || !was_versioned {
        store.set("settings", encode_settings(&settings));
    }
    settings
}

pub fn get_settings(app: &AppHandle) -> AppSettings {
    let store = app
        .store(SETTINGS_STORE_PATH)
        .expect("Failed to initialize store");

    let (mut settings, was_versioned) = if let Some(settings_value) = store.get("settings") {
        match decode_settings(settings_value) {
            Ok((settings, was_versioned)) => (settings, was_versioned),
            Err(_) => (get_default_settings(), false),
        }
    } else {
        (get_default_settings(), false)
    };

    let migrated = migrate_settings(&mut settings);
    if migrated || !was_versioned {
        store.set("settings", encode_settings(&settings));
    }

    settings
}

pub fn write_settings(app: &AppHandle, settings: AppSettings) {
    let store = app
        .store(SETTINGS_STORE_PATH)
        .expect("Failed to initialize store");

    store.set("settings", encode_settings(&settings));
}

pub fn get_bindings(app: &AppHandle) -> HashMap<String, ShortcutBinding> {
    let settings = get_settings(app);

    settings.bindings
}

pub fn get_stored_binding(app: &AppHandle, id: &str) -> ShortcutBinding {
    let bindings = get_bindings(app);

    let binding = bindings.get(id).unwrap().clone();

    binding
}

pub fn get_history_limit(app: &AppHandle) -> usize {
    let settings = get_settings(app);
    settings.history_limit
}

pub fn get_recording_retention_period(app: &AppHandle) -> RecordingRetentionPeriod {
    let settings = get_settings(app);
    settings.recording_retention_period
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_settings_disable_auto_submit() {
        let settings = get_default_settings();
        assert!(!settings.auto_submit);
        assert_eq!(settings.auto_submit_key, AutoSubmitKey::Enter);
    }

    #[test]
    fn decode_legacy_settings_payload() {
        let settings = get_default_settings();
        let value = serde_json::to_value(&settings).expect("serialize legacy settings");
        let (decoded, was_versioned) = decode_settings(value).expect("decode settings");
        assert!(!was_versioned);
        assert_eq!(decoded.selected_model, settings.selected_model);
    }

    #[test]
    fn decode_versioned_settings_payload() {
        let settings = get_default_settings();
        let value = encode_settings(&settings);
        let (decoded, was_versioned) = decode_settings(value).expect("decode settings");
        assert!(was_versioned);
        assert_eq!(decoded.bindings.len(), settings.bindings.len());
    }
}
