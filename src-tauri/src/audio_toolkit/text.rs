use natural::phonetics::soundex;
use once_cell::sync::Lazy;
use regex::Regex;
use strsim::levenshtein;

/// Builds an n-gram string by cleaning and concatenating words
///
/// Strips punctuation from each word, lowercases, and joins without spaces.
/// This allows matching "Charge B" against "ChargeBee".
fn build_ngram(words: &[&str]) -> String {
    words
        .iter()
        .map(|w| {
            w.trim_matches(|c: char| !c.is_alphanumeric())
                .to_lowercase()
        })
        .collect::<Vec<_>>()
        .concat()
}

/// Finds the best matching custom word for a candidate string
///
/// Uses Levenshtein distance and Soundex phonetic matching to find
/// the best match above the given threshold.
///
/// # Arguments
/// * `candidate` - The cleaned/lowercased candidate string to match
/// * `custom_words` - Original custom words (for returning the replacement)
/// * `custom_words_nospace` - Custom words with spaces removed, lowercased (for comparison)
/// * `threshold` - Maximum similarity score to accept
///
/// # Returns
/// The best matching custom word and its score, if any match was found
fn find_best_match<'a>(
    candidate: &str,
    custom_words: &'a [String],
    custom_words_nospace: &[String],
    threshold: f64,
) -> Option<(&'a String, f64)> {
    if candidate.is_empty() || candidate.len() > 50 {
        return None;
    }

    let mut best_match: Option<&String> = None;
    let mut best_score = f64::MAX;

    for (i, custom_word_nospace) in custom_words_nospace.iter().enumerate() {
        // Skip if lengths are too different (optimization + prevents over-matching)
        // Use percentage-based check: max 25% length difference (prevents n-grams from
        // matching significantly shorter custom words, e.g., "openaigpt" vs "openai")
        let len_diff = (candidate.len() as i32 - custom_word_nospace.len() as i32).abs() as f64;
        let max_len = candidate.len().max(custom_word_nospace.len()) as f64;
        let max_allowed_diff = (max_len * 0.25).max(2.0); // At least 2 chars difference allowed
        if len_diff > max_allowed_diff {
            continue;
        }

        // Calculate Levenshtein distance (normalized by length)
        let levenshtein_dist = levenshtein(candidate, custom_word_nospace);
        let max_len = candidate.len().max(custom_word_nospace.len()) as f64;
        let levenshtein_score = if max_len > 0.0 {
            levenshtein_dist as f64 / max_len
        } else {
            1.0
        };

        // Calculate phonetic similarity using Soundex
        let phonetic_match = soundex(candidate, custom_word_nospace);

        // Combine scores: favor phonetic matches, but also consider string similarity
        let combined_score = if phonetic_match {
            levenshtein_score * 0.3 // Give significant boost to phonetic matches
        } else {
            levenshtein_score
        };

        // Accept if the score is good enough (configurable threshold)
        if combined_score < threshold && combined_score < best_score {
            best_match = Some(&custom_words[i]);
            best_score = combined_score;
        }
    }

    best_match.map(|m| (m, best_score))
}

/// Applies custom word corrections to transcribed text using fuzzy matching
///
/// This function corrects words in the input text by finding the best matches
/// from a list of custom words using a combination of:
/// - Levenshtein distance for string similarity
/// - Soundex phonetic matching for pronunciation similarity
/// - N-gram matching for multi-word speech artifacts (e.g., "Charge B" -> "ChargeBee")
///
/// # Arguments
/// * `text` - The input text to correct
/// * `custom_words` - List of custom words to match against
/// * `threshold` - Maximum similarity score to accept (0.0 = exact match, 1.0 = any match)
///
/// # Returns
/// The corrected text with custom words applied
pub fn apply_custom_words(text: &str, custom_words: &[String], threshold: f64) -> String {
    if custom_words.is_empty() {
        return text.to_string();
    }

    // Pre-compute lowercase versions to avoid repeated allocations
    let custom_words_lower: Vec<String> = custom_words.iter().map(|w| w.to_lowercase()).collect();

    // Pre-compute versions with spaces removed for n-gram comparison
    let custom_words_nospace: Vec<String> = custom_words_lower
        .iter()
        .map(|w| w.replace(' ', ""))
        .collect();

    let words: Vec<&str> = text.split_whitespace().collect();
    let mut result = Vec::new();
    let mut i = 0;

    while i < words.len() {
        let mut matched = false;

        // Try n-grams from longest (3) to shortest (1) - greedy matching
        for n in (1..=3).rev() {
            if i + n > words.len() {
                continue;
            }

            let ngram_words = &words[i..i + n];
            let ngram = build_ngram(ngram_words);

            if let Some((replacement, _score)) =
                find_best_match(&ngram, custom_words, &custom_words_nospace, threshold)
            {
                // Extract punctuation from first and last words of the n-gram
                let (prefix, _) = extract_punctuation(ngram_words[0]);
                let (_, suffix) = extract_punctuation(ngram_words[n - 1]);

                // Preserve case from first word
                let corrected = preserve_case_pattern(ngram_words[0], replacement);

                result.push(format!("{}{}{}", prefix, corrected, suffix));
                i += n;
                matched = true;
                break;
            }
        }

        if !matched {
            result.push(words[i].to_string());
            i += 1;
        }
    }

    result.join(" ")
}

