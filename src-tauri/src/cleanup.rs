use anyhow::{Context, Result};
use log::{debug, info, warn};
use ort::session::Session;
use ort::value::Tensor;
use regex::Regex;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokenizers::Tokenizer;

// ============================================================================
// TextCleanupProvider trait — abstraction for local and API-based cleanup
// ============================================================================

/// Trait for text cleanup providers. Implementations can be local (T5 ONNX)
/// or remote (API-based). The pipeline in actions.rs uses this trait so that
/// any provider can be swapped in.
#[async_trait::async_trait]
pub trait TextCleanupProvider: Send + Sync {
    /// Clean the given text. Implementations should fall back to returning the
    /// original text on any non-fatal error.
    async fn cleanup(&self, text: &str) -> Result<String>;

    /// Human-readable name for logging.
    fn name(&self) -> &str;
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
        let placeholder = format!("\u{27E6}P{}\u{27E7}", idx); // ⟦P0⟧, ⟦P1⟧ ...
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
// Cleanup Session (encoder + decoder + tokenizer)
// ============================================================================

struct CleanupSession {
    encoder: Session,
    decoder: Session,
    tokenizer: Tokenizer,
}

// ============================================================================
// CleanupManager — local T5 ONNX provider
// ============================================================================

pub struct CleanupManager {
    session: Arc<Mutex<Option<CleanupSession>>>,
    model_dir: PathBuf,
}

impl CleanupManager {
    pub fn new(app_data_dir: PathBuf) -> Self {
        let model_dir = app_data_dir.join("models").join("flan-t5-small-onnx");
        Self {
            session: Arc::new(Mutex::new(None)),
            model_dir,
        }
    }

    /// Download model files if they don't exist yet.
    async fn ensure_models_downloaded(&self) -> Result<()> {
        std::fs::create_dir_all(&self.model_dir)?;

        let base_url = "https://huggingface.co/nickmuchi/flan-t5-small-onnx/resolve/main";
        let files = [
            (
                "encoder_model.onnx",
                format!("{}/encoder_model.onnx", base_url),
            ),
            (
                "decoder_model.onnx",
                format!("{}/decoder_model.onnx", base_url),
            ),
            ("tokenizer.json", format!("{}/tokenizer.json", base_url)),
        ];

        let client = reqwest::Client::new();
        for (filename, url) in &files {
            let path = self.model_dir.join(filename);
            if path.exists() {
                debug!("Cleanup model file already exists: {}", filename);
                continue;
            }

            info!("Downloading cleanup model file: {} from {}", filename, url);
            let response = client
                .get(url)
                .send()
                .await
                .with_context(|| format!("Failed to download {}", filename))?;

            if !response.status().is_success() {
                anyhow::bail!(
                    "Failed to download {}: HTTP {}",
                    filename,
                    response.status()
                );
            }

            let bytes = response
                .bytes()
                .await
                .with_context(|| format!("Failed to read bytes for {}", filename))?;

            let tmp_path = path.with_extension("tmp");
            std::fs::write(&tmp_path, &bytes)
                .with_context(|| format!("Failed to write {}", filename))?;
            std::fs::rename(&tmp_path, &path)
                .with_context(|| format!("Failed to rename {}", filename))?;
            info!(
                "Downloaded cleanup model file: {} ({} bytes)",
                filename,
                bytes.len()
            );
        }

        Ok(())
    }

    /// Load the ONNX sessions + tokenizer if not already loaded.
    fn ensure_session_loaded(&self) -> Result<()> {
        let mut guard = self.session.lock().map_err(|e| anyhow::anyhow!("{}", e))?;
        if guard.is_some() {
            return Ok(());
        }

        let encoder_path = self.model_dir.join("encoder_model.onnx");
        let decoder_path = self.model_dir.join("decoder_model.onnx");
        let tokenizer_path = self.model_dir.join("tokenizer.json");

        if !encoder_path.exists() || !decoder_path.exists() || !tokenizer_path.exists() {
            anyhow::bail!("Cleanup model files not found. Download required.");
        }

        info!("Loading cleanup T5 sessions...");
        let encoder = Session::builder()
            .with_context(|| "Failed to create encoder session builder")?
            .commit_from_file(&encoder_path)
            .with_context(|| "Failed to load encoder ONNX model")?;

        let decoder = Session::builder()
            .with_context(|| "Failed to create decoder session builder")?
            .commit_from_file(&decoder_path)
            .with_context(|| "Failed to load decoder ONNX model")?;

        let tokenizer = Tokenizer::from_file(&tokenizer_path)
            .map_err(|e| anyhow::anyhow!("Failed to load tokenizer: {}", e))?;

        info!("Cleanup T5 sessions loaded successfully");
        *guard = Some(CleanupSession {
            encoder,
            decoder,
            tokenizer,
        });

        Ok(())
    }

    /// Run T5 inference on a single text segment.
    fn run_t5(&self, input_text: &str) -> Result<String> {
        let mut guard = self.session.lock().map_err(|e| anyhow::anyhow!("{}", e))?;
        let session = guard
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("Session not loaded"))?;

