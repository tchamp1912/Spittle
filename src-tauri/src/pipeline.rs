//! State-machine-driven transcription pipeline.
//!
//! Drives the entire flow from recording-stopped to text-finalized.
//! Each phase is an explicit enum variant; transitions are methods
//! that consume the current state and produce the next.

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
use crate::apple_intelligence;
use crate::managers::domain_selector::{DomainContext, DomainSelectorManager};
use crate::managers::history::HistoryManager;
use crate::managers::transcription::TranscriptionManager;
use crate::settings::{AppSettings, APPLE_INTELLIGENCE_PROVIDER_ID};
use crate::tray::{change_tray_icon, TrayIconState};
use crate::utils;
use crate::ManagedToggleState;
use ferrous_opencc::{config::BuiltinConfig, OpenCC};
use log::{debug, error, info};
use once_cell::sync::Lazy;
use regex::Regex;
// similar crate is available but we use a simpler prefix/suffix diff
use std::sync::Arc;
use std::time::Instant;
use tauri::{AppHandle, Manager};

static SPACE_BEFORE_PUNCT_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"\s+([,.;:!?])").unwrap());

fn normalize_segment_text_for_post_process(text: &str) -> String {
    let collapsed = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let trimmed = collapsed.trim();
    SPACE_BEFORE_PUNCT_RE.replace_all(trimmed, "$1").to_string()
}

fn should_insert_boundary_space(left: &str, right: &str) -> bool {
    if left.is_empty() || right.is_empty() {
        return false;
    }

    let left_last = left.chars().last().unwrap_or(' ');
    let right_first = right.chars().next().unwrap_or(' ');

    !left_last.is_whitespace()
        && !matches!(left_last, '(' | '[' | '{' | '"' | '\'')
        && !right_first.is_whitespace()
        && !matches!(
            right_first,
            '.' | ',' | ';' | ':' | '!' | '?' | ')' | ']' | '}'
        )
}

fn build_profiles_map(
    settings: &AppSettings,
) -> std::collections::HashMap<String, crate::jargon::JargonProfile> {
    let mut profiles = crate::jargon::builtin_profiles();
    for pack in &settings.jargon_packs {
        profiles.insert(
            pack.id.clone(),
            crate::jargon::JargonProfile {
                label: pack.label.clone(),
                terms: pack.terms.clone(),
                corrections: pack.corrections.clone(),
            },
        );
    }
    profiles
}

fn effective_profiles_for_text(app: &AppHandle, settings: &AppSettings, text: &str) -> Vec<String> {
    let mut profile_ids = settings.jargon_enabled_profiles.clone();
    if let Some(selector) = app.try_state::<Arc<DomainSelectorManager>>() {
        if let Some(auto_profiles) = selector.select_profiles_with_timeout(
            settings,
            &DomainContext {
                text: text.to_string(),
            },
        ) {
            if settings.domain_selector_blend_manual_profiles {
                for profile in auto_profiles {
                    if !profile_ids.iter().any(|id| id == &profile) {
                        profile_ids.push(profile);
                    }
                }
            } else {
                profile_ids = auto_profiles;
            }
        }
    }
    profile_ids
}

// ============================================================================
// Pipeline state enum
// ============================================================================

enum PipelineState {
    /// Recording just stopped. We have raw audio samples and possibly
    /// pre-pasted segment strings.
    Stopped {
        samples: Vec<f32>,
        pasted_segments: Vec<String>,
    },

    /// All raw text has been transcribed (and pasted if post-processing).
    RawTextVisible {
        raw_text: String,
        had_segments: bool,
        raw_text_pasted: bool,
    },

    /// Post-processing complete. We have the original and processed text.
    PostProcessed {
        raw_text: String,
        final_text: String,
        raw_text_pasted: bool,
    },

    /// Pipeline complete.
    Done,
}

// ============================================================================
// Pipeline struct & driver
// ============================================================================

pub struct TranscriptionPipeline {
    state: PipelineState,
    app: AppHandle,
    settings: AppSettings,
    post_process: bool,
    binding_id: String,
    /// Audio samples kept around for history saving.
    samples_for_history: Vec<f32>,
}

impl TranscriptionPipeline {
    pub fn new(
        samples: Vec<f32>,
        pasted_segments: Vec<String>,
        settings: AppSettings,
        post_process: bool,
        binding_id: String,
        app: AppHandle,
    ) -> Self {
        let samples_for_history = samples.clone();
        Self {
            state: PipelineState::Stopped {
                samples,
                pasted_segments,
            },
            app,
            settings,
            post_process,
            binding_id,
            samples_for_history,
        }
    }

