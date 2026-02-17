use log::warn;
use regex::Regex;
use serde::{Deserialize, Serialize};
use specta::Type;
use std::collections::HashMap;

// ============================================================================
// Data Types
// ============================================================================

#[derive(Serialize, Deserialize, Debug, Clone, Type)]
pub struct JargonCorrection {
    pub from: String,
    pub to: String,
}

#[derive(Serialize, Debug, Clone, Type)]
pub struct JargonProfile {
    pub label: String,
    pub terms: Vec<String>,
    pub corrections: Vec<JargonCorrection>,
}

pub struct JargonSettings {
    pub enabled_profiles: Vec<String>,
    pub custom_terms: Vec<String>,
    pub custom_corrections: Vec<JargonCorrection>,
}

pub struct ActiveDictionary {
    pub terms: Vec<String>,
    pub corrections: Vec<JargonCorrection>,
}

// ============================================================================
// Built-in Profiles
// ============================================================================

pub fn builtin_profiles() -> HashMap<String, JargonProfile> {
    let mut profiles = HashMap::new();

    profiles.insert(
        "web_dev".to_string(),
        JargonProfile {
            label: "Web Development".to_string(),
            terms: vec![
                "TypeScript",
                "JavaScript",
                "React",
                "Next.js",
                "Tailwind",
                "Webpack",
                "Vite",
                "GraphQL",
                "REST",
                "API",
                "JSON",
                "CORS",
                "OAuth",
                "JWT",
                "WebSocket",
                "SSR",
                "CSR",
                "SSG",
                "CDN",
                "DNS",
                "Vercel",
                "Netlify",
                "Supabase",
                "Prisma",
                "PostgreSQL",
                "MongoDB",
                "Redis",
                "Docker",
                "Kubernetes",
                "CI/CD",
                "GitHub",
                "npm",
                "pnpm",
                "Bun",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
            corrections: vec![
                JargonCorrection {
                    from: "next js".into(),
                    to: "Next.js".into(),
                },
                JargonCorrection {
                    from: "post gres".into(),
                    to: "PostgreSQL".into(),
                },
                JargonCorrection {
                    from: "type script".into(),
                    to: "TypeScript".into(),
                },
                JargonCorrection {
                    from: "java script".into(),
                    to: "JavaScript".into(),
                },
                JargonCorrection {
                    from: "web socket".into(),
                    to: "WebSocket".into(),
                },
                JargonCorrection {
                    from: "graph QL".into(),
                    to: "GraphQL".into(),
                },
                JargonCorrection {
                    from: "tail wind".into(),
                    to: "Tailwind".into(),
                },
                JargonCorrection {
                    from: "web pack".into(),
                    to: "Webpack".into(),
                },
            ],
        },
    );

    profiles.insert(
        "embedded".to_string(),
        JargonProfile {
            label: "Embedded Systems".to_string(),
            terms: vec![
                "UART",
                "SPI",
                "I2C",
                "GPIO",
                "RTOS",
                "JTAG",
                "FPGA",
                "ARM",
                "RISC-V",
                "STM32",
                "ESP32",
                "Arduino",
                "Raspberry Pi",
                "PWM",
                "ADC",
                "DAC",
                "DMA",
                "ISR",
                "HAL",
                "PCB",
                "VHDL",
                "Verilog",
                "GDB",
                "OpenOCD",
                "FreeRTOS",
                "Zephyr",
                "PlatformIO",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
            corrections: vec![
                JargonCorrection {
                    from: "I two C".into(),
                    to: "I2C".into(),
                },
                JargonCorrection {
                    from: "risk five".into(),
                    to: "RISC-V".into(),
                },
                JargonCorrection {
                    from: "S T M 32".into(),
                    to: "STM32".into(),
                },
                JargonCorrection {
                    from: "E S P 32".into(),
                    to: "ESP32".into(),
                },
                JargonCorrection {
                    from: "you art".into(),
                    to: "UART".into(),
                },
                JargonCorrection {
                    from: "G P I O".into(),
                    to: "GPIO".into(),
                },
                JargonCorrection {
                    from: "jay tag".into(),
                    to: "JTAG".into(),
                },
            ],
        },
    );

    profiles.insert(
        "data_science".to_string(),
        JargonProfile {
            label: "Data Science & ML".to_string(),
            terms: vec![
                "TensorFlow",
                "PyTorch",
                "NumPy",
                "Pandas",
                "Scikit-learn",
                "Jupyter",
                "Matplotlib",
                "Keras",
                "CUDA",
                "GPU",
                "TPU",
                "CNN",
                "RNN",
                "LSTM",
                "GAN",
                "NLP",
                "BERT",
                "GPT",
                "LLM",
                "RAG",
                "Hugging Face",
                "MLflow",
                "Spark",
                "Hadoop",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
            corrections: vec![
                JargonCorrection {
                    from: "tensor flow".into(),
                    to: "TensorFlow".into(),
                },
                JargonCorrection {
                    from: "pie torch".into(),
                    to: "PyTorch".into(),
                },
                JargonCorrection {
                    from: "num pie".into(),
                    to: "NumPy".into(),
                },
                JargonCorrection {
                    from: "hugging face".into(),
                    to: "Hugging Face".into(),
                },
                JargonCorrection {
                    from: "sick it learn".into(),
                    to: "Scikit-learn".into(),
                },
                JargonCorrection {
                    from: "L L M".into(),
                    to: "LLM".into(),
                },
            ],
        },
    );

    profiles.insert(
        "devops".to_string(),
        JargonProfile {
            label: "DevOps & Cloud".to_string(),
            terms: vec![
                "Terraform",
                "Ansible",
                "Jenkins",
                "GitLab",
                "Prometheus",
                "Grafana",
                "Nginx",
                "Apache",
                "AWS",
                "GCP",
                "Azure",
                "S3",
                "EC2",
                "Lambda",
                "ECS",
                "EKS",
                "Helm",
                "Istio",
                "gRPC",
                "Kafka",
                "RabbitMQ",
                "Elasticsearch",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
            corrections: vec![
                JargonCorrection {
                    from: "engine X".into(),
                    to: "Nginx".into(),
                },
                JargonCorrection {
                    from: "terra form".into(),
                    to: "Terraform".into(),
                },
                JargonCorrection {
                    from: "cube CTL".into(),
                    to: "kubectl".into(),
                },
                JargonCorrection {
                    from: "G R P C".into(),
                    to: "gRPC".into(),
                },
                JargonCorrection {
                    from: "E K S".into(),
                    to: "EKS".into(),
                },
                JargonCorrection {
                    from: "E C S".into(),
                    to: "ECS".into(),
                },
                JargonCorrection {
                    from: "E C two".into(),
                    to: "EC2".into(),
                },
            ],
        },
    );

    profiles.insert(
        "coding".to_string(),
        JargonProfile {
            label: "Coding".to_string(),
            terms: vec![
                "TypeScript",
                "JavaScript",
                "Rust",
                "Python",
                "Go",
                "SQL",
                "PostgreSQL",
                "Redis",
                "Docker",
                "Kubernetes",
                "Git",
                "GitHub",
                "Pull Request",
                "Code Review",
                "Refactor",
                "Lint",
                "CI/CD",
                "API",
                "gRPC",
                "GraphQL",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
            corrections: vec![
                JargonCorrection {
                    from: "type script".into(),
                    to: "TypeScript".into(),
                },
                JargonCorrection {
                    from: "java script".into(),
                    to: "JavaScript".into(),
                },
                JargonCorrection {
                    from: "post gres".into(),
                    to: "PostgreSQL".into(),
                },
                JargonCorrection {
                    from: "G R P C".into(),
                    to: "gRPC".into(),
                },
                JargonCorrection {
                    from: "graph Q L".into(),
                    to: "GraphQL".into(),
                },
                JargonCorrection {
                    from: "pull request".into(),
                    to: "Pull Request".into(),
                },
            ],
        },
    );

    profiles.insert(
        "business".to_string(),
        JargonProfile {
            label: "Business".to_string(),
            terms: vec![
                "Revenue",
                "Gross Margin",
                "Operating Expense",
                "Cash Flow",
                "Forecast",
                "Pipeline",
                "Conversion Rate",
                "Customer Retention",
                "Churn",
                "ARR",
                "MRR",
                "KPI",
                "OKR",
                "Roadmap",
                "Go-to-market",
                "ROI",
                "CAC",
                "LTV",
                "Stakeholder",
                "Quarterly Planning",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
            corrections: vec![
                JargonCorrection {
                    from: "A R R".into(),
                    to: "ARR".into(),
                },
                JargonCorrection {
                    from: "M R R".into(),
                    to: "MRR".into(),
                },
                JargonCorrection {
                    from: "K P I".into(),
                    to: "KPI".into(),
                },
                JargonCorrection {
                    from: "O K R".into(),
                    to: "OKR".into(),
                },
                JargonCorrection {
                    from: "go to market".into(),
                    to: "Go-to-market".into(),
                },
                JargonCorrection {
                    from: "R O I".into(),
                    to: "ROI".into(),
                },
                JargonCorrection {
                    from: "C A C".into(),
                    to: "CAC".into(),
                },
                JargonCorrection {
                    from: "L T V".into(),
                    to: "LTV".into(),
                },
            ],
        },
    );

    profiles.insert(
        "law_enforcement".to_string(),
        JargonProfile {
            label: "Law Enforcement".to_string(),
            terms: vec![
                "Probable Cause",
                "Miranda",
                "Warrant",
                "Search Warrant",
                "Arrest Warrant",
                "BOLO",
                "Dispatch",
                "Patrol",
                "Incident Report",
                "Evidence",
                "Chain of Custody",
                "Body Camera",
                "Use of Force",
                "De-escalation",
                "Detention",
                "Felony",
                "Misdemeanor",
                "Citation",
                "Perimeter",
                "Suspect",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
            corrections: vec![
                JargonCorrection {
                    from: "B O L O".into(),
                    to: "BOLO".into(),
                },
                JargonCorrection {
                    from: "miranda rights".into(),
                    to: "Miranda".into(),
                },
                JargonCorrection {
                    from: "chain of custody".into(),
                    to: "Chain of Custody".into(),
                },
                JargonCorrection {
                    from: "body cam".into(),
                    to: "Body Camera".into(),
                },
                JargonCorrection {
                    from: "use of force".into(),
                    to: "Use of Force".into(),
                },
                JargonCorrection {
                    from: "de escalation".into(),
                    to: "De-escalation".into(),
                },
            ],
        },
    );

    profiles
}

// ============================================================================
// Active Dictionary Computation
// ============================================================================

pub fn compute_active_dictionary(
    settings: &JargonSettings,
    profiles: &HashMap<String, JargonProfile>,
) -> ActiveDictionary {
    // Collect terms: custom first, then profiles in alphabetical order
    let mut terms_map: HashMap<String, String> = HashMap::new();

    // Add custom terms first (they win on casing)
    for term in &settings.custom_terms {
        terms_map.insert(term.to_lowercase(), term.clone());
    }

    // Add profile terms in alphabetical order by profile id
    let mut profile_ids: Vec<&String> = settings
        .enabled_profiles
        .iter()
        .filter(|id| profiles.contains_key(id.as_str()))
        .collect();
    profile_ids.sort();

    for profile_id in &profile_ids {
        if let Some(profile) = profiles.get(profile_id.as_str()) {
            for term in &profile.terms {
                let key = term.to_lowercase();
                // Only insert if not already present (custom terms take priority)
                terms_map.entry(key).or_insert_with(|| term.clone());
            }
        }
    }

    // Build terms list: custom first, then profile terms
    let mut terms = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();

    for term in &settings.custom_terms {
        let key = term.to_lowercase();
        if seen.insert(key.clone()) {
            if let Some(t) = terms_map.get(&key) {
                terms.push(t.clone());
            }
        }
    }
    for profile_id in &profile_ids {
        if let Some(profile) = profiles.get(profile_id.as_str()) {
            for term in &profile.terms {
                let key = term.to_lowercase();
                if seen.insert(key.clone()) {
                    if let Some(t) = terms_map.get(&key) {
                        terms.push(t.clone());
                    }
                }
            }
        }
    }

    // Collect corrections: custom corrections override profile corrections
    let mut corrections_map: HashMap<String, JargonCorrection> = HashMap::new();

    // Add profile corrections first
    for profile_id in &profile_ids {
        if let Some(profile) = profiles.get(profile_id.as_str()) {
            for correction in &profile.corrections {
                corrections_map.insert(correction.from.to_lowercase(), correction.clone());
            }
        }
    }

    // Custom corrections override profile corrections
    for correction in &settings.custom_corrections {
        corrections_map.insert(correction.from.to_lowercase(), correction.clone());
    }

    // Sort corrections longest-phrase-first
    let mut corrections: Vec<JargonCorrection> = corrections_map.into_values().collect();
    corrections.sort_by(|a, b| {
        b.from
            .len()
            .cmp(&a.from.len())
            .then_with(|| a.from.cmp(&b.from))
    });

    ActiveDictionary { terms, corrections }
}

// ============================================================================
// Initial Prompt Builder
// ============================================================================

pub fn build_initial_prompt(dictionary: &ActiveDictionary) -> String {
    if dictionary.terms.is_empty() {
        return String::new();
    }

    let prefix = "Technical dictation. Common terms: ";
    let suffix = ".";
    let max_len = 1000;
    let available = max_len - prefix.len() - suffix.len();

    let mut parts = Vec::new();
    let mut current_len = 0;

    for term in &dictionary.terms {
        let addition = if parts.is_empty() {
            term.len()
        } else {
            term.len() + 2 // ", " separator
        };

        if current_len + addition > available {
            break;
        }

        parts.push(term.as_str());
        current_len += addition;
    }

    if parts.is_empty() {
        return String::new();
    }

    format!("{}{}{}", prefix, parts.join(", "), suffix)
}

// ============================================================================
// Protected Span Masking
// ============================================================================

struct ProtectedSpan {
    placeholder: String,
    original: String,
}

fn mask_protected_spans(text: &str) -> (String, Vec<ProtectedSpan>) {
    let patterns = [
        r"@[\w\-./]+",                         // @tokens like @file.rs
        r"`[^`]+`",                            // backtick code
        r"https?://[^\s]+",                    // URLs
        r"(?:~/|/[\w\-]+(?:/[\w\-.*]+)+)",     // file paths
        r"(?:^|\s)--?[\w\-]+=?(?:[\w\-./]+)?", // CLI flags
    ];

    let combined = patterns.join("|");
    let re = Regex::new(&combined).expect("invalid protected-span regex");

    let mut spans = Vec::new();
    let mut masked = text.to_string();

    // Collect matches in reverse order so replacement indices stay valid
    let matches: Vec<_> = re.find_iter(text).collect();
    for (i, m) in matches.iter().rev().enumerate() {
        let idx = matches.len() - 1 - i;
        let placeholder = format!("\u{27E6}S{}\u{27E7}", idx); // ⟦S0⟧, ⟦S1⟧ ...
        spans.push(ProtectedSpan {
            placeholder: placeholder.clone(),
            original: m.as_str().to_string(),
        });
        masked.replace_range(m.start()..m.end(), &placeholder);
    }

    // Reverse spans so they're in forward order (index 0 first)
    spans.reverse();
    (masked, spans)
}

fn restore_protected_spans(text: &str, spans: &[ProtectedSpan]) -> String {
    let mut result = text.to_string();
    for span in spans {
        result = result.replace(&span.placeholder, &span.original);
    }
    result
}

// ============================================================================
// Correction Application
// ============================================================================

pub fn apply_corrections(text: &str, corrections: &[JargonCorrection]) -> String {
    if corrections.is_empty() || text.is_empty() {
        return text.to_string();
    }

    // Mask protected spans
    let (mut masked, spans) = mask_protected_spans(text);

    // Apply corrections (longest first, already sorted)
    for correction in corrections {
        let pattern = format!(r"(?i)\b{}\b", regex::escape(&correction.from));
        if let Ok(re) = Regex::new(&pattern) {
            masked = re.replace_all(&masked, correction.to.as_str()).to_string();
        }
    }

    // Restore protected spans
    let restored = restore_protected_spans(&masked, &spans);

    // Safety check: verify all placeholders were restored
    for span in &spans {
        if restored.contains(&span.placeholder) {
            warn!(
                "Placeholder {} was not properly restored, returning original text",
                span.placeholder
            );
            return text.to_string();
        }
    }

    restored
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_settings(
        profiles: Vec<&str>,
        terms: Vec<&str>,
        corrections: Vec<(&str, &str)>,
    ) -> JargonSettings {
        JargonSettings {
            enabled_profiles: profiles.into_iter().map(String::from).collect(),
            custom_terms: terms.into_iter().map(String::from).collect(),
            custom_corrections: corrections
                .into_iter()
                .map(|(f, t)| JargonCorrection {
                    from: f.to_string(),
                    to: t.to_string(),
                })
                .collect(),
        }
    }

    #[test]
    fn test_profile_merging() {
        let profiles = builtin_profiles();
        let settings = make_settings(vec!["web_dev", "devops"], vec![], vec![]);
        let dict = compute_active_dictionary(&settings, &profiles);
        // Should have terms from both profiles
        assert!(dict.terms.iter().any(|t| t == "TypeScript"));
        assert!(dict.terms.iter().any(|t| t == "Terraform"));
    }

    #[test]
    fn test_correction_override_priority() {
        let profiles = builtin_profiles();
        // Custom correction overrides the web_dev profile's "next js" -> "Next.js"
        let settings = make_settings(vec!["web_dev"], vec![], vec![("next js", "NextJS")]);
        let dict = compute_active_dictionary(&settings, &profiles);
        let correction = dict
            .corrections
            .iter()
            .find(|c| c.from.eq_ignore_ascii_case("next js"));
        assert!(correction.is_some());
        assert_eq!(correction.unwrap().to, "NextJS");
    }

    #[test]
    fn test_case_insensitive_dedup() {
        let profiles = builtin_profiles();
        // Custom term "typescript" (lowercase) should override profile's "TypeScript" casing
        let settings = make_settings(vec!["web_dev"], vec!["typescript"], vec![]);
        let dict = compute_active_dictionary(&settings, &profiles);
        let ts_terms: Vec<_> = dict
            .terms
            .iter()
            .filter(|t| t.to_lowercase() == "typescript")
            .collect();
        assert_eq!(ts_terms.len(), 1);
        assert_eq!(ts_terms[0], "typescript"); // custom casing wins
    }

    #[test]
    fn test_protected_span_at_refs() {
        let result = apply_corrections(
            "Check @file.rs for type script code",
            &[JargonCorrection {
                from: "type script".into(),
                to: "TypeScript".into(),
            }],
        );
        assert!(result.contains("@file.rs"));
        assert!(result.contains("TypeScript"));
    }

    #[test]
    fn test_protected_span_backticks() {
        let result = apply_corrections(
            "Run `type script build` with type script",
            &[JargonCorrection {
                from: "type script".into(),
                to: "TypeScript".into(),
            }],
        );
        assert!(result.contains("`type script build`"));
        assert!(result.contains("TypeScript"));
    }

    #[test]
    fn test_protected_span_urls() {
        let result = apply_corrections(
            "Visit https://type-script.org for type script docs",
            &[JargonCorrection {
                from: "type script".into(),
                to: "TypeScript".into(),
            }],
        );
        assert!(result.contains("https://type-script.org"));
        assert!(result.contains("TypeScript"));
    }

    #[test]
    fn test_protected_span_paths() {
        let result = apply_corrections(
            "Open /usr/local/bin/app and type script",
            &[JargonCorrection {
                from: "type script".into(),
                to: "TypeScript".into(),
            }],
        );
        assert!(result.contains("/usr/local/bin/app"));
        assert!(result.contains("TypeScript"));
    }

    #[test]
    fn test_protected_span_cli_flags() {
        let result = apply_corrections(
            "Use --verbose and type script",
            &[JargonCorrection {
                from: "type script".into(),
                to: "TypeScript".into(),
            }],
        );
        assert!(result.contains("--verbose"));
        assert!(result.contains("TypeScript"));
    }

    #[test]
    fn test_multi_word_boundary_safety() {
        // "script" alone should not be replaced by a "type script" correction
        let result = apply_corrections(
            "This script is good",
            &[JargonCorrection {
                from: "type script".into(),
                to: "TypeScript".into(),
            }],
        );
        assert_eq!(result, "This script is good");
    }

    #[test]
    fn test_stable_initial_prompt() {
        let profiles = builtin_profiles();
        let settings = make_settings(vec!["web_dev"], vec!["MyCustomTerm"], vec![]);
        let dict = compute_active_dictionary(&settings, &profiles);
        let prompt = build_initial_prompt(&dict);

        assert!(prompt.starts_with("Technical dictation. Common terms: "));
        assert!(prompt.ends_with('.'));
        assert!(prompt.len() <= 1000);
        // Custom terms should come first
        assert!(prompt.find("MyCustomTerm").unwrap() < prompt.find("TypeScript").unwrap());
    }

    #[test]
    fn test_initial_prompt_char_limit() {
        let mut terms = Vec::new();
        for i in 0..200 {
            terms.push(format!("VeryLongTermNumber{}", i));
        }
        let dict = ActiveDictionary {
            terms,
            corrections: vec![],
        };
        let prompt = build_initial_prompt(&dict);
        assert!(prompt.len() <= 1000);
    }

    #[test]
    fn test_longest_first_ordering() {
        let profiles = builtin_profiles();
        let settings = make_settings(vec![], vec![], vec![("E C", "EC"), ("E C two", "EC2")]);
        let dict = compute_active_dictionary(&settings, &profiles);
        // "E C two" should come before "E C" because it's longer
        assert_eq!(dict.corrections[0].from, "E C two");
        assert_eq!(dict.corrections[1].from, "E C");
    }

    #[test]
    fn test_empty_input() {
        let result = apply_corrections(
            "",
            &[JargonCorrection {
                from: "test".into(),
                to: "Test".into(),
            }],
        );
        assert_eq!(result, "");
    }

    #[test]
    fn test_no_corrections() {
        let result = apply_corrections("Hello world", &[]);
        assert_eq!(result, "Hello world");
    }

    #[test]
    fn test_case_insensitive_correction() {
        let result = apply_corrections(
            "I use Type Script and TYPE SCRIPT",
            &[JargonCorrection {
                from: "type script".into(),
                to: "TypeScript".into(),
            }],
        );
        assert_eq!(result, "I use TypeScript and TypeScript");
    }

    #[test]
    fn test_multiple_corrections() {
        let result = apply_corrections(
            "I use type script with next js",
            &[
                JargonCorrection {
                    from: "type script".into(),
                    to: "TypeScript".into(),
                },
                JargonCorrection {
                    from: "next js".into(),
                    to: "Next.js".into(),
                },
            ],
        );
        assert_eq!(result, "I use TypeScript with Next.js");
    }

    #[test]
    fn test_empty_dictionary_prompt() {
        let dict = ActiveDictionary {
            terms: vec![],
            corrections: vec![],
        };
        let prompt = build_initial_prompt(&dict);
        assert!(prompt.is_empty());
    }

    #[test]
    fn test_new_domain_profiles_present() {
        let profiles = builtin_profiles();
        assert!(profiles.contains_key("coding"));
        assert!(profiles.contains_key("business"));
        assert!(profiles.contains_key("law_enforcement"));
    }
}