        // Tokenize input
        let encoding = session
            .tokenizer
            .encode(input_text, false)
            .map_err(|e| anyhow::anyhow!("Tokenization failed: {}", e))?;

        let input_ids: Vec<i64> = encoding.get_ids().iter().map(|&id| id as i64).collect();
        let attention_mask: Vec<i64> = encoding
            .get_attention_mask()
            .iter()
            .map(|&m| m as i64)
            .collect();

        let seq_len = input_ids.len();

        // Create ort Tensors for encoder input
        let input_ids_tensor = Tensor::from_array((vec![1i64, seq_len as i64], input_ids))?;
        let attention_mask_tensor =
            Tensor::from_array((vec![1i64, seq_len as i64], attention_mask))?;

        // Run encoder
        let encoder_outputs = session.encoder.run(ort::inputs![
            "input_ids" => input_ids_tensor,
            "attention_mask" => attention_mask_tensor,
        ])?;

        let encoder_hidden_value = &encoder_outputs["last_hidden_state"];
        let (enc_shape, enc_data) = encoder_hidden_value.try_extract_tensor::<f32>()?;
        let enc_dims: Vec<i64> = enc_shape.iter().map(|&d| d as i64).collect();
        let _hidden_size = enc_dims.last().copied().unwrap_or(0) as usize;

        // Greedy autoregressive decoding
        let pad_token_id: i64 = 0;
        let eos_token_id: i64 = 1;
        let max_output_tokens: usize = 512;

        let mut generated_ids: Vec<i64> = vec![pad_token_id]; // decoder_start_token = pad

        for _ in 0..max_output_tokens {
            let dec_len = generated_ids.len();

            let decoder_input_tensor =
                Tensor::from_array((vec![1i64, dec_len as i64], generated_ids.clone()))?;

            let enc_attn_tensor =
                Tensor::from_array((vec![1i64, seq_len as i64], vec![1i64; seq_len]))?;

            // Reconstruct encoder hidden states tensor for decoder
            let enc_hidden_tensor = Tensor::from_array((enc_dims.clone(), enc_data.to_vec()))?;

            let decoder_outputs = session.decoder.run(ort::inputs![
                "input_ids" => decoder_input_tensor,
                "encoder_attention_mask" => enc_attn_tensor,
                "encoder_hidden_states" => enc_hidden_tensor,
            ])?;

            let logits_value = &decoder_outputs["logits"];
            let (logits_shape, logits_data) = logits_value.try_extract_tensor::<f32>()?;

            // logits shape: [1, dec_len, vocab_size]
            let vocab_size = *logits_shape.last().unwrap_or(&0) as usize;
            let last_pos = dec_len - 1;

            // Argmax over vocabulary for the last position
            let offset = last_pos * vocab_size;
            let mut best_id: i64 = 0;
            let mut best_score = f32::NEG_INFINITY;
            for v in 0..vocab_size {
                let score = logits_data[offset + v];
                if score > best_score {
                    best_score = score;
                    best_id = v as i64;
                }
            }

            if best_id == eos_token_id {
                break;
            }

            generated_ids.push(best_id);
        }

        // Decode output tokens (skip the initial pad token)
        let output_ids: Vec<u32> = generated_ids[1..].iter().map(|&id| id as u32).collect();

        let decoded = session
            .tokenizer
            .decode(&output_ids, true)
            .map_err(|e| anyhow::anyhow!("Decoding failed: {}", e))?;

        Ok(decoded)
    }

    /// Split text at sentence boundaries for better T5 quality.
    fn split_sentences(text: &str) -> Vec<&str> {
        // Rust regex doesn't support lookbehind, so we manually find split points
        let re = Regex::new(r"[.!?]\s+").expect("sentence split regex");
        let mut segments = Vec::new();
        let mut last = 0;
        for m in re.find_iter(text) {
            // Include the punctuation in the segment (split after the punctuation, before whitespace)
            let split_at = m.start() + 1; // after the punctuation char
            if split_at < text.len() {
                segments.push(&text[last..split_at]);
                // Skip whitespace
                last = m.end();
            }
        }
        if last < text.len() {
            segments.push(&text[last..]);
        }
        if segments.is_empty() {
            segments.push(text);
        }
        segments
    }
}

#[async_trait::async_trait]
impl TextCleanupProvider for CleanupManager {
    fn name(&self) -> &str {
        "flan-t5-small (local)"
    }