    /// Run the pipeline to completion.
    pub async fn run(mut self) {
        loop {
            self.state = match self.state {
                PipelineState::Stopped { .. } => match self.transcribe_and_paste() {
                    Ok(next) => next,
                    Err(err) => {
                        error!("Pipeline error in transcribe_and_paste: {}", err);
                        self.cleanup();
                        return;
                    }
                },
                PipelineState::RawTextVisible { .. } => {
                    if self.post_process {
                        self.post_process_text().await
                    } else {
                        self.finalize()
                    }
                }
                PipelineState::PostProcessed { .. } => self.apply_diff_and_finalize(),
                PipelineState::Done => break,
            };
        }

        // Clear toggle state now that transcription is complete
        if let Ok(mut states) = self.app.state::<ManagedToggleState>().lock() {
            states.active_toggles.insert(self.binding_id.clone(), false);
        }
    }

    // ========================================================================
    // Transitions
    // ========================================================================

    /// Stopped → RawTextVisible | Done
    fn transcribe_and_paste(&mut self) -> Result<PipelineState, anyhow::Error> {
        let (samples, pasted_segments) =
            match std::mem::replace(&mut self.state, PipelineState::Done) {
                PipelineState::Stopped {
                    samples,
                    pasted_segments,
                } => (samples, pasted_segments),
                _ => unreachable!(),
            };

        let tm = self.app.state::<Arc<TranscriptionManager>>();
        let transcription_time = Instant::now();
        let remaining_transcription = tm.transcribe(samples)?;

        // Reconstruct full text from segments + remaining
        let transcription = if pasted_segments.is_empty() {
            remaining_transcription.clone()
        } else if remaining_transcription.is_empty() {
            pasted_segments.join("")
        } else {
            let joined = pasted_segments.join("");
            format!("{}{}", joined, remaining_transcription)
        };

        debug!(
            "Transcription completed in {:?}: '{}'",
            transcription_time.elapsed(),
            transcription
        );

        if transcription.is_empty() {
            return Ok(PipelineState::Done);
        }

        if self.post_process {
            let had_segments = !pasted_segments.is_empty();

            // Normalize spacing for the remaining transcription to match rolling
            // segment normalization (without changing casing/punctuation style).
            let cleaned_remaining = if had_segments && !remaining_transcription.is_empty() {
                normalize_segment_text_for_post_process(&remaining_transcription)
            } else {
                remaining_transcription.clone()
            };

            let joined = pasted_segments.join("");
            let needs_boundary_space = should_insert_boundary_space(&joined, &cleaned_remaining);

            // Reconstruct full text from normalized segments + normalized remaining.
            // This is what was actually pasted, so it must match for diffing.
            let raw_text = if had_segments {
                if cleaned_remaining.is_empty() {
                    joined.clone()
                } else {
                    if needs_boundary_space {
                        format!("{} {}", joined, cleaned_remaining)
                    } else {
                        format!("{}{}", joined, cleaned_remaining)
                    }
                }
            } else {
                transcription.clone()
            };

            // Single-write mode: do not paste raw text during post-process.
            // We paste exactly once after processing completes.

            utils::show_processing_overlay(&self.app);

            Ok(PipelineState::RawTextVisible {
                raw_text,
                had_segments,
                raw_text_pasted: false,
            })
        } else {
            // No post-processing — paste final text with trailing space / auto-submit
            if pasted_segments.is_empty() {
                // Simple case: single paste
                let ah = self.app.clone();
                let text = self.expand_at_refs_for_output(&transcription);
                let paste_time = Instant::now();
                self.app
                    .run_on_main_thread(move || {
                        match utils::paste(text, ah.clone()) {
                            Ok(()) => {
                                debug!("Text pasted successfully in {:?}", paste_time.elapsed())
                            }
                            Err(e) => error!("Failed to paste transcription: {}", e),
                        }
                        utils::hide_recording_overlay(&ah);
                        change_tray_icon(&ah, TrayIconState::Idle);
                    })
                    .unwrap_or_else(|e| {
                        error!("Failed to run paste on main thread: {:?}", e);
                        utils::hide_recording_overlay(&self.app);
                        change_tray_icon(&self.app, TrayIconState::Idle);
                    });
            } else {
                // Segments already pasted live with trailing space via paste().
                // Just paste the remaining if any, then finalize UI.
                if !remaining_transcription.is_empty() {
                    let ah = self.app.clone();
                    let text = self.expand_at_refs_for_output(&remaining_transcription);
                    self.app
                        .run_on_main_thread(move || {
                            if let Err(e) = utils::paste(text, ah) {
                                error!("Failed to paste remaining transcription: {}", e);
                            }
                        })
                        .unwrap_or_else(|e| {
                            error!("Failed to run paste on main thread: {:?}", e);
                        });
                }
                utils::hide_recording_overlay(&self.app);
                change_tray_icon(&self.app, TrayIconState::Idle);
            }

            Ok(PipelineState::Done)
        }
    }