/// Preserves the case pattern of the original word when applying a replacement
fn preserve_case_pattern(original: &str, replacement: &str) -> String {
    if original.chars().all(|c| c.is_uppercase()) {
        replacement.to_uppercase()
    } else if original.chars().next().map_or(false, |c| c.is_uppercase()) {
        let mut chars: Vec<char> = replacement.chars().collect();
        if let Some(first_char) = chars.get_mut(0) {
            *first_char = first_char.to_uppercase().next().unwrap_or(*first_char);
        }
        chars.into_iter().collect()
    } else {
        replacement.to_string()
    }
}

/// Extracts punctuation prefix and suffix from a word
fn extract_punctuation(word: &str) -> (&str, &str) {
    let prefix_end = word.chars().take_while(|c| !c.is_alphanumeric()).count();
    let suffix_start = word
        .char_indices()
        .rev()
        .take_while(|(_, c)| !c.is_alphanumeric())
        .count();

    let prefix = if prefix_end > 0 {
        &word[..prefix_end]
    } else {
        ""
    };

    let suffix = if suffix_start > 0 {
        &word[word.len() - suffix_start..]
    } else {
        ""
    };

    (prefix, suffix)
}

/// Cleans segment boundaries by stripping trailing punctuation from each segment,
/// lowercasing everything, and joining into a single run-on sentence.
/// The LLM post-processor (if enabled) can then add proper punctuation and capitalization.
///
/// # Arguments
/// * `segments` - The individual transcription segments
/// * `remaining` - The final transcription of remaining audio after segments
///
/// # Returns
/// A single lowercased string with segment artifacts removed
pub fn clean_segment_boundaries(segments: &[String], remaining: &str) -> String {
    let mut parts: Vec<String> = Vec::new();

    for segment in segments {
        let trimmed = segment
            .trim()
            .trim_end_matches('.')
            .trim_end_matches("...")
            .trim_end_matches('!')
            .trim_end_matches('?')
            .trim_end_matches(',')
            .trim();
        if !trimmed.is_empty() {
            parts.push(trimmed.to_lowercase());
        }
    }

    let remaining_trimmed = remaining
        .trim()
        .trim_end_matches('.')
        .trim_end_matches("...")
        .trim_end_matches('!')
        .trim_end_matches('?')
        .trim_end_matches(',')
        .trim();
    if !remaining_trimmed.is_empty() {
        parts.push(remaining_trimmed.to_lowercase());
    }

    parts.join(" ")
}

/// Filler words to remove from transcriptions
const FILLER_WORDS: &[&str] = &[
    "uh", "um", "uhm", "umm", "uhh", "uhhh", "ah", "eh", "hmm", "hm", "mmm", "mm", "mh", "ha",
    "ehh",
];

static MULTI_SPACE_PATTERN: Lazy<Regex> = Lazy::new(|| Regex::new(r"\s{2,}").unwrap());

/// Collapses repeated 1-2 letter words (3+ repetitions) to a single instance.
/// E.g., "wh wh wh wh" -> "wh", "I I I I" -> "I"
fn collapse_stutters(text: &str) -> String {
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.is_empty() {
        return text.to_string();
    }

    let mut result: Vec<&str> = Vec::new();
    let mut i = 0;

    while i < words.len() {
        let word = words[i];
        let word_lower = word.to_lowercase();

        // Only process 1-2 letter words
        if word_lower.len() <= 2 && word_lower.chars().all(|c| c.is_alphabetic()) {
            // Count consecutive repetitions (case-insensitive)
            let mut count = 1;
            while i + count < words.len() && words[i + count].to_lowercase() == word_lower {
                count += 1;
            }

            // If 3+ repetitions, collapse to single instance
            if count >= 3 {
                result.push(word);
                i += count;
            } else {
                result.push(word);
                i += 1;
            }
        } else {
            result.push(word);
            i += 1;
        }
    }

    result.join(" ")
}

