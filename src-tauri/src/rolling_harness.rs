use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};

static SPACE_BEFORE_PUNCT_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"\s+([,.;:!?])").unwrap());

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayScenario {
    pub name: String,
    pub hypotheses: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum RewriteStrategy {
    /// Ideal behavior: each rewrite fully replaces previously written rolling text.
    Perfect,
    /// Simulate dropped deletes where N chars survive each rewrite.
    UnderDeletePerRewrite(usize),
}

#[derive(Debug, Clone)]
pub struct ReplayReport {
    pub hypotheses_count: usize,
    pub final_expected: String,
    pub final_actual: String,
    pub rewrites_applied: usize,
    pub matches_expected: bool,
}

/// Keep model casing/punctuation, only normalize spacing artifacts, matching
/// rolling-mode normalization in audio manager.
pub fn normalize_hypothesis(text: &str) -> String {
    let collapsed = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let trimmed = collapsed.trim();
    SPACE_BEFORE_PUNCT_RE.replace_all(trimmed, "$1").to_string()
}

/// Normalize all hypotheses in a scenario.
pub fn normalize_scenario(mut scenario: ReplayScenario) -> ReplayScenario {
    scenario.hypotheses = scenario
        .hypotheses
        .into_iter()
        .map(|h| normalize_hypothesis(&h))
        .filter(|h| !h.is_empty())
        .collect();
    scenario
}

/// Replay rolling rewrite behavior against a virtual target buffer.
/// This lets us evaluate drift risk from rewrite strategy alone.
pub fn replay_hypotheses(hypotheses: &[String], strategy: RewriteStrategy) -> ReplayReport {
    let mut buffer = String::new();
    let mut rewrites_applied = 0usize;
    let mut last_emitted = String::new();

    for hypothesis in hypotheses {
        if last_emitted.is_empty() {
            buffer.push_str(hypothesis);
            last_emitted = hypothesis.clone();
            continue;
        }

        rewrites_applied += 1;
        match strategy {
            RewriteStrategy::Perfect => {
                buffer = hypothesis.clone();
            }
            RewriteStrategy::UnderDeletePerRewrite(remaining) => {
                // Simulate "N chars were left behind from previous span"
                // and new text got inserted after that surviving prefix.
                let prefix: String = last_emitted.chars().take(remaining).collect();
                buffer = format!("{}{}", prefix, hypothesis);
            }
        }
        last_emitted = hypothesis.clone();
    }

    let final_expected = hypotheses.last().cloned().unwrap_or_default();
    let matches_expected = buffer == final_expected;

    ReplayReport {
        hypotheses_count: hypotheses.len(),
        final_expected,
        final_actual: buffer,
        rewrites_applied,
        matches_expected,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_scenario_hypotheses() {
        let scenario = ReplayScenario {
            name: "spacing".to_string(),
            hypotheses: vec![
                " All   right . ".to_string(),
                "Alright , let's see".to_string(),
            ],
        };
        let normalized = normalize_scenario(scenario);
        assert_eq!(normalized.hypotheses.len(), 2);
        assert_eq!(normalized.hypotheses[0], "All right.");
        assert_eq!(normalized.hypotheses[1], "Alright, let's see");
    }

    #[test]
    fn perfect_replay_matches_last_hypothesis() {
        let hyps = vec![
            "All right.".to_string(),
            "Alright, let's see if that's doing any better.".to_string(),
            "Alright, let's see if that's doing any better. Nope.".to_string(),
        ];
        let report = replay_hypotheses(&hyps, RewriteStrategy::Perfect);
        assert!(report.matches_expected);
    }

    #[test]
    fn under_delete_replay_detects_prefix_drift() {
        let hyps = vec![
            "All right.".to_string(),
            "Alright, let's see if that's doing any better.".to_string(),
            "Alright, let's see if that's doing any better. Nope.".to_string(),
        ];
        let report = replay_hypotheses(&hyps, RewriteStrategy::UnderDeletePerRewrite(1));
        assert!(!report.matches_expected);
        assert!(report.final_actual.starts_with('A'));
        assert!(report.final_actual.len() > report.final_expected.len());
    }

    #[test]
    fn regression_prefix_repeat_alright_case() {
        // Derived from reported behavior where repeated rewrites left
        // prefix artifacts like "AAlAl..." in the target field.
        let scenario = normalize_scenario(ReplayScenario {
            name: "alright-prefix-drift".to_string(),
            hypotheses: vec![
                "All right.".to_string(),
                "Alright, let's see if that's doing any better.".to_string(),
                "Alright, let's see if that's doing any better. Nope.".to_string(),
                "Alright, let's see if that's doing any better. Nope, it's still leaving one to two characters per rewrite.".to_string(),
            ],
        });

        let perfect = replay_hypotheses(&scenario.hypotheses, RewriteStrategy::Perfect);
        assert!(
            perfect.matches_expected,
            "Perfect rewrite should end at exact final hypothesis"
        );

        let drift = replay_hypotheses(
            &scenario.hypotheses,
            RewriteStrategy::UnderDeletePerRewrite(1),
        );
        assert!(
            !drift.matches_expected,
            "Under-delete simulation must expose prefix drift risk"
        );
    }

    #[test]
    fn regression_prefix_repeat_okay_case() {
        let scenario = normalize_scenario(ReplayScenario {
            name: "okay-prefix-drift".to_string(),
            hypotheses: vec![
                "Okay.".to_string(),
                "Okay, let's see if this is working.".to_string(),
                "Okay, let's see if this is working. It's adding unnecessary commas and maybe extra spaces as well.".to_string(),
            ],
        });

        let perfect = replay_hypotheses(&scenario.hypotheses, RewriteStrategy::Perfect);
        assert!(perfect.matches_expected);

        let drift2 = replay_hypotheses(
            &scenario.hypotheses,
            RewriteStrategy::UnderDeletePerRewrite(2),
        );
        assert!(
            !drift2.matches_expected,
            "Two-char under-delete should create obvious duplication artifact"
        );
    }
}