    /// RawTextVisible → PostProcessed
    async fn post_process_text(&mut self) -> PipelineState {
        let (raw_text, had_segments, raw_text_pasted) =
            match std::mem::replace(&mut self.state, PipelineState::Done) {
                PipelineState::RawTextVisible {
                    raw_text,
                    had_segments,
                    raw_text_pasted,
                    ..
                } => (raw_text, had_segments, raw_text_pasted),
                _ => unreachable!(),
            };

        let mut final_text = raw_text.clone();
        let mut post_processed_text: Option<String> = None;
        let mut post_process_prompt: Option<String> = None;

        // Chinese variant conversion
        if let Some(converted) = maybe_convert_chinese_variant(&self.settings, &raw_text).await {
            final_text = converted;
        }

        // LLM post-processing
        info!(
            "Starting LLM post-processing on text ({} chars, had_segments={}): '{}'",
            final_text.len(),
            had_segments,
            &final_text[..final_text.len().min(100)]
        );
        let processed =
            post_process_transcription(&self.app, &self.settings, &final_text, had_segments).await;
        match &processed {
            Some(text) => info!(
                "LLM post-processing returned ({} chars): '{}'",
                text.len(),
                &text[..text.len().min(100)]
            ),
            None => error!(
                "LLM post-processing returned None — check provider/model/prompt/API key config"
            ),
        }

        if let Some(processed_text) = processed {
            post_processed_text = Some(processed_text.clone());
            final_text = processed_text;

            if let Some(prompt_id) = &self.settings.post_process_selected_prompt_id {
                if let Some(prompt) = self
                    .settings
                    .post_process_prompts
                    .iter()
                    .find(|p| &p.id == prompt_id)
                {
                    post_process_prompt = Some(prompt.prompt.clone());
                }
            }
        } else if final_text != raw_text {
            post_processed_text = Some(final_text.clone());
        }

        // Save to history (pre-expansion text)
        let hm = Arc::clone(&self.app.state::<Arc<HistoryManager>>());
        let transcription_for_history = raw_text.clone();
        let samples_for_history = self.samples_for_history.clone();
        tauri::async_runtime::spawn(async move {
            if let Err(e) = hm
                .save_transcription(
                    samples_for_history,
                    transcription_for_history,
                    post_processed_text,
                    post_process_prompt,
                )
                .await
            {
                error!("Failed to save transcription to history: {}", e);
            }
        });

        final_text = self.expand_at_refs_for_output(&final_text);

        PipelineState::PostProcessed {
            raw_text,
            final_text,
            raw_text_pasted,
        }
    }

    /// PostProcessed → Done
    fn apply_diff_and_finalize(&mut self) -> PipelineState {
        let (raw_text, final_text, raw_text_pasted) =
            match std::mem::replace(&mut self.state, PipelineState::Done) {
                PipelineState::PostProcessed {
                    raw_text,
                    final_text,
                    raw_text_pasted,
                } => (raw_text, final_text, raw_text_pasted),
                _ => unreachable!(),
            };

        let paste_time = Instant::now();
        info!(
            "Comparing for diff — original ({} chars): '{}' vs final ({} chars): '{}'",
            raw_text.len(),
            &raw_text[..raw_text.len().min(80)],
            final_text.len(),
            &final_text[..final_text.len().min(80)],
        );

        if !raw_text_pasted {
            let ah = self.app.clone();
            let text = final_text.clone();
            let settings = self.settings.clone();
            self.app
                .run_on_main_thread(move || {
                    if !text.is_empty() {
                        if let Err(e) = utils::paste_raw(text, ah.clone()) {
                            error!("Failed to paste finalized post-processed text: {}", e);
                        } else {
                            debug!(
                                "Final text pasted successfully in {:?}",
                                paste_time.elapsed()
                            );
                        }
                    }
                    apply_trailing_space_and_autosubmit(&ah, &settings);
                    utils::hide_recording_overlay(&ah);
                    change_tray_icon(&ah, TrayIconState::Idle);
                })
                .unwrap_or_else(|e| {
                    error!("Failed to run final paste on main thread: {:?}", e);
                    utils::hide_recording_overlay(&self.app);
                    change_tray_icon(&self.app, TrayIconState::Idle);
                });
        } else if let Some(diff) = compute_text_diff(&raw_text, &final_text) {
            debug!(
                "Applying diff: delete {} chars, insert {} chars, suffix {} chars",
                diff.delete_chars,
                diff.insert.len(),
                diff.suffix_chars,
            );
            let diff_insert = diff.insert.clone();
            let diff_suffix = diff.suffix_chars;
            let diff_delete = diff.delete_chars;
            let ah = self.app.clone();
            let settings = self.settings.clone();
            self.app
                .run_on_main_thread(move || {
                    match utils::apply_text_diff(diff_suffix, diff_delete, &diff_insert, ah.clone())
                    {
                        Ok(()) => debug!(
                            "Text diff applied successfully in {:?}",
                            paste_time.elapsed()
                        ),
                        Err(e) => error!("Failed to apply text diff: {}", e),
                    }
                    // Apply trailing space and auto-submit after diff
                    apply_trailing_space_and_autosubmit(&ah, &settings);
                    utils::hide_recording_overlay(&ah);
                    change_tray_icon(&ah, TrayIconState::Idle);
                })
                .unwrap_or_else(|e| {
                    error!("Failed to run diff on main thread: {:?}", e);
                    utils::hide_recording_overlay(&self.app);
                    change_tray_icon(&self.app, TrayIconState::Idle);
                });
        } else {
            // Text unchanged after processing
            info!("Text unchanged after processing (diff is None), no replacement needed");
            let ah = self.app.clone();
            let settings = self.settings.clone();
            self.app
                .run_on_main_thread(move || {
                    apply_trailing_space_and_autosubmit(&ah, &settings);
                    utils::hide_recording_overlay(&ah);
                    change_tray_icon(&ah, TrayIconState::Idle);
                })
                .unwrap_or_else(|e| {
                    error!("Failed to run finalize on main thread: {:?}", e);
                    utils::hide_recording_overlay(&self.app);
                    change_tray_icon(&self.app, TrayIconState::Idle);
                });
        }

        PipelineState::Done
    }