/// Pre-compiled filler word patterns (built lazily)
static FILLER_PATTERNS: Lazy<Vec<Regex>> = Lazy::new(|| {
    FILLER_WORDS
        .iter()
        .map(|word| {
            // Match filler word with word boundaries, optionally followed by comma or period
            Regex::new(&format!(r"(?i)\b{}\b[,.]?", regex::escape(word))).unwrap()
        })
        .collect()
});

/// Known Whisper hallucination phrases that appear when transcribing short or
/// silent audio segments. Only used for whole-output matching.
const HALLUCINATION_PHRASES: &[&str] = &[
    "thank you for watching",
    "thanks for watching",
    "thank you for listening",
    "thanks for listening",
    "please subscribe",
    "like and subscribe",
    "see you next time",
    "see you in the next video",
    "bye bye",
    "bye",
    "thank you",
    "thanks",
    "subtitles by",
    "you",
];

/// Regex patterns for hallucinations that contain variable parts (e.g. URLs).
/// These match the entire output (after trimming/lowercasing).
static HALLUCINATION_REGEXES: Lazy<Vec<Regex>> = Lazy::new(|| {
    vec![
        // Matches various forms of "for more information visit URL" hallucinations,
        // including compound forms like "visit X or visit X for more information"
        Regex::new(r"(?i)^(for more information[,.]?\s*)?(visit|go to)\s+\S+(\s+(or\s+)?(visit|go to)\s+\S+)*(\s+for more information)?[.,]?\s*$").unwrap(),
        // "For more information, visit URL" as prefix
        Regex::new(r"(?i)^for more information[,.]?\s*(visit|go to)\s+\S+[.,]?\s*$").unwrap(),
        // "Subtitles by ..." (various attribution patterns)
        Regex::new(r"(?i)^subtitles\s+(by|provided by|created by)\s+.*$").unwrap(),
    ]
});

/// Checks whether the entire transcription is a known Whisper hallucination.
///
/// Returns `true` if the trimmed, punctuation-stripped, lowercased text matches
/// any entry in `HALLUCINATION_PHRASES` exactly, or matches a hallucination regex pattern.
fn is_hallucination(text: &str) -> bool {
    let stripped: String = text
        .trim()
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect();
    let normalized = stripped.trim().to_lowercase();
    if normalized.is_empty() {
        return false;
    }

    // Check exact phrase matches (punctuation-stripped)
    if HALLUCINATION_PHRASES
        .iter()
        .any(|phrase| normalized == *phrase)
    {
        return true;
    }

    // Check regex patterns (on trimmed original, preserving punctuation for URL matching)
    let trimmed = text.trim();
    HALLUCINATION_REGEXES.iter().any(|re| re.is_match(trimmed))
}

