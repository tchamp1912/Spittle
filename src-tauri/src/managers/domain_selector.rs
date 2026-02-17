use crate::jargon::{JargonCorrection, JargonProfile};
use crate::settings::{AppSettings, LLMPrompt};
use log::{debug, warn};
use std::collections::{HashMap, HashSet};
use std::sync::mpsc;
use std::sync::Mutex;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct DomainContext {
    pub text: String,
}

#[derive(Debug, Clone)]
struct RankedProfile {
    profile_id: String,
    score: f32,
}

#[derive(Debug, Clone, Default)]
struct LastSelection {
    profile_id: String,
    score: f32,
}

#[derive(Default)]
pub struct DomainSelectorManager {
    last_selection: Mutex<Option<LastSelection>>,
    last_prompt_selection: Mutex<Option<LastSelection>>,
}

impl DomainSelectorManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn select_profiles_with_timeout(
        &self,
        settings: &AppSettings,
        context: &DomainContext,
    ) -> Option<Vec<String>> {
        if !settings.domain_selector_enabled {
            return None;
        }
        if context.text.trim().is_empty() {
            return None;
        }

        let timeout = Duration::from_millis(settings.domain_selector_timeout_ms.max(25));
        let top_k = settings.domain_selector_top_k.max(1);
        let min_score = settings.domain_selector_min_score.clamp(0.0, 1.0);
        let hysteresis = settings.domain_selector_hysteresis.clamp(0.0, 1.0);

        let profiles = build_profile_map(settings);
        let context_text = context.text.clone();
        let (tx, rx) = mpsc::channel();

        std::thread::spawn(move || {
            let ranked = score_profiles(&profiles, &context_text);
            let _ = tx.send(ranked);
        });

        let ranked = match rx.recv_timeout(timeout) {
            Ok(ranked) => ranked,
            Err(_) => {
                warn!(
                    "Domain selector sidecar timed out after {}ms",
                    timeout.as_millis()
                );
                return None;
            }
        };

        let mut selected: Vec<RankedProfile> = ranked
            .into_iter()
            .filter(|item| item.score >= min_score)
            .take(top_k)
            .collect();
        if selected.is_empty() {
            return None;
        }

        let maybe_last = self
            .last_selection
            .lock()
            .ok()
            .and_then(|last| (*last).clone());
        if let Some(last) = maybe_last {
            if let Some(current_top) = selected.first() {
                let switched = current_top.profile_id != last.profile_id;
                let beat_by_margin = current_top.score >= (last.score + hysteresis);
                if switched && !beat_by_margin {
                    selected.insert(
                        0,
                        RankedProfile {
                            profile_id: last.profile_id.clone(),
                            score: last.score,
                        },
                    );
                    selected.truncate(top_k);
                }
            }
        }

        if let Some(top) = selected.first() {
            if let Ok(mut last) = self.last_selection.lock() {
                *last = Some(LastSelection {
                    profile_id: top.profile_id.clone(),
                    score: top.score,
                });
            }
        }

        debug!(
            "Domain selector picked profiles: {:?}",
            selected
                .iter()
                .map(|item| format!("{}:{:.3}", item.profile_id, item.score))
                .collect::<Vec<_>>()
        );

        Some(selected.into_iter().map(|item| item.profile_id).collect())
    }

    pub fn select_post_process_prompt_with_timeout(
        &self,
        settings: &AppSettings,
        context: &DomainContext,
        prompts: &[LLMPrompt],
    ) -> Option<String> {
        if !settings.post_process_auto_prompt_selection {
            return None;
        }
        if context.text.trim().is_empty() || prompts.is_empty() {
            return None;
        }

        // Keep prompt auto-selection ultra-fast; if it cannot finish quickly,
        // fail open and let the normal selected prompt flow continue.
        let timeout_ms = settings.domain_selector_timeout_ms.clamp(10, 80);
        let timeout = Duration::from_millis(timeout_ms);
        let min_score = settings.domain_selector_min_score.clamp(0.0, 1.0);
        let hysteresis = settings.domain_selector_hysteresis.clamp(0.0, 1.0);
        let context_text: String = context.text.chars().take(2000).collect();
        let prompts_vec = prompts.to_vec();
        let (tx, rx) = mpsc::channel();

        std::thread::spawn(move || {
            let ranked = score_prompts(&prompts_vec, &context_text);
            let _ = tx.send(ranked);
        });

        let mut ranked = match rx.recv_timeout(timeout) {
            Ok(ranked) => ranked,
            Err(_) => {
                warn!(
                    "Prompt selector sidecar timed out after {}ms",
                    timeout.as_millis()
                );
                return None;
            }
        };

        if ranked.is_empty() || ranked[0].score < min_score {
            return None;
        }

        if let Some(last) = self
            .last_prompt_selection
            .lock()
            .ok()
            .and_then(|last| (*last).clone())
        {
            let current_top = &ranked[0];
            let switched = current_top.profile_id != last.profile_id;
            let beat_by_margin = current_top.score >= (last.score + hysteresis);
            if switched && !beat_by_margin {
                ranked.insert(
                    0,
                    RankedProfile {
                        profile_id: last.profile_id,
                        score: last.score,
                    },
                );
            }
        }

        let selected = ranked[0].profile_id.clone();
        if let Ok(mut last) = self.last_prompt_selection.lock() {
            *last = Some(LastSelection {
                profile_id: selected.clone(),
                score: ranked[0].score,
            });
        }
        debug!(
            "Post-process prompt selector picked '{}': {:.3}",
            selected, ranked[0].score
        );
        Some(selected)
    }
}