    /// Non-post-process finalization (already handled in transcribe_and_paste for that path).
    fn finalize(&self) -> PipelineState {
        utils::hide_recording_overlay(&self.app);
        change_tray_icon(&self.app, TrayIconState::Idle);
        PipelineState::Done
    }

    fn expand_at_refs_for_output(&self, text: &str) -> String {
        if !self.settings.at_file_expansion_enabled {
            return text.to_string();
        }

        let expanded =
            crate::at_file_expansion::maybe_expand_at_refs(text, &self.settings, &self.app);
        if expanded != text {
            let delta = expanded.len() as isize - text.len() as isize;
            debug!("@file expansion adjusted output by {} chars", delta);
        }
        expanded
    }

    /// Cleanup on error — hide overlay, reset tray.
    fn cleanup(&self) {
        utils::hide_recording_overlay(&self.app);
        change_tray_icon(&self.app, TrayIconState::Idle);
    }
}

// ============================================================================
// Trailing space & auto-submit helper
// ============================================================================

/// Apply trailing space and auto-submit as the very last step of
/// post-process mode. Raw text was pasted without these; this adds them.
fn apply_trailing_space_and_autosubmit(app: &AppHandle, settings: &AppSettings) {
    use crate::clipboard::paste_raw;
    use crate::input::EnigoState;
    use crate::settings::{AutoSubmitKey, PasteMethod};
    use enigo::{Direction, Key, Keyboard};

    if settings.append_trailing_space {
        if let Err(e) = paste_raw(" ".to_string(), app.clone()) {
            error!("Failed to paste trailing space: {}", e);
        }
    }

    if settings.auto_submit && settings.paste_method != PasteMethod::None {
        std::thread::sleep(std::time::Duration::from_millis(50));
        let enigo_state = match app.try_state::<EnigoState>() {
            Some(s) => s,
            None => {
                error!("Enigo state not initialized for auto-submit");
                return;
            }
        };
        let mut enigo = match enigo_state.0.lock() {
            Ok(e) => e,
            Err(e) => {
                error!("Failed to lock Enigo for auto-submit: {}", e);
                return;
            }
        };

        let result = match settings.auto_submit_key {
            AutoSubmitKey::Enter => enigo
                .key(Key::Return, Direction::Click)
                .map_err(|e| format!("{}", e)),
            AutoSubmitKey::CtrlEnter => enigo
                .key(Key::Control, Direction::Press)
                .and_then(|_| enigo.key(Key::Return, Direction::Click))
                .and_then(|_| enigo.key(Key::Control, Direction::Release))
                .map_err(|e| format!("{}", e)),
            AutoSubmitKey::CmdEnter => enigo
                .key(Key::Meta, Direction::Press)
                .and_then(|_| enigo.key(Key::Return, Direction::Click))
                .and_then(|_| enigo.key(Key::Meta, Direction::Release))
                .map_err(|e| format!("{}", e)),
        };
        if let Err(e) = result {
            error!("Failed to send auto-submit key: {}", e);
        }
    }

    // Copy to clipboard if configured
    use crate::settings::ClipboardHandling;
    if settings.clipboard_handling == ClipboardHandling::CopyToClipboard {
        // Note: we don't have the final text here easily, but this matches
        // the existing behaviour where paste() handles it internally.
        // For post-process mode, the clipboard copy already happens in paste_raw
        // or we skip it (matching current behavior).
    }
}