/// Filters transcription output by removing filler words, stutter artifacts,
/// and known Whisper hallucinations.
///
/// This function cleans up raw transcription text by:
/// 1. Removing filler words (uh, um, hmm, etc.)
/// 2. Collapsing repeated 1-2 letter stutters (e.g., "wh wh wh" -> "wh")
/// 3. Cleaning up excess whitespace
/// 4. Discarding the entire output if it is a known hallucination phrase
///
/// # Arguments
/// * `text` - The raw transcription text to filter
///
/// # Returns
/// The filtered text with filler words and stutters removed
pub fn filter_transcription_output(text: &str) -> String {
    let mut filtered = text.to_string();

    // Remove filler words
    for pattern in FILLER_PATTERNS.iter() {
        filtered = pattern.replace_all(&filtered, "").to_string();
    }

    // Collapse repeated 1-2 letter words (stutter artifacts like "wh wh wh wh")
    filtered = collapse_stutters(&filtered);

    // Clean up multiple spaces to single space
    filtered = MULTI_SPACE_PATTERN.replace_all(&filtered, " ").to_string();

    // Trim leading/trailing whitespace
    filtered = filtered.trim().to_string();

    // Discard entire output if it is a known hallucination phrase
    if is_hallucination(&filtered) {
        return String::new();
    }

    filtered
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_custom_words_exact_match() {
        let text = "hello world";
        let custom_words = vec!["Hello".to_string(), "World".to_string()];
        let result = apply_custom_words(text, &custom_words, 0.5);
        assert_eq!(result, "Hello World");
    }

    #[test]
    fn test_apply_custom_words_fuzzy_match() {
        let text = "helo wrold";
        let custom_words = vec!["hello".to_string(), "world".to_string()];
        let result = apply_custom_words(text, &custom_words, 0.5);
        assert_eq!(result, "hello world");
    }

    #[test]
    fn test_preserve_case_pattern() {
        assert_eq!(preserve_case_pattern("HELLO", "world"), "WORLD");
        assert_eq!(preserve_case_pattern("Hello", "world"), "World");
        assert_eq!(preserve_case_pattern("hello", "WORLD"), "WORLD");
    }

    #[test]
    fn test_extract_punctuation() {
        assert_eq!(extract_punctuation("hello"), ("", ""));
        assert_eq!(extract_punctuation("!hello?"), ("!", "?"));
        assert_eq!(extract_punctuation("...hello..."), ("...", "..."));
    }

    #[test]
    fn test_empty_custom_words() {
        let text = "hello world";
        let custom_words = vec![];
        let result = apply_custom_words(text, &custom_words, 0.5);
        assert_eq!(result, "hello world");
    }

    #[test]
    fn test_filter_filler_words() {
        let text = "So um I was thinking uh about this";
        let result = filter_transcription_output(text);
        assert_eq!(result, "So I was thinking about this");
    }

    #[test]
    fn test_filter_filler_words_case_insensitive() {
        let text = "UM this is UH a test";
        let result = filter_transcription_output(text);
        assert_eq!(result, "this is a test");
    }

    #[test]
    fn test_filter_filler_words_with_punctuation() {
        let text = "Well, um, I think, uh. that's right";
        let result = filter_transcription_output(text);
        assert_eq!(result, "Well, I think, that's right");
    }

    #[test]
    fn test_filter_cleans_whitespace() {
        let text = "Hello    world   test";
        let result = filter_transcription_output(text);
        assert_eq!(result, "Hello world test");
    }

    #[test]
    fn test_filter_trims() {
        let text = "  Hello world  ";
        let result = filter_transcription_output(text);
        assert_eq!(result, "Hello world");
    }

    #[test]
    fn test_filter_combined() {
        let text = "  Um, so I was, uh, thinking about this  ";
        let result = filter_transcription_output(text);
        assert_eq!(result, "so I was, thinking about this");
    }

    #[test]
    fn test_filter_preserves_valid_text() {
        let text = "This is a completely normal sentence.";
        let result = filter_transcription_output(text);
        assert_eq!(result, "This is a completely normal sentence.");
    }

    #[test]
    fn test_filter_stutter_collapse() {
        let text = "w wh wh wh wh wh wh wh wh wh why";
        let result = filter_transcription_output(text);
        assert_eq!(result, "w wh why");
    }

    #[test]
    fn test_filter_stutter_short_words() {
        let text = "I I I I think so so so so";
        let result = filter_transcription_output(text);
        assert_eq!(result, "I think so");
    }

    #[test]
    fn test_filter_stutter_mixed_case() {
        let text = "No NO no NO no";
        let result = filter_transcription_output(text);
        assert_eq!(result, "No");
    }

    #[test]
    fn test_filter_stutter_preserves_two_repetitions() {
        let text = "no no is fine";
        let result = filter_transcription_output(text);
        assert_eq!(result, "no no is fine");
    }

    #[test]
    fn test_apply_custom_words_ngram_two_words() {
        let text = "il cui nome è Charge B, che permette";
        let custom_words = vec!["ChargeBee".to_string()];
        let result = apply_custom_words(text, &custom_words, 0.5);
        assert!(result.contains("ChargeBee,"));
        assert!(!result.contains("Charge B"));
    }

    #[test]
    fn test_apply_custom_words_ngram_three_words() {
        let text = "use Chat G P T for this";
        let custom_words = vec!["ChatGPT".to_string()];
        let result = apply_custom_words(text, &custom_words, 0.5);
        assert!(result.contains("ChatGPT"));
    }

    #[test]
    fn test_apply_custom_words_prefers_longer_ngram() {
        let text = "Open AI GPT model";
        let custom_words = vec!["OpenAI".to_string(), "GPT".to_string()];
        let result = apply_custom_words(text, &custom_words, 0.5);
        assert_eq!(result, "OpenAI GPT model");
    }

    #[test]
    fn test_apply_custom_words_ngram_preserves_case() {
        let text = "CHARGE B is great";
        let custom_words = vec!["ChargeBee".to_string()];
        let result = apply_custom_words(text, &custom_words, 0.5);
        assert!(result.contains("CHARGEBEE"));
    }

    #[test]
    fn test_apply_custom_words_ngram_with_spaces_in_custom() {
        // Custom word with space should also match against split words
        let text = "using Mac Book Pro";
        let custom_words = vec!["MacBook Pro".to_string()];
        let result = apply_custom_words(text, &custom_words, 0.5);
        assert!(result.contains("MacBook"));
    }

    #[test]
    fn test_apply_custom_words_trailing_number_not_doubled() {
        // Verify that trailing non-alpha chars (like numbers) aren't double-counted
        // between build_ngram stripping them and extract_punctuation capturing them
        let text = "use GPT4 for this";
        let custom_words = vec!["GPT-4".to_string()];
        let result = apply_custom_words(text, &custom_words, 0.5);
        // Should NOT produce "GPT-44" (double-counting the trailing 4)
        assert!(
            !result.contains("GPT-44"),
            "got double-counted result: {}",
            result
        );
    }

    #[test]
    fn test_hallucination_exact_match() {
        assert_eq!(filter_transcription_output("Thank you for watching"), "");
        assert_eq!(filter_transcription_output("bye"), "");
        assert_eq!(filter_transcription_output("you"), "");
    }

    #[test]
    fn test_hallucination_case_insensitive() {
        assert_eq!(filter_transcription_output("THANK YOU FOR WATCHING"), "");
        assert_eq!(filter_transcription_output("Thank You"), "");
        assert_eq!(filter_transcription_output("Please Subscribe"), "");
    }

    #[test]
    fn test_hallucination_with_trailing_punctuation() {
        assert_eq!(filter_transcription_output("Thank you for watching."), "");
        assert_eq!(filter_transcription_output("Bye bye!"), "");
        assert_eq!(filter_transcription_output("Thanks..."), "");
        assert_eq!(filter_transcription_output("See you next time!"), "");
    }

    #[test]
    fn test_hallucination_url_patterns() {
        assert_eq!(
            filter_transcription_output("For more information, visit www.microsoft.com"),
            ""
        );
        assert_eq!(
            filter_transcription_output(
                "For more information, visit www.microsoft.com or visit www.microsoft.com for more information."
            ),
            ""
        );
        assert_eq!(
            filter_transcription_output("Visit www.example.org for more information."),
            ""
        );
        assert_eq!(
            filter_transcription_output("Subtitles by the Amara.org community"),
            ""
        );
    }

    #[test]
    fn test_clean_segment_boundaries_basic() {
        let segments = vec![
            "So I'm trying out.".to_string(),
            "With parakeet instead of Whisper.".to_string(),
            "Because it seems to have better.".to_string(),
        ];
        let result = clean_segment_boundaries(&segments, "Who cares?");
        assert_eq!(
            result,
            "so i'm trying out with parakeet instead of whisper because it seems to have better who cares"
        );
    }

    #[test]
    fn test_clean_segment_boundaries_ellipsis() {
        let segments = vec![
            "And see if that...".to_string(),
            "It starts to collapse.".to_string(),
        ];
        let result = clean_segment_boundaries(&segments, "");
        assert_eq!(result, "and see if that it starts to collapse");
    }

    #[test]
    fn test_clean_segment_boundaries_empty_segments() {
        let segments: Vec<String> = vec![];
        let result = clean_segment_boundaries(&segments, "Just the remaining text.");
        assert_eq!(result, "just the remaining text");
    }

    #[test]
    fn test_clean_segment_boundaries_no_remaining() {
        let segments = vec!["Hello world.".to_string(), "Goodbye.".to_string()];
        let result = clean_segment_boundaries(&segments, "");
        assert_eq!(result, "hello world goodbye");
    }

    #[test]
    fn test_hallucination_does_not_filter_legitimate_text() {
        // Text that contains hallucination phrases as substrings should NOT be filtered
        let result =
            filter_transcription_output("Thank you for watching the demo, now let me explain");
        assert!(!result.is_empty());

        let result = filter_transcription_output("I want to say thank you for the help");
        assert!(!result.is_empty());

        let result = filter_transcription_output("Please subscribe to the newsletter for updates");
        assert!(!result.is_empty());

        let result =
            filter_transcription_output("See you next time we discuss this topic in detail");
        assert!(!result.is_empty());
    }
}