fn build_profile_map(settings: &AppSettings) -> HashMap<String, JargonProfile> {
    let mut profiles = crate::jargon::builtin_profiles();
    for pack in &settings.jargon_packs {
        profiles.insert(
            pack.id.clone(),
            JargonProfile {
                label: pack.label.clone(),
                terms: pack.terms.clone(),
                corrections: pack.corrections.clone(),
            },
        );
    }
    profiles
}

fn score_profiles(profiles: &HashMap<String, JargonProfile>, text: &str) -> Vec<RankedProfile> {
    let context_tokens = tokenize(text);
    if context_tokens.is_empty() {
        return Vec::new();
    }

    let mut ranked = Vec::new();
    for (profile_id, profile) in profiles {
        let mut score = 0.0_f32;

        for term in &profile.terms {
            let term_tokens = tokenize(term);
            if term_tokens.is_empty() {
                continue;
            }
            let overlap = token_overlap_ratio(&context_tokens, &term_tokens);
            score += overlap * 1.0;
        }

        for JargonCorrection { from, to } in &profile.corrections {
            let from_tokens = tokenize(from);
            if !from_tokens.is_empty() {
                score += token_overlap_ratio(&context_tokens, &from_tokens) * 1.2;
            }
            let to_tokens = tokenize(to);
            if !to_tokens.is_empty() {
                score += token_overlap_ratio(&context_tokens, &to_tokens) * 1.0;
            }
        }

        let normalization =
            (profile.terms.len() as f32 + profile.corrections.len() as f32 * 1.5).max(1.0);
        let normalized = (score / normalization).clamp(0.0, 1.0);
        if normalized > 0.0 {
            ranked.push(RankedProfile {
                profile_id: profile_id.clone(),
                score: normalized,
            });
        }
    }

    ranked.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.profile_id.cmp(&b.profile_id))
    });
    ranked
}

fn tokenize(text: &str) -> HashSet<String> {
    text.split(|c: char| !c.is_alphanumeric() && c != '+' && c != '#')
        .map(|token| token.trim().to_lowercase())
        .filter(|token| token.len() > 1)
        .collect()
}

fn token_overlap_ratio(
    context_tokens: &HashSet<String>,
    candidate_tokens: &HashSet<String>,
) -> f32 {
    if candidate_tokens.is_empty() {
        return 0.0;
    }
    let overlap = candidate_tokens
        .iter()
        .filter(|token| context_tokens.contains(*token))
        .count();
    overlap as f32 / candidate_tokens.len() as f32
}

fn score_prompts(prompts: &[LLMPrompt], text: &str) -> Vec<RankedProfile> {
    let context_tokens = tokenize(text);
    if context_tokens.is_empty() {
        return Vec::new();
    }
    let joined_text = text.to_lowercase();
    let mut ranked = Vec::new();

    for prompt in prompts {
        let mut score = 0.0_f32;
        let signature = prompt_signature(prompt);
        let signature_tokens = tokenize(&signature);
        score += token_overlap_ratio(&context_tokens, &signature_tokens) * 1.8;

        for keyword in prompt_keywords(&prompt.id) {
            if joined_text.contains(keyword) {
                score += 0.2;
            }
        }

        let normalized = score.clamp(0.0, 1.0);
        if normalized > 0.0 {
            ranked.push(RankedProfile {
                profile_id: prompt.id.clone(),
                score: normalized,
            });
        }
    }

    ranked.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.profile_id.cmp(&b.profile_id))
    });
    ranked
}