// ============================================================================
// Minimal diff for post-processing replacement
// ============================================================================

/// Describes the minimal edit needed to transform the original pasted text into
/// the post-processed text.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TextDiff {
    /// Number of characters from the common suffix that must be backspaced over.
    pub suffix_chars: usize,
    /// Number of characters in the original's changed middle region to delete.
    pub delete_chars: usize,
    /// The replacement text to insert in place of the deleted region.
    pub insert: String,
}

/// Compute the minimal diff between the original text (already pasted and visible)
/// and the new post-processed text. Finds the common prefix and suffix, then treats
/// everything in between as a single replacement region suitable for keyboard-based
/// apply (backspace over suffix + changed region, type replacement + suffix).
///
/// Returns `None` if the texts are identical.
pub(crate) fn compute_text_diff(original: &str, processed: &str) -> Option<TextDiff> {
    if original == processed {
        return None;
    }

    let orig_chars: Vec<char> = original.chars().collect();
    let proc_chars: Vec<char> = processed.chars().collect();

    // Find common prefix length
    let prefix_len = orig_chars
        .iter()
        .zip(proc_chars.iter())
        .take_while(|(a, b)| a == b)
        .count();

    // Find common suffix length (must not overlap with prefix)
    let max_suffix = orig_chars.len().min(proc_chars.len()) - prefix_len;
    let suffix_len = orig_chars
        .iter()
        .rev()
        .zip(proc_chars.iter().rev())
        .take(max_suffix)
        .take_while(|(a, b)| a == b)
        .count();

    let delete_chars = orig_chars.len() - prefix_len - suffix_len;
    let insert: String = proc_chars[prefix_len..proc_chars.len() - suffix_len]
        .iter()
        .collect();

    debug!(
        "Text diff: prefix={}, delete={}, insert={} chars, suffix={}",
        prefix_len,
        delete_chars,
        insert.len(),
        suffix_len,
    );

    Some(TextDiff {
        suffix_chars: suffix_len,
        delete_chars,
        insert,
    })
}

// ============================================================================
// Post-processing helpers (moved from actions.rs)
// ============================================================================

static LEAKED_JARGON_INSTRUCTION_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?is)\n?\s*IMPORTANT:\s*Use these exact spellings for technical terms:\s*.*?(?:\n\s*\n|$)",
    )
    .unwrap()
});

static LEAKED_AT_FILE_INSTRUCTION_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r#"(?is)\n?\s*IMPORTANT:\s*Preserve any @file-style references exactly\s*\(for example @main\.rs or @"my file\.ts"\)\.\s*Do not expand, remove, or rewrite these references\.\s*"#,
    )
    .unwrap()
});

static LEAKED_SEGMENT_INSTRUCTION_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?is)\n?\s*IMPORTANT:\s*This text was transcribed from multiple independent audio segments split on silence\..*?Remove these artifacts and produce natural, flowing text that reflects what the speaker actually said\.\s*",
    )
    .unwrap()
});

const BASE_DICTATION_SYSTEM_MESSAGE: &str =
    "You are a dictation post-processor. Follow these rules strictly:
1) Do not invent facts, events, names, owners, dates, or outcomes.
2) Preserve the speaker's exact claims and intent.
3) If a detail is uncertain or missing, keep it vague rather than guessing.
4) Keep technical identifiers, code tokens, file paths, CLI flags, and URLs unchanged.
5) Do not add extra explanation or commentary beyond the requested output format.";

fn strip_leaked_prompt_instructions(text: &str) -> String {
    let no_jargon = LEAKED_JARGON_INSTRUCTION_REGEX.replace_all(text, "\n");
    let no_at_file = LEAKED_AT_FILE_INSTRUCTION_REGEX.replace_all(&no_jargon, "\n");
    let no_segments = LEAKED_SEGMENT_INSTRUCTION_REGEX.replace_all(&no_at_file, "\n");
    no_segments.trim().to_string()
}