    async fn cleanup(&self, text: &str) -> Result<String> {
        if text.trim().is_empty() {
            return Ok(text.to_string());
        }

        // Download models if needed
        self.ensure_models_downloaded().await?;

        // Load sessions if needed
        self.ensure_session_loaded()?;

        // Mask protected spans
        let (masked_text, spans) = mask_protected_spans(text);

        // Split into sentences for long texts
        let segments = if masked_text.len() > 100 {
            Self::split_sentences(&masked_text)
        } else {
            vec![masked_text.as_str()]
        };

        let mut cleaned_parts = Vec::new();
        for segment in &segments {
            let trimmed = segment.trim();
            if trimmed.is_empty() {
                cleaned_parts.push(String::new());
                continue;
            }

            let prompt = format!(
                "Fix punctuation and capitalization, remove filler words, and keep meaning. \
                 Do not rewrite placeholders like \u{27E6}P0\u{27E7}. Text: {}",
                trimmed
            );

            match self.run_t5(&prompt) {
                Ok(cleaned) => {
                    cleaned_parts.push(cleaned);
                }
                Err(e) => {
                    warn!("T5 cleanup failed for segment, using original: {}", e);
                    cleaned_parts.push(trimmed.to_string());
                }
            }
        }

        let joined = cleaned_parts.join(" ");

        // Restore protected spans
        let restored = restore_protected_spans(&joined, &spans);

        // Guardrails
        if !validate_cleanup(text, &restored, &spans) {
            warn!("Cleanup guardrails failed, falling back to original text");
            return Ok(text.to_string());
        }

        Ok(restored)
    }
}

// ============================================================================
// Guardrails
// ============================================================================

fn validate_cleanup(original: &str, cleaned: &str, spans: &[ProtectedSpan]) -> bool {
    // Non-empty check
    if cleaned.trim().is_empty() {
        warn!("Cleanup produced empty text");
        return false;
    }

    // Length ratio check (0.5x to 2.0x)
    let orig_len = original.len() as f64;
    let clean_len = cleaned.len() as f64;
    if orig_len > 0.0 {
        let ratio = clean_len / orig_len;
        if ratio < 0.5 || ratio > 2.0 {
            warn!(
                "Cleanup length ratio out of bounds: {:.2} (original: {}, cleaned: {})",
                ratio,
                original.len(),
                cleaned.len()
            );
            return false;
        }
    }

    // Verify all placeholders were restored (none should remain)
    for span in spans {
        if cleaned.contains(&span.placeholder) {
            warn!(
                "Placeholder {} was not properly restored in cleaned text",
                span.placeholder
            );
            return false;
        }
    }

    true
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_and_restore_at_tokens() {
        let text = "Check @file.rs and @other/path for details";
        let (masked, spans) = mask_protected_spans(text);
        assert!(!masked.contains("@file.rs"));
        assert!(!masked.contains("@other/path"));
        let restored = restore_protected_spans(&masked, &spans);
        assert_eq!(restored, text);
    }

    #[test]
    fn test_mask_and_restore_backtick_code() {
        let text = "Run `cargo build` and `bun install`";
        let (masked, spans) = mask_protected_spans(text);
        assert!(!masked.contains("`cargo build`"));
        let restored = restore_protected_spans(&masked, &spans);
        assert_eq!(restored, text);
    }

    #[test]
    fn test_mask_and_restore_urls() {
        let text = "Visit https://example.com/path?q=1 for info";
        let (masked, spans) = mask_protected_spans(text);
        assert!(!masked.contains("https://example.com"));
        let restored = restore_protected_spans(&masked, &spans);
        assert_eq!(restored, text);
    }

    #[test]
    fn test_mask_and_restore_paths() {
        let text = "Open /usr/local/bin/app and ~/Documents/file.txt";
        let (masked, spans) = mask_protected_spans(text);
        assert!(!masked.contains("/usr/local/bin/app"));
        assert!(!masked.contains("~/Documents/file.txt"));
        let restored = restore_protected_spans(&masked, &spans);
        assert_eq!(restored, text);
    }

    #[test]
    fn test_mask_and_restore_cli_flags() {
        let text = "Use --verbose and -o=output.txt";
        let (masked, spans) = mask_protected_spans(text);
        assert!(!masked.contains("--verbose"));
        assert!(!masked.contains("-o=output.txt"));
        let restored = restore_protected_spans(&masked, &spans);
        assert_eq!(restored, text);
    }

    #[test]
    fn test_validate_cleanup_empty() {
        assert!(!validate_cleanup("hello world", "", &[]));
        assert!(!validate_cleanup("hello world", "   ", &[]));
    }

    #[test]
    fn test_validate_cleanup_length_ratio() {
        // Too short
        assert!(!validate_cleanup(
            "this is a relatively long sentence here",
            "hi",
            &[]
        ));
        // Too long (3x)
        let original = "short";
        let cleaned = "this is way too long for the original short text here";
        assert!(!validate_cleanup(original, cleaned, &[]));
    }

    #[test]
    fn test_validate_cleanup_unreplaced_placeholder() {
        let spans = vec![ProtectedSpan {
            placeholder: "\u{27E6}P0\u{27E7}".to_string(),
            original: "@file.rs".to_string(),
        }];
        assert!(!validate_cleanup(
            "Check @file.rs",
            "Check \u{27E6}P0\u{27E7}",
            &spans
        ));
    }

    #[test]
    fn test_validate_cleanup_ok() {
        assert!(validate_cleanup("hello world", "Hello world.", &[]));
    }

    #[test]
    fn test_sentence_splitting() {
        let text = "Hello world. This is a test! Another sentence? Final one.";
        let segments = CleanupManager::split_sentences(text);
        assert_eq!(segments.len(), 4);
    }
}