fn prompt_signature(prompt: &LLMPrompt) -> String {
    // Avoid scoring against full prompt instructions (can be large and noisy).
    // We rely on short, stable metadata for low-latency routing.
    format!("{} {}", prompt.id, prompt.name)
}

fn prompt_keywords(prompt_id: &str) -> &'static [&'static str] {
    match prompt_id {
        "default_action_items" => &[
            "action item",
            "todo",
            "next steps",
            "owner",
            "deadline",
            "task",
        ],
        "default_document_writer" => &[
            "document",
            "proposal",
            "design doc",
            "write-up",
            "spec",
            "draft",
        ],
        "default_meeting_notes" => &[
            "meeting",
            "agenda",
            "decisions",
            "attendees",
            "recap",
            "notes",
        ],
        "default_slack_message" => &["slack", "channel", "team update", "quick update", "message"],
        _ => &[],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::JargonPack;
    use serde::Deserialize;
    use std::fs;
    use std::path::PathBuf;

    fn make_settings() -> AppSettings {
        let mut settings = crate::settings::get_default_settings();
        settings.domain_selector_enabled = true;
        settings.domain_selector_timeout_ms = 100;
        settings.domain_selector_top_k = 2;
        settings.domain_selector_min_score = 0.05;
        settings.domain_selector_hysteresis = 0.05;
        settings.jargon_packs = vec![JargonPack {
            id: "custom_rust".to_string(),
            label: "Rust Pack".to_string(),
            terms: vec![
                "Rust".to_string(),
                "Cargo".to_string(),
                "Clippy".to_string(),
            ],
            corrections: vec![JargonCorrection {
                from: "rust lang".to_string(),
                to: "Rust".to_string(),
            }],
        }];
        settings
    }

    #[test]
    fn selector_returns_profile_for_matching_text() {
        let manager = DomainSelectorManager::new();
        let settings = make_settings();
        let context = DomainContext {
            text: "cargo clippy rust lang".to_string(),
        };
        let result = manager.select_profiles_with_timeout(&settings, &context);
        assert!(result.is_some());
        let ids = result.unwrap();
        assert!(ids.iter().any(|id| id == "custom_rust"));
    }

    #[test]
    fn selector_is_disabled_by_setting() {
        let manager = DomainSelectorManager::new();
        let mut settings = make_settings();
        settings.domain_selector_enabled = false;
        let context = DomainContext {
            text: "terraform kubernetes".to_string(),
        };
        assert!(manager
            .select_profiles_with_timeout(&settings, &context)
            .is_none());
    }

    #[test]
    fn selector_returns_none_for_empty_context() {
        let manager = DomainSelectorManager::new();
        let settings = make_settings();
        let context = DomainContext {
            text: "   ".to_string(),
        };
        assert!(manager
            .select_profiles_with_timeout(&settings, &context)
            .is_none());
    }

    #[test]
    fn prompt_selector_prefers_action_items_keywords() {
        let manager = DomainSelectorManager::new();
        let mut settings = make_settings();
        settings.post_process_auto_prompt_selection = true;
        let prompts = vec![
            LLMPrompt {
                id: "default_action_items".to_string(),
                name: "Action Items".to_string(),
                prompt: "Extract tasks".to_string(),
            },
            LLMPrompt {
                id: "default_document_writer".to_string(),
                name: "Document Writer".to_string(),
                prompt: "Write a document".to_string(),
            },
        ];
        let selected = manager.select_post_process_prompt_with_timeout(
            &settings,
            &DomainContext {
                text: "next steps owner and deadline for each task".to_string(),
            },
            &prompts,
        );
        assert_eq!(selected, Some("default_action_items".to_string()));
    }

    #[derive(Debug, Deserialize)]
    struct ProfileEvalSuite {
        #[serde(default)]
        description: String,
        #[serde(default = "default_min_pass_rate")]
        min_pass_rate: f32,
        #[serde(default)]
        settings: ProfileEvalSettings,
        cases: Vec<ProfileEvalCase>,
    }

    #[derive(Debug, Deserialize, Default)]
    struct ProfileEvalSettings {
        #[serde(default)]
        top_k: Option<usize>,
        #[serde(default)]
        min_score: Option<f32>,
        #[serde(default)]
        timeout_ms: Option<u64>,
    }

    #[derive(Debug, Deserialize)]
    struct ProfileEvalCase {
        id: String,
        input: String,
        #[serde(default)]
        expect_any_of: Vec<String>,
        #[serde(default)]
        forbid: Vec<String>,
        #[serde(default)]
        expect_none: bool,
        #[serde(default)]
        track_only: bool,
        #[serde(default)]
        notes: Option<String>,
    }

    #[derive(Debug, Deserialize)]
    struct PromptEvalSuite {
        #[serde(default)]
        description: String,
        #[serde(default = "default_min_pass_rate")]
        min_pass_rate: f32,
        #[serde(default)]
        settings: PromptEvalSettings,
        cases: Vec<PromptEvalCase>,
    }

    #[derive(Debug, Deserialize, Default)]
    struct PromptEvalSettings {
        #[serde(default)]
        min_score: Option<f32>,
        #[serde(default)]
        timeout_ms: Option<u64>,
        #[serde(default)]
        hysteresis: Option<f32>,
    }

    #[derive(Debug, Deserialize)]
    struct PromptEvalCase {
        id: String,
        input: String,
        expect_prompt: String,
        #[serde(default)]
        fallback_prompt: Option<String>,
        #[serde(default)]
        track_only: bool,
        #[serde(default)]
        notes: Option<String>,
    }

    fn default_min_pass_rate() -> f32 {
        0.8
    }

    fn load_eval_suite() -> ProfileEvalSuite {
        let path = std::env::var("SPITTLE_DOMAIN_SELECTOR_EVALS")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("tests")
                    .join("domain_selector_profiles_evals.json")
            });

        let raw = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("failed to read eval file {}: {}", path.display(), e));
        serde_json::from_str::<ProfileEvalSuite>(&raw)
            .unwrap_or_else(|e| panic!("invalid eval JSON {}: {}", path.display(), e))
    }

    fn load_prompt_eval_suite() -> PromptEvalSuite {
        let path = std::env::var("SPITTLE_PROMPT_SELECTOR_EVALS")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("tests")
                    .join("prompt_selector_evals.json")
            });

        let raw = fs::read_to_string(&path).unwrap_or_else(|e| {
            panic!("failed to read prompt eval file {}: {}", path.display(), e)
        });
        serde_json::from_str::<PromptEvalSuite>(&raw)
            .unwrap_or_else(|e| panic!("invalid prompt eval JSON {}: {}", path.display(), e))
    }

    #[test]
    fn profile_selector_passes_eval_suite() {
        let suite = load_eval_suite();
        assert!(
            !suite.cases.is_empty(),
            "eval suite must contain at least one case"
        );

        let mut settings = make_settings();
        settings.domain_selector_enabled = true;
        settings.domain_selector_top_k = suite.settings.top_k.unwrap_or(2);
        settings.domain_selector_min_score = suite.settings.min_score.unwrap_or(0.08);
        settings.domain_selector_timeout_ms = suite.settings.timeout_ms.unwrap_or(80);

        let scored_total = suite.cases.iter().filter(|case| !case.track_only).count();
        assert!(scored_total > 0, "profile eval suite has no scored cases");

        let mut passed = 0usize;
        let mut failures = Vec::new();
        let mut tracked_misses = Vec::new();

        for case in &suite.cases {
            let manager = DomainSelectorManager::new();
            let selected = manager
                .select_profiles_with_timeout(
                    &settings,
                    &DomainContext {
                        text: case.input.clone(),
                    },
                )
                .unwrap_or_default();

            let has_forbidden = selected
                .iter()
                .any(|id| case.forbid.iter().any(|forbidden| forbidden == id));
            let has_expected = if case.expect_any_of.is_empty() {
                true
            } else {
                selected
                    .iter()
                    .any(|id| case.expect_any_of.iter().any(|expected| expected == id))
            };
            let pass = if case.expect_none {
                selected.is_empty()
            } else {
                has_expected && !has_forbidden
            };

            if case.track_only {
                if !pass {
                    tracked_misses.push(format!(
                        "{} => selected={:?}, expect_any_of={:?}, forbid={:?}, expect_none={}, notes={}",
                        case.id,
                        selected,
                        case.expect_any_of,
                        case.forbid,
                        case.expect_none,
                        case.notes.clone().unwrap_or_default()
                    ));
                }
                continue;
            }

            if pass {
                passed += 1;
            } else {
                failures.push(format!(
                    "{} => selected={:?}, expect_any_of={:?}, forbid={:?}, expect_none={}, notes={}",
                    case.id,
                    selected,
                    case.expect_any_of,
                    case.forbid,
                    case.expect_none,
                    case.notes.clone().unwrap_or_default()
                ));
            }
        }

        let pass_rate = passed as f32 / scored_total as f32;
        eprintln!(
            "[profile eval] suite='{}' pass_rate={:.2} scored={}/{} tracked_misses={}",
            suite.description,
            pass_rate,
            passed,
            scored_total,
            tracked_misses.len()
        );
        if !tracked_misses.is_empty() {
            eprintln!(
                "[profile eval] tracked misses:\n{}",
                tracked_misses.join("\n")
            );
        }
        assert!(
            pass_rate >= suite.min_pass_rate,
            "domain selector eval failed: pass_rate={:.2} < {:.2}; suite='{}'; failures:\n{}",
            pass_rate,
            suite.min_pass_rate,
            suite.description,
            failures.join("\n")
        );
    }

    fn eval_prompts() -> Vec<LLMPrompt> {
        vec![
            LLMPrompt {
                id: "default_action_items".to_string(),
                name: "Action Items".to_string(),
                prompt: "Extract actionable tasks".to_string(),
            },
            LLMPrompt {
                id: "default_document_writer".to_string(),
                name: "Document Writer".to_string(),
                prompt: "Structured document draft".to_string(),
            },
            LLMPrompt {
                id: "default_meeting_notes".to_string(),
                name: "Meeting Notes".to_string(),
                prompt: "Meeting summary decisions notes".to_string(),
            },
            LLMPrompt {
                id: "default_slack_message".to_string(),
                name: "Slack Message".to_string(),
                prompt: "Team update in slack format".to_string(),
            },
            LLMPrompt {
                id: "default_standup_update".to_string(),
                name: "Standup Update".to_string(),
                prompt: "Yesterday Today Blockers update".to_string(),
            },
        ]
    }

    #[test]
    fn prompt_selector_passes_eval_suite() {
        let suite = load_prompt_eval_suite();
        assert!(
            !suite.cases.is_empty(),
            "prompt eval suite must contain at least one case"
        );

        let prompts = eval_prompts();
        let mut settings = make_settings();
        settings.post_process_auto_prompt_selection = true;
        settings.domain_selector_min_score = suite.settings.min_score.unwrap_or(0.08);
        settings.domain_selector_timeout_ms = suite.settings.timeout_ms.unwrap_or(50);
        settings.domain_selector_hysteresis = suite.settings.hysteresis.unwrap_or(0.06);

        let scored_total = suite.cases.iter().filter(|case| !case.track_only).count();
        assert!(scored_total > 0, "prompt eval suite has no scored cases");

        let mut passed = 0usize;
        let mut failures = Vec::new();
        let mut tracked_misses = Vec::new();

        for case in &suite.cases {
            settings.post_process_selected_prompt_id = case
                .fallback_prompt
                .clone()
                .or_else(|| Some("default_improve_transcriptions".to_string()));

            let manager = DomainSelectorManager::new();
            let selected = manager.select_post_process_prompt_with_timeout(
                &settings,
                &DomainContext {
                    text: case.input.clone(),
                },
                &prompts,
            );

            let pass = selected.as_deref() == Some(case.expect_prompt.as_str());
            if case.track_only {
                if !pass {
                    tracked_misses.push(format!(
                        "{} => selected={:?}, expected={}, notes={}",
                        case.id,
                        selected,
                        case.expect_prompt,
                        case.notes.clone().unwrap_or_default()
                    ));
                }
                continue;
            }

            if pass {
                passed += 1;
            } else {
                failures.push(format!(
                    "{} => selected={:?}, expected={}, notes={}",
                    case.id,
                    selected,
                    case.expect_prompt,
                    case.notes.clone().unwrap_or_default()
                ));
            }
        }

        let pass_rate = passed as f32 / scored_total as f32;
        eprintln!(
            "[prompt eval] suite='{}' pass_rate={:.2} scored={}/{} tracked_misses={}",
            suite.description,
            pass_rate,
            passed,
            scored_total,
            tracked_misses.len()
        );
        if !tracked_misses.is_empty() {
            eprintln!(
                "[prompt eval] tracked misses:\n{}",
                tracked_misses.join("\n")
            );
        }
        assert!(
            pass_rate >= suite.min_pass_rate,
            "prompt selector eval failed: pass_rate={:.2} < {:.2}; suite='{}'; failures:\n{}",
            pass_rate,
            suite.min_pass_rate,
            suite.description,
            failures.join("\n")
        );
    }
}