async fn post_process_transcription(
    app: &AppHandle,
    settings: &AppSettings,
    transcription: &str,
    had_segments: bool,
) -> Option<String> {
    let provider = match settings.active_post_process_provider().cloned() {
        Some(provider) => provider,
        None => {
            debug!("Post-processing enabled but no provider is selected");
            return None;
        }
    };

    let model = settings
        .post_process_models
        .get(&provider.id)
        .cloned()
        .unwrap_or_default();

    if model.trim().is_empty() {
        debug!(
            "Post-processing skipped because provider '{}' has no model configured",
            provider.id
        );
        return None;
    }

    let selected_prompt_id = match select_post_process_prompt_id(app, settings, transcription) {
        Some(id) => id.clone(),
        None => {
            debug!("Post-processing skipped because no prompt is selected");
            return None;
        }
    };

    let prompt = match settings
        .post_process_prompts
        .iter()
        .find(|prompt| prompt.id == selected_prompt_id)
    {
        Some(prompt) => prompt.prompt.clone(),
        None => {
            debug!(
                "Post-processing skipped because prompt '{}' was not found",
                selected_prompt_id
            );
            return None;
        }
    };

    if prompt.trim().is_empty() {
        debug!("Post-processing skipped because the selected prompt is empty");
        return None;
    }

    debug!(
        "Starting LLM post-processing with provider '{}' (model: {})",
        provider.id, model
    );

    let mut processed_prompt = prompt.replace("${output}", transcription);

    // Build system message with global dictation safety rules plus optional segment context.
    let mut system_parts: Vec<&str> = vec![BASE_DICTATION_SYSTEM_MESSAGE];
    if had_segments {
        system_parts.push(
            "This text was transcribed from multiple independent audio chunks during live dictation. \
            The speech recognition model processed each segment separately, which causes several artifacts you must fix: \
            missing spaces between segments (words from adjacent segments may be concatenated together without a space), \
            incorrect sentence-ending punctuation inserted mid-thought (periods, ellipses where the speaker was just pausing), \
            incorrect capitalization at segment boundaries (words capitalized because they started a new segment, not a new sentence), \
            ellipses or trailing punctuation where the speaker simply paused, \
            and utterance completion artifacts (the model may have added filler words or tried to complete a sentence at a segment boundary). \
            Remove these artifacts and produce natural, flowing text that reflects what the speaker actually said.",
        );
    }
    let system_message = Some(system_parts.join("\n\n"));

    // Inject jargon context if active
    if !settings.jargon_enabled_profiles.is_empty()
        || !settings.jargon_custom_terms.is_empty()
        || !settings.jargon_packs.is_empty()
    {
        let profiles = build_profiles_map(settings);
        let effective_profiles = effective_profiles_for_text(app, settings, transcription);
        let jargon_settings = crate::jargon::JargonSettings {
            enabled_profiles: effective_profiles,
            custom_terms: settings.jargon_custom_terms.clone(),
            custom_corrections: settings.jargon_custom_corrections.clone(),
        };
        let dict = crate::jargon::compute_active_dictionary(&jargon_settings, &profiles);
        if !dict.terms.is_empty() {
            let terms_str: Vec<&str> = dict.terms.iter().map(|s| s.as_str()).collect();
            processed_prompt = format!(
                "{}\n\nIMPORTANT: Use these exact spellings for technical terms: {}",
                processed_prompt,
                terms_str.join(", ")
            );
        }
    }

    if settings.at_file_expansion_enabled {
        processed_prompt.push_str(
            "\n\nIMPORTANT: Preserve any @file-style references exactly (for example @main.rs or @\"my file.ts\"). Do not expand, remove, or rewrite these references.",
        );
    }

    debug!(
        "Processed prompt (had_segments={}, {} chars): '{}'",
        had_segments,
        processed_prompt.len(),
        &processed_prompt[..processed_prompt.len().min(500)]
    );

    if provider.id == APPLE_INTELLIGENCE_PROVIDER_ID {
        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        {
            if !apple_intelligence::check_apple_intelligence_availability() {
                debug!("Apple Intelligence selected but not currently available on this device");
                return None;
            }

            // Apple Intelligence has no system message support, so prepend
            // segment context directly into the prompt as a preamble.
            let ai_prompt = if let Some(ref sys) = system_message {
                format!("[System instruction: {}]\n\n{}", sys, processed_prompt)
            } else {
                processed_prompt.clone()
            };

            let token_limit = model.trim().parse::<i32>().unwrap_or(0);
            return match apple_intelligence::process_text(&ai_prompt, token_limit) {
                Ok(result) => {
                    let sanitized = strip_leaked_prompt_instructions(&result);
                    if sanitized.trim().is_empty() {
                        debug!("Apple Intelligence returned an empty response");
                        None
                    } else {
                        debug!(
                            "Apple Intelligence post-processing succeeded. Output length: {} chars",
                            sanitized.len()
                        );
                        Some(sanitized)
                    }
                }
                Err(err) => {
                    error!("Apple Intelligence post-processing failed: {}", err);
                    None
                }
            };
        }

        #[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
        {
            debug!("Apple Intelligence provider selected on unsupported platform");
            return None;
        }
    }

    let api_key = settings
        .post_process_api_keys
        .get(&provider.id)
        .cloned()
        .unwrap_or_default();

    match crate::llm_client::send_chat_completion(
        &provider,
        api_key,
        &model,
        processed_prompt,
        system_message,
    )
    .await
    {
        Ok(Some(content)) => {
            let content = content
                .replace('\u{200B}', "")
                .replace('\u{200C}', "")
                .replace('\u{200D}', "")
                .replace('\u{FEFF}', "");
            let content = strip_leaked_prompt_instructions(&content);
            debug!(
                "LLM post-processing succeeded for provider '{}'. Output length: {} chars",
                provider.id,
                content.len()
            );
            Some(content)
        }
        Ok(None) => {
            error!("LLM API response has no content");
            None
        }
        Err(e) => {
            error!(
                "LLM post-processing failed for provider '{}': {}. Falling back to original transcription.",
                provider.id,
                e
            );
            None
        }
    }
}

fn select_post_process_prompt_id(
    app: &AppHandle,
    settings: &AppSettings,
    transcription: &str,
) -> Option<String> {
    let fallback = settings.post_process_selected_prompt_id.clone();
    if !settings.post_process_auto_prompt_selection {
        return fallback;
    }

    let selector = match app.try_state::<Arc<DomainSelectorManager>>() {
        Some(selector) => selector,
        None => return fallback,
    };

    selector
        .select_post_process_prompt_with_timeout(
            settings,
            &DomainContext {
                text: transcription.to_string(),
            },
            &settings.post_process_prompts,
        )
        .or(fallback)
}

async fn maybe_convert_chinese_variant(
    settings: &AppSettings,
    transcription: &str,
) -> Option<String> {
    let is_simplified = settings.selected_language == "zh-Hans";
    let is_traditional = settings.selected_language == "zh-Hant";

    if !is_simplified && !is_traditional {
        debug!("selected_language is not Simplified or Traditional Chinese; skipping translation");
        return None;
    }

    debug!(
        "Starting Chinese translation using OpenCC for language: {}",
        settings.selected_language
    );

    let config = if is_simplified {
        BuiltinConfig::Tw2sp
    } else {
        BuiltinConfig::S2twp
    };

    match OpenCC::from_config(config) {
        Ok(converter) => {
            let converted = converter.convert(transcription);
            debug!(
                "OpenCC translation completed. Input length: {}, Output length: {}",
                transcription.len(),
                converted.len()
            );
            Some(converted)
        }
        Err(e) => {
            error!("Failed to initialize OpenCC converter: {}. Falling back to original transcription.", e);
            None
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diff_identical_returns_none() {
        assert_eq!(compute_text_diff("hello world", "hello world"), None);
    }

    #[test]
    fn test_diff_completely_different() {
        let diff = compute_text_diff("abc", "xyz").unwrap();
        assert_eq!(diff.suffix_chars, 0);
        assert_eq!(diff.delete_chars, 3);
        assert_eq!(diff.insert, "xyz");
    }

    #[test]
    fn test_diff_common_prefix() {
        let diff = compute_text_diff("hello world", "hello there").unwrap();
        assert_eq!(diff.suffix_chars, 0);
        assert_eq!(diff.delete_chars, 5);
        assert_eq!(diff.insert, "there");
    }

    #[test]
    fn test_diff_common_suffix() {
        let diff = compute_text_diff("bad world", "good world").unwrap();
        assert_eq!(diff.suffix_chars, 7);
        assert_eq!(diff.delete_chars, 2);
        assert_eq!(diff.insert, "goo");
    }

    #[test]
    fn test_diff_common_prefix_and_suffix() {
        let diff = compute_text_diff("the quick brown fox", "the slow brown fox").unwrap();
        assert_eq!(diff.suffix_chars, 10);
        assert_eq!(diff.delete_chars, 5);
        assert_eq!(diff.insert, "slow");
    }

    #[test]
    fn test_diff_insertion_only() {
        let diff = compute_text_diff("hello world", "hello beautiful world").unwrap();
        assert_eq!(diff.suffix_chars, 5);
        assert_eq!(diff.delete_chars, 0);
        assert_eq!(diff.insert, "beautiful ");
    }

    #[test]
    fn test_diff_deletion_only() {
        let diff = compute_text_diff("hello beautiful world", "hello world").unwrap();
        assert_eq!(diff.suffix_chars, 5);
        assert_eq!(diff.delete_chars, 10);
        assert_eq!(diff.insert, "");
    }

    #[test]
    fn test_diff_filler_word_removal() {
        let diff = compute_text_diff(
            "so um I think we should refactor",
            "I think we should refactor",
        )
        .unwrap();
        assert!(diff.delete_chars > 0);
        assert!(diff.insert.len() < "so um I think we should refactor".len());
    }

    #[test]
    fn test_diff_punctuation_change() {
        let diff = compute_text_diff("hello world", "Hello world.").unwrap();
        assert_eq!(diff.suffix_chars, 0);
        assert_eq!(diff.delete_chars, 11);
        assert_eq!(diff.insert, "Hello world.");
    }

    #[test]
    fn test_diff_middle_change_preserves_both_ends() {
        let diff = compute_text_diff("The cat sat on the mat", "The dog sat on the mat").unwrap();
        assert_eq!(diff.suffix_chars, 15);
        assert_eq!(diff.delete_chars, 3);
        assert_eq!(diff.insert, "dog");
    }

    // ── Real-world segment-on-silence test cases ──────────────────────

    /// Regression: multiple scattered changes (filler removal + punctuation)
    /// caused the old similar-based diff to fail with "Diff coalescing failed".
    #[test]
    fn test_diff_segment_filler_removal_and_punctuation() {
        // From actual segment-on-silence session: LLM removes "ni" filler
        // and adds period after "speech"
        let original = "this is the test to determine if it's a little bit of a \
                         ni text to speech silence segmentation is working \
                         the pasting is not working";
        let processed = "this is the test to determine if it's a little bit of a \
                          text to speech. Silence segmentation is working, \
                          the pasting is not working.";
        let diff = compute_text_diff(original, processed);
        assert!(
            diff.is_some(),
            "diff must not be None for multi-change text"
        );

        // Verify round-trip: applying diff to original produces processed
        let d = diff.unwrap();
        let orig_chars: Vec<char> = original.chars().collect();
        let applied = format!(
            "{}{}{}",
            orig_chars[..orig_chars.len() - d.suffix_chars - d.delete_chars]
                .iter()
                .collect::<String>(),
            d.insert,
            orig_chars[orig_chars.len() - d.suffix_chars..]
                .iter()
                .collect::<String>(),
        );
        assert_eq!(applied, processed);
    }

    /// Segments concatenated without spaces (no trailing space in segments)
    /// should still diff correctly when LLM adds spaces back.
    #[test]
    fn test_diff_segments_no_spaces_llm_adds_them() {
        let original = "please do add the integration tests, preferably with an i term to\
                         process actually launched\
                         such that we can\
                         properly\
                         assess whether or not\
                         our current integration method";
        let processed = "Please do add the integration tests, preferably with an iTerm to \
                          process actually launched such that we can properly assess \
                          whether or not our current integration method.";
        let diff = compute_text_diff(original, processed);
        assert!(diff.is_some(), "diff must succeed when LLM inserts spaces");
    }

    /// Segments with trailing spaces — the consistent tracking approach.
    /// Each segment stored with its trailing space, joined with "".
    #[test]
    fn test_diff_segments_with_trailing_spaces() {
        // Simulates: segments are ["okay let's test ", "i'm not sure ", "there may be bugs "]
        // joined with "" = "okay let's test i'm not sure there may be bugs "
        let segments = vec!["okay let's test ", "i'm not sure ", "there may be bugs "];
        let original: String = segments.join("");
        let processed = "Okay, let's test. I'm not sure there may be bugs.";
        let diff = compute_text_diff(&original, processed);
        assert!(
            diff.is_some(),
            "diff must succeed for segments with trailing spaces"
        );

        let d = diff.unwrap();
        let orig_chars: Vec<char> = original.chars().collect();
        let applied = format!(
            "{}{}{}",
            orig_chars[..orig_chars.len() - d.suffix_chars - d.delete_chars]
                .iter()
                .collect::<String>(),
            d.insert,
            orig_chars[orig_chars.len() - d.suffix_chars..]
                .iter()
                .collect::<String>(),
        );
        assert_eq!(applied, processed);
    }

    /// LLM capitalizes first letter and adds trailing period — changes at both ends.
    #[test]
    fn test_diff_capitalize_and_add_period() {
        let diff =
            compute_text_diff("this is a test sentence", "This is a test sentence.").unwrap();
        // No common suffix (period added at end), prefix is 0 (case change at start)
        assert_eq!(diff.suffix_chars, 0);
        assert_eq!(diff.delete_chars, 23);
        assert_eq!(diff.insert, "This is a test sentence.");
    }

    /// Multiple segment boundaries cleaned up by LLM.
    #[test]
    fn test_diff_multiple_segment_boundary_cleanup() {
        let original = "okay let's test i'm not sure if it's going to there may be other small bugs such as the growing number of spaces being added to the end of each segments ";
        let processed = "Okay, let's test. I'm not sure if it's going to there may be other small bugs, such as the growing number of spaces being added to the end of each segment.";
        let diff = compute_text_diff(original, processed);
        assert!(diff.is_some());

        let d = diff.unwrap();
        let orig_chars: Vec<char> = original.chars().collect();
        let applied = format!(
            "{}{}{}",
            orig_chars[..orig_chars.len() - d.suffix_chars - d.delete_chars]
                .iter()
                .collect::<String>(),
            d.insert,
            orig_chars[orig_chars.len() - d.suffix_chars..]
                .iter()
                .collect::<String>(),
        );
        assert_eq!(applied, processed);
    }
}
