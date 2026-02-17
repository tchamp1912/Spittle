use log::debug;
use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use strsim::{damerau_levenshtein, levenshtein};
use walkdir::WalkDir;

#[derive(Debug, Clone)]
struct AtToken {
    token: String,
    start: usize,
    end: usize,
}

/// Parse @tokens from text. Supports `@filename` and `@"file name with spaces"`.
/// Skips email-like patterns where the char before `@` is alphanumeric.
fn parse_at_tokens(text: &str) -> Vec<AtToken> {
    let re = Regex::new(r#"@([a-zA-Z0-9_\-./]+)|@"([^"]+)""#).unwrap();
    let command_re =
        Regex::new(r#"(?i)\b(at|include|reference|for|file)\s+(?:file\s+)?([^\n,;:!?]+)"#).unwrap();
    let mut tokens = Vec::new();

    for cap in re.captures_iter(text) {
        let full_match = cap.get(0).unwrap();
        let start = full_match.start();

        // Skip if the char before `@` is alphanumeric (email pattern)
        if start > 0 {
            let prev_char = text.as_bytes()[start - 1];
            if prev_char.is_ascii_alphanumeric() || prev_char == b'_' {
                continue;
            }
        }

        let token_value = if let Some(unquoted) = cap.get(1) {
            normalize_token(unquoted.as_str(), false)
        } else if let Some(quoted) = cap.get(2) {
            // Keep quoted tokens exact except whitespace trim.
            quoted.as_str().trim().to_string()
        } else {
            String::new()
        };

        if !token_value.is_empty() {
            tokens.push(AtToken {
                token: token_value,
                start,
                end: full_match.end(),
            });
        }
    }

    // Spoken command aliases:
    // - "at file auth dot ts"
    // - "include file src slash lib dot rs"
    // - "reference auth.ts"
    // - "for main dot rs"
    // - "file pipeline.rs"
    for cap in command_re.captures_iter(text) {
        let full_match = cap.get(0).unwrap();
        let start = full_match.start();
        let trigger = cap
            .get(1)
            .map(|m| m.as_str().to_ascii_lowercase())
            .unwrap_or_default();
        let raw = cap.get(2).map(|m| m.as_str()).unwrap_or("").trim();
        if raw.contains('@') {
            continue;
        }
        // "for" is a softer alias and should only trigger on file-like phrases.
        if trigger == "for" && !looks_file_like_speech(raw) {
            continue;
        }
        let token_value = normalize_token(raw, true);
        if !token_value.is_empty()
            && (is_file_like_token(&token_value) || looks_bare_spoken_file_alias(&token_value))
        {
            tokens.push(AtToken {
                token: token_value,
                start,
                end: full_match.end(),
            });
        }
    }

    tokens
}

fn normalize_token(raw: &str, spoken_alias: bool) -> String {
    let mut s = raw.trim().to_string();
    if spoken_alias {
        s = s
            .replace(" dot ", ".")
            .replace(" slash ", "/")
            .replace(" backslash ", "/")
            .replace(" underscore ", "_")
            .replace(" hyphen ", "-")
            .replace(" dash ", "-");

        // Transcription often splits extensions: "trade. r s" -> "trade.rs"
        for (pattern, replacement) in SPOKEN_SPLIT_EXTENSION_PATTERNS.iter() {
            s = pattern.replace_all(&s, *replacement).to_string();
        }

        // If speech includes extra phrase text after the extension, trim it.
        if let Some(caps) = TRAILING_AFTER_EXTENSION_RE.captures(&s) {
            if let Some(m) = caps.get(1) {
                s = m.as_str().to_string();
            }
        }
    }
    s = s
        .trim_matches(|c: char| c == '"' || c == '\'' || c == '`' || c.is_whitespace())
        .to_string();
    // Trim sentence punctuation for unquoted / spoken forms.
    s.trim_end_matches(|c: char| ".,;:!?)]}".contains(c))
        .to_string()
}

fn is_file_like_token(token: &str) -> bool {
    token.contains('/') || token.contains('.')
}

fn looks_bare_spoken_file_alias(token: &str) -> bool {
    if token.is_empty() || token.contains('/') || token.contains('.') {
        return false;
    }
    if token.split_whitespace().count() != 1 {
        return false;
    }
    token
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

fn looks_file_like_speech(raw: &str) -> bool {
    let lower = raw.to_ascii_lowercase();
    lower.contains(" dot ")
        || lower.contains('.')
        || lower.contains(" slash ")
        || lower.contains(" backslash ")
        || lower.contains('/')
}

/// Extract lowercase words from a string by splitting on camelCase boundaries,
/// underscores, hyphens, spaces, and dots (preserving the extension separately).
fn normalize_to_words(s: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut current = String::new();

    for ch in s.chars() {
        if ch == '_' || ch == '-' || ch == ' ' || ch == '.' {
            if !current.is_empty() {
                words.push(current.to_lowercase());
                current.clear();
            }
        } else if ch.is_uppercase()
            && !current.is_empty()
            && current
                .chars()
                .last()
                .map(|c| c.is_lowercase())
                .unwrap_or(false)
        {
            // camelCase boundary (but keep consecutive ALL-CAPS runs together)
            words.push(current.to_lowercase());
            current.clear();
            current.push(ch);
        } else {
            current.push(ch);
        }
    }
    if !current.is_empty() {
        words.push(current.to_lowercase());
    }

    words
}

fn words_close_enough(token: &str, candidate: &str) -> bool {
    if token.eq_ignore_ascii_case(candidate) {
        return true;
    }
    let t = token.to_ascii_lowercase();
    let c = candidate.to_ascii_lowercase();
    let d_damerau = damerau_levenshtein(&t, &c);
    if d_damerau <= 1 {
        return true;
    }
    let d = levenshtein(&t, &c);
    d <= 1 || (d == 2 && t.len().max(c.len()) >= 6)
}

/// Check if the normalized words from a token match a filename's stem words.
/// The token words must match all stem words in order for it to be considered a match.
fn fuzzy_basename_match(token: &str, filename: &str) -> bool {
    // Split filename into stem and extension
    let (stem, _ext) = match filename.rsplit_once('.') {
        Some((s, e)) => (s, Some(e)),
        None => (filename, None),
    };

    let token_words = normalize_to_words(token);
    let stem_words = normalize_to_words(stem);

    if token_words.is_empty() || stem_words.is_empty() {
        return false;
    }

    if token_words.len() != stem_words.len() {
        return false;
    }

    token_words
        .iter()
        .zip(stem_words.iter())
        .all(|(t, s)| words_close_enough(t, s))
}

fn extension_matches(token_ext: &str, file_ext: &str) -> bool {
    if file_ext.is_empty() {
        return false;
    }
    if file_ext.eq_ignore_ascii_case(token_ext) {
        return true;
    }

    // For short extensions (e.g. rs/ts/js), require exact match to avoid
    // false positives like ts -> rs.
    if token_ext.len() < 3 || file_ext.len() < 3 {
        return false;
    }

    // Accept minor transcription misspellings like "tomal" -> "toml".
    let token_l = token_ext.to_ascii_lowercase();
    let file_l = file_ext.to_ascii_lowercase();
    let distance = levenshtein(&token_l, &file_l);
    distance <= 1 || (distance == 2 && token_l.len().abs_diff(file_l.len()) <= 1)
}

fn fuzzy_path_match(token: &str, workspace_root: &Path, candidate: &Path) -> bool {
    let rel = match candidate.strip_prefix(workspace_root) {
        Ok(p) => p,
        Err(_) => return false,
    };

    let token_parts: Vec<&str> = token.split('/').filter(|s| !s.is_empty()).collect();
    let candidate_parts: Vec<String> = rel
        .iter()
        .map(|s| s.to_string_lossy().to_string())
        .collect();

    if token_parts.is_empty() || token_parts.len() != candidate_parts.len() {
        return false;
    }

    let last = token_parts.len() - 1;

    // Directory segments: typo-tolerant word matching
    for i in 0..last {
        if !fuzzy_basename_match(token_parts[i], &candidate_parts[i]) {
            return false;
        }
    }

    // File segment: handle extension and stem separately
    let token_file = token_parts[last];
    let candidate_file = &candidate_parts[last];
    let (candidate_stem, candidate_ext) = match candidate_file.rsplit_once('.') {
        Some((s, e)) => (s, Some(e)),
        None => (candidate_file.as_str(), None),
    };

    let (token_stem, token_ext) = match token_file.rsplit_once('.') {
        Some((s, e)) if !e.contains(' ') && e.len() <= 10 => (s, Some(e)),
        _ => (token_file, None),
    };

    if let Some(ext) = token_ext {
        let file_ext = candidate_ext.unwrap_or("");
        if !extension_matches(ext, file_ext) {
            return false;
        }
    }

    fuzzy_basename_match(token_stem, candidate_stem)
}

/// Resolve a token to a file path within the workspace.
/// Returns Some only if exactly one match is found.
/// First tries exact matching, then falls back to fuzzy matching
/// that normalizes casing, underscores, hyphens, camelCase, and spaces.
fn resolve_token(token: &str, workspace_root: &Path, entries: &[PathBuf]) -> Option<PathBuf> {
    // First try exact match
    let exact_matches: Vec<&PathBuf> = if token.contains('/') {
        let target = workspace_root.join(token);
        entries.iter().filter(|e| *e == &target).collect()
    } else {
        entries
            .iter()
            .filter(|e| {
                e.file_name()
                    .map(|name| name.to_string_lossy() == token)
                    .unwrap_or(false)
            })
            .collect()
    };

    if exact_matches.len() == 1 {
        return Some(exact_matches[0].clone());
    }

    // If token contains '/', try fuzzy path matching with typo tolerance.
    if token.contains('/') {
        let fuzzy_path_matches: Vec<&PathBuf> = entries
            .iter()
            .filter(|e| fuzzy_path_match(token, workspace_root, e))
            .collect();
        if fuzzy_path_matches.len() == 1 {
            return Some(fuzzy_path_matches[0].clone());
        }
        return None;
    }

    // Split token into potential name part and extension part
    // e.g. "auth file.ts" -> name="auth file", ext=Some("ts")
    // e.g. "auth file" -> name="auth file", ext=None
    let (token_name, token_ext) = match token.rsplit_once('.') {
        Some((name, ext)) if !ext.contains(' ') && ext.len() <= 10 => (name, Some(ext)),
        _ => (token, None),
    };

    // Fuzzy match against basenames
    let fuzzy_matches: Vec<&PathBuf> = entries
        .iter()
        .filter(|e| {
            let filename = match e.file_name() {
                Some(name) => name.to_string_lossy(),
                None => return false,
            };
            let filename_str = filename.as_ref();

            // If token has an extension, the file must have the same extension
            if let Some(ext) = token_ext {
                let file_ext = e.extension().and_then(|e| e.to_str()).unwrap_or("");
                if !extension_matches(ext, file_ext) {
                    return false;
                }
            }

            fuzzy_basename_match(token_name, filename_str)
        })
        .collect();

    if fuzzy_matches.len() == 1 {
        Some(fuzzy_matches[0].clone())
    } else {
        None
    }
}

const SKIP_DIRS: &[&str] = &[
    ".git",
    "node_modules",
    "dist",
    "build",
    "target",
    ".next",
    "__pycache__",
    ".venv",
];

const MAX_ENTRIES: usize = 50_000;
const INDEX_CACHE_TTL: Duration = Duration::from_secs(5);

struct WorkspaceIndexCacheEntry {
    created_at: Instant,
    entries: Arc<Vec<PathBuf>>,
}

static WORKSPACE_INDEX_CACHE: Lazy<Mutex<HashMap<PathBuf, WorkspaceIndexCacheEntry>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

static SPOKEN_SPLIT_EXTENSION_PATTERNS: Lazy<Vec<(Regex, &'static str)>> = Lazy::new(|| {
    vec![
        (Regex::new(r"(?i)\.\s*r\s*s\b").unwrap(), ".rs"),
        (Regex::new(r"(?i)\.\s*t\s*s\b").unwrap(), ".ts"),
        (Regex::new(r"(?i)\.\s*j\s*s\b").unwrap(), ".js"),
        (Regex::new(r"(?i)\.\s*p\s*y\b").unwrap(), ".py"),
        (Regex::new(r"(?i)\.\s*g\s*o\b").unwrap(), ".go"),
        (Regex::new(r"(?i)\.\s*m\s*d\b").unwrap(), ".md"),
        (Regex::new(r"(?i)\.\s*j\s*s\s*x\b").unwrap(), ".jsx"),
        (Regex::new(r"(?i)\.\s*t\s*s\s*x\b").unwrap(), ".tsx"),
    ]
});

static TRAILING_AFTER_EXTENSION_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)^(.+?\.[a-z0-9]{1,10})(?:\s+.*)?$").unwrap());

/// Walk the workspace directory, skipping common non-source directories.
fn walk_workspace(root: &Path) -> Vec<PathBuf> {
    let mut entries = Vec::new();

    let walker = WalkDir::new(root)
        .max_depth(10)
        .into_iter()
        .filter_entry(|e| {
            if e.file_type().is_dir() {
                if let Some(name) = e.file_name().to_str() {
                    return !SKIP_DIRS.contains(&name);
                }
            }
            true
        });

    for entry in walker.flatten() {
        if entry.file_type().is_file() {
            entries.push(entry.into_path());
            if entries.len() >= MAX_ENTRIES {
                break;
            }
        }
    }

    entries
}

fn get_workspace_entries_cached(root: &Path) -> Arc<Vec<PathBuf>> {
    if let Ok(mut cache) = WORKSPACE_INDEX_CACHE.lock() {
        if let Some(entry) = cache.get(root) {
            if entry.created_at.elapsed() <= INDEX_CACHE_TTL {
                return Arc::clone(&entry.entries);
            }
        }

        let entries = Arc::new(walk_workspace(root));
        cache.insert(
            root.to_path_buf(),
            WorkspaceIndexCacheEntry {
                created_at: Instant::now(),
                entries: Arc::clone(&entries),
            },
        );
        return entries;
    }

    Arc::new(walk_workspace(root))
}

/// Map file extension to a markdown code fence language identifier.
#[cfg(test)]
fn ext_to_lang(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()) {
        Some("rs") => "rust",
        Some("ts") | Some("tsx") => "typescript",
        Some("js") | Some("jsx") => "javascript",
        Some("py") => "python",
        Some("go") => "go",
        Some("java") => "java",
        Some("c") | Some("h") => "c",
        Some("cpp") | Some("hpp") | Some("cc") => "cpp",
        Some("rb") => "ruby",
        Some("sh") | Some("bash") => "bash",
        Some("json") => "json",
        Some("yaml") | Some("yml") => "yaml",
        Some("toml") => "toml",
        Some("md") => "markdown",
        Some("html") => "html",
        Some("css") => "css",
        Some("sql") => "sql",
        Some("swift") => "swift",
        Some("kt") | Some("kts") => "kotlin",
        _ => "",
    }
}

#[cfg(test)]
const MAX_LINES: usize = 200;
#[cfg(test)]
const MAX_CHARS: usize = 25_000;

/// Extract a code snippet from a file. Returns None for binary files.
#[cfg(test)]
fn extract_snippet(path: &Path, workspace_root: &Path) -> Option<String> {
    let content = std::fs::read(path).ok()?;

    // Check for binary content (null bytes in first 8KB)
    let check_len = content.len().min(8192);
    if content[..check_len].contains(&0) {
        return None;
    }

    let text = String::from_utf8(content).ok()?;

    // Cap at MAX_LINES and MAX_CHARS
    let mut result = String::new();
    let mut line_count = 0;
    for line in text.lines() {
        if line_count >= MAX_LINES || result.len() + line.len() > MAX_CHARS {
            break;
        }
        if !result.is_empty() {
            result.push('\n');
        }
        result.push_str(line);
        line_count += 1;
    }

    let rel_path = path
        .strip_prefix(workspace_root)
        .unwrap_or(path)
        .to_string_lossy();
    let lang = ext_to_lang(path);

    Some(format!(
        "\n------------------------------------------------------------\n### Referenced file: {}\n```{}\n{}\n```",
        rel_path, lang, result
    ))
}

fn format_resolved_at_path(path: &Path) -> String {
    let abs = path.to_string_lossy();
    if abs.contains(' ') {
        format!("@\"{}\"", abs)
    } else {
        format!("@{}", abs)
    }
}

/// Expand @tokens in the given text by resolving them against the workspace.
/// Replaces spoken aliases and @tokens with canonical @absolute/path references.
pub fn expand_at_refs(text: &str, workspace_root: &Path) -> String {
    let tokens = parse_at_tokens(text);
    if tokens.is_empty() {
        return text.to_string();
    }

    debug!(
        "Found {} @tokens, resolving against workspace: {}",
        tokens.len(),
        workspace_root.display()
    );

    let entries = get_workspace_entries_cached(workspace_root);
    debug!("Indexed workspace, found {} file entries", entries.len());

    let mut replacements: Vec<(usize, usize, String)> = Vec::new();

    for token in &tokens {
        if let Some(path) = resolve_token(&token.token, workspace_root, entries.as_ref()) {
            debug!("Resolved @{} -> {}", token.token, path.display());
            // Replace both spoken aliases ("at trade.rs") and explicit @tokens
            // with canonical absolute paths.
            replacements.push((token.start, token.end, format_resolved_at_path(&path)));
        } else {
            debug!(
                "Could not uniquely resolve @{} (0 or 2+ matches)",
                token.token
            );
        }
    }

    // If nothing to do, return unchanged
    if replacements.is_empty() {
        return text.to_string();
    }

    let mut result = text.to_string();

    // Apply replacements in reverse order so indices stay valid
    replacements.sort_by(|a, b| b.0.cmp(&a.0));
    for (start, end, replacement) in replacements {
        if end <= result.len() {
            result.replace_range(start..end, &replacement);
        }
    }

    result
}

/// Top-level entry point called from the pipeline.
/// Resolves workspace, expands @refs, and updates MRU.
pub fn maybe_expand_at_refs(
    text: &str,
    settings: &crate::settings::AppSettings,
    app: &tauri::AppHandle,
) -> String {
    if !settings.at_file_expansion_enabled {
        return text.to_string();
    }

    let workspace_root = match crate::context_providers::get_workspace_root(settings) {
        Some(root) => root,
        None => {
            // Fallback to process current directory when frontmost-app context
            // resolution is unavailable.
            match std::env::current_dir() {
                Ok(dir) if dir.is_dir() => dir,
                _ => {
                    debug!("@file expansion enabled but no workspace root detected");
                    return text.to_string();
                }
            }
        }
    };

    // Only allow expansion for Git repositories.
    if !is_git_repository(&workspace_root) {
        debug!(
            "@file expansion skipped: workspace is not inside a Git repository ({})",
            workspace_root.display()
        );
        return text.to_string();
    }

    let expanded = expand_at_refs(text, &workspace_root);

    // Update MRU if we actually found a workspace
    crate::context_providers::update_mru(app, &workspace_root);

    expanded
}

fn is_git_repository(start: &Path) -> bool {
    let mut current = Some(start);
    while let Some(path) = current {
        let git_path = path.join(".git");
        if git_path.exists() {
            return true;
        }
        current = path.parent();
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_parse_at_tokens_simple() {
        let tokens = parse_at_tokens("Check @auth.ts for the bug");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].token, "auth.ts");
    }

    #[test]
    fn test_parse_at_tokens_quoted() {
        let tokens = parse_at_tokens("Look at @\"my file.ts\" please");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].token, "my file.ts");
    }

    #[test]
    fn test_parse_at_tokens_email_skip() {
        let tokens = parse_at_tokens("Send to user@example.com please");
        assert_eq!(tokens.len(), 0);
    }

    #[test]
    fn test_parse_at_tokens_multiple() {
        let tokens = parse_at_tokens("See @auth.ts and @utils.rs");
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].token, "auth.ts");
        assert_eq!(tokens[1].token, "utils.rs");
    }

    #[test]
    fn test_parse_at_tokens_with_path() {
        let tokens = parse_at_tokens("Check @src/lib.rs");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].token, "src/lib.rs");
    }

    #[test]
    fn test_parse_spoken_alias_without_extension() {
        let tokens = parse_at_tokens("At pipeline.");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].token, "pipeline");
    }

    #[test]
    fn test_resolve_token_unique_match() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("auth.ts");
        fs::write(&file_path, "export const auth = {};").unwrap();

        let entries = vec![file_path.clone()];
        let result = resolve_token("auth.ts", dir.path(), &entries);
        assert_eq!(result, Some(file_path));
    }

    #[test]
    fn test_resolve_token_no_match() {
        let dir = TempDir::new().unwrap();
        let entries: Vec<PathBuf> = vec![];
        let result = resolve_token("auth.ts", dir.path(), &entries);
        assert_eq!(result, None);
    }

    #[test]
    fn test_resolve_token_multiple_matches() {
        let dir = TempDir::new().unwrap();
        let path1 = dir.path().join("src").join("auth.ts");
        let path2 = dir.path().join("lib").join("auth.ts");
        fs::create_dir_all(path1.parent().unwrap()).unwrap();
        fs::create_dir_all(path2.parent().unwrap()).unwrap();
        fs::write(&path1, "a").unwrap();
        fs::write(&path2, "b").unwrap();

        let entries = vec![path1, path2];
        let result = resolve_token("auth.ts", dir.path(), &entries);
        assert_eq!(result, None);
    }

    #[test]
    fn test_resolve_token_relative_path() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("src").join("lib.rs");
        fs::create_dir_all(file_path.parent().unwrap()).unwrap();
        fs::write(&file_path, "fn main() {}").unwrap();

        let entries = vec![file_path.clone()];
        let result = resolve_token("src/lib.rs", dir.path(), &entries);
        assert_eq!(result, Some(file_path));
    }

    #[test]
    fn test_expand_at_refs_no_tokens() {
        let dir = TempDir::new().unwrap();
        let text = "Hello world";
        let result = expand_at_refs(text, dir.path());
        assert_eq!(result, text);
    }

    #[test]
    fn test_expand_spoken_alias_without_extension() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("pipeline.rs");
        fs::write(&file_path, "fn main() {}").unwrap();

        let result = expand_at_refs("At pipeline.", dir.path());
        assert!(result.contains(&format!("@{}", file_path.to_string_lossy())));
    }

    #[test]
    fn test_expand_sentence_with_at_filename_and_punctuation() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("README.md");
        fs::write(&file_path, "# title").unwrap();

        let result = expand_at_refs("Can we update at readme.md?", dir.path());
        assert!(result.contains(&format!("@{}", file_path.to_string_lossy())));
    }

    #[test]
    fn test_expand_at_refs_with_match() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("auth.ts");
        fs::write(&file_path, "export const auth = {};").unwrap();

        let text = "Check @auth.ts for the bug";
        let result = expand_at_refs(text, dir.path());
        assert!(result.contains(&format!("@{}", file_path.to_string_lossy())));
        assert!(!result.contains("Referenced file:"));
    }

    #[test]
    fn test_expand_at_refs_no_match() {
        let dir = TempDir::new().unwrap();
        let text = "Check @nonexistent.ts for the bug";
        let result = expand_at_refs(text, dir.path());
        assert_eq!(result, text);
    }

    #[test]
    fn test_walk_workspace_skips_node_modules() {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join("node_modules/pkg")).unwrap();
        fs::write(dir.path().join("node_modules/pkg/index.js"), "x").unwrap();
        fs::write(dir.path().join("app.ts"), "y").unwrap();

        let entries = walk_workspace(dir.path());
        assert_eq!(entries.len(), 1);
        assert!(entries[0].ends_with("app.ts"));
    }

    #[test]
    fn test_extract_snippet_binary_file() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("image.png");
        let mut content = vec![0x89, 0x50, 0x4E, 0x47]; // PNG header
        content.extend_from_slice(&[0u8; 100]); // null bytes
        fs::write(&file_path, &content).unwrap();

        let result = extract_snippet(&file_path, dir.path());
        assert!(result.is_none());
    }

    // Additional tests for replacement logic and edge cases

    #[test]
    fn test_expand_at_refs_multiple_tokens_mixed() {
        let dir = TempDir::new().unwrap();
        let auth_path = dir.path().join("auth.ts");
        let config_path = dir.path().join("config.ts");

        fs::write(&auth_path, "export const auth = {};").unwrap();
        fs::write(&config_path, "export const config = {};").unwrap();

        let text = "Check @auth.ts and also @config.ts";
        let result = expand_at_refs(text, dir.path());

        assert!(result.contains(&format!("@{}", auth_path.to_string_lossy())));
        assert!(result.contains(&format!("@{}", config_path.to_string_lossy())));
    }

    #[test]
    fn test_expand_at_refs_one_match_one_no_match() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("auth.ts");
        fs::write(&file_path, "export const auth = {};").unwrap();

        let text = "Check @auth.ts and @missing.ts";
        let result = expand_at_refs(text, dir.path());

        assert!(result.contains(&format!("@{}", file_path.to_string_lossy())));
        assert!(result.contains("@missing.ts"));
    }

    #[test]
    fn test_expand_at_refs_preserves_original_text() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("auth.ts");
        fs::write(&file_path, "code here").unwrap();

        let text = "Original text with @auth.ts token";
        let result = expand_at_refs(text, dir.path());

        assert!(result.contains("Original text with "));
        assert!(result.contains(" token"));
        assert!(result.contains(&format!("@{}", file_path.to_string_lossy())));
    }

    #[test]
    fn test_expand_at_refs_replacements_preserve_order() {
        let dir = TempDir::new().unwrap();
        let file1 = dir.path().join("first.ts");
        let file2 = dir.path().join("second.ts");

        fs::write(&file1, "first content").unwrap();
        fs::write(&file2, "second content").unwrap();

        let text = "See @first.ts then @second.ts";
        let result = expand_at_refs(text, dir.path());

        let first_abs = format!("@{}", file1.to_string_lossy());
        let second_abs = format!("@{}", file2.to_string_lossy());
        let first_pos = result.find(&first_abs).unwrap();
        let second_pos = result.find(&second_abs).unwrap();
        assert!(first_pos < second_pos);
    }

    #[test]
    fn test_parse_at_tokens_at_start_of_text() {
        let tokens = parse_at_tokens("@auth.ts is important");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].token, "auth.ts");
    }

    #[test]
    fn test_parse_at_tokens_at_end_of_text() {
        let tokens = parse_at_tokens("Check the file @auth.ts");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].token, "auth.ts");
    }

    #[test]
    fn test_parse_at_tokens_with_punctuation() {
        // Trailing sentence punctuation should be trimmed for unquoted tokens.
        let tokens = parse_at_tokens("See @auth.ts.");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].token, "auth.ts");
    }

    #[test]
    fn test_parse_at_tokens_with_comma() {
        let tokens = parse_at_tokens("Check @auth.ts, @config.ts");
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].token, "auth.ts");
        assert_eq!(tokens[1].token, "config.ts");
    }

    #[test]
    fn test_parse_at_tokens_duplicate_same_token() {
        let tokens = parse_at_tokens("@auth.ts and @auth.ts again");
        // Should parse both instances
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].token, "auth.ts");
        assert_eq!(tokens[1].token, "auth.ts");
    }

    #[test]
    fn test_parse_at_tokens_with_special_chars_in_name() {
        let tokens = parse_at_tokens("Check @auth-config.ts and @utils_helpers.ts");
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].token, "auth-config.ts");
        assert_eq!(tokens[1].token, "utils_helpers.ts");
    }

    #[test]
    fn test_parse_at_tokens_quoted_with_spaces() {
        let tokens = parse_at_tokens("Look at @\"my auth file.ts\"");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].token, "my auth file.ts");
    }

    #[test]
    fn test_parse_at_tokens_quoted_with_path() {
        let tokens = parse_at_tokens("See @\"src/my helpers.ts\"");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].token, "src/my helpers.ts");
    }

    #[test]
    fn test_parse_at_tokens_no_false_positives() {
        // Various strings that should NOT be treated as tokens
        let cases = vec![
            "hello@world.com",   // email
            "test@test.org",     // email
            "user@domain.co.uk", // email
            "a@b",               // short email
            "@",                 // just @
            "@ ",                // @ with space
            "@\"\"",             // empty quotes
        ];

        for case in cases {
            let tokens = parse_at_tokens(case);
            assert_eq!(tokens.len(), 0, "Should not parse: {}", case);
        }
    }

    #[test]
    fn test_parse_at_tokens_spoken_at_file_alias() {
        let tokens = parse_at_tokens("please at file auth dot ts");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].token, "auth.ts");
    }

    #[test]
    fn test_parse_at_tokens_spoken_include_file_alias() {
        let tokens = parse_at_tokens("include file src slash main dot rs");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].token, "src/main.rs");
    }

    #[test]
    fn test_parse_at_tokens_spoken_for_alias() {
        let tokens = parse_at_tokens("specifically for main dot rs");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].token, "main.rs");
    }

    #[test]
    fn test_parse_at_tokens_spoken_file_alias() {
        let tokens = parse_at_tokens("please file pipeline dot rs");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].token, "pipeline.rs");
    }

    #[test]
    fn test_parse_at_tokens_sentence_with_at_filename() {
        let tokens = parse_at_tokens("Can we update at readme.md?");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].token, "readme.md");
    }

    #[test]
    fn test_parse_at_tokens_spoken_split_extension() {
        // Transcription often outputs "trade. r s" instead of "trade.rs"
        let tokens = parse_at_tokens("check at trade. r s for the bug");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].token, "trade.rs");
    }

    #[test]
    fn test_parse_at_tokens_spoken_split_extension_with_tsx() {
        let tokens = parse_at_tokens("include file app. t s x in the prompt");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].token, "app.tsx");
    }

    #[test]
    fn test_parse_at_tokens_spoken_path_with_trailing_phrase() {
        let tokens = parse_at_tokens("reference src slash core dot rs for context");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].token, "src/core.rs");
    }

    #[test]
    fn test_expand_at_refs_replaces_spoken_alias() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("trade.rs");
        fs::create_dir_all(file_path.parent().unwrap()).unwrap();
        fs::write(&file_path, "pub fn main() {}").unwrap();

        let text = "check at trade. r s for the bug";
        let result = expand_at_refs(text, dir.path());
        // Spoken alias should be replaced with absolute @path
        let expected = format!("@{}", file_path.to_string_lossy());
        assert!(
            result.contains(&expected),
            "Expected {} in result, got: {}",
            expected,
            result
        );
        assert!(
            !result.contains("at trade. r s"),
            "Spoken alias should be replaced, got: {}",
            result
        );
    }

    #[test]
    fn test_resolve_token_case_sensitivity() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("Auth.ts");
        fs::write(&file_path, "test").unwrap();

        let entries = vec![file_path.clone()];

        // Exact match should work
        let result = resolve_token("Auth.ts", dir.path(), &entries);
        assert_eq!(result, Some(file_path.clone()));

        // Fuzzy match with different case should now match
        let result_fuzzy = resolve_token("auth.ts", dir.path(), &entries);
        assert_eq!(result_fuzzy, Some(file_path));
    }

    #[test]
    fn test_normalize_to_words() {
        // snake_case
        assert_eq!(normalize_to_words("auth_file"), vec!["auth", "file"]);
        // camelCase
        assert_eq!(normalize_to_words("authFile"), vec!["auth", "file"]);
        // kebab-case
        assert_eq!(normalize_to_words("auth-file"), vec!["auth", "file"]);
        // spaces (from speech)
        assert_eq!(normalize_to_words("auth file"), vec!["auth", "file"]);
        // PascalCase
        assert_eq!(normalize_to_words("AuthFile"), vec!["auth", "file"]);
        // mixed
        assert_eq!(
            normalize_to_words("myAuth_file"),
            vec!["my", "auth", "file"]
        );
        // with dots
        assert_eq!(
            normalize_to_words("auth.file.ts"),
            vec!["auth", "file", "ts"]
        );
    }

    #[test]
    fn test_fuzzy_resolve_snake_case_from_speech() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("auth_helpers.ts");
        fs::write(&file_path, "test").unwrap();
        let entries = vec![file_path.clone()];

        // Speech transcription: "auth helpers.ts" -> should match auth_helpers.ts
        let result = resolve_token("auth helpers.ts", dir.path(), &entries);
        assert_eq!(result, Some(file_path));
    }

    #[test]
    fn test_fuzzy_resolve_camel_case_from_speech() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("authHelpers.ts");
        fs::write(&file_path, "test").unwrap();
        let entries = vec![file_path.clone()];

        // Speech: "auth helpers.ts" -> should match authHelpers.ts
        let result = resolve_token("auth helpers.ts", dir.path(), &entries);
        assert_eq!(result, Some(file_path));
    }

    #[test]
    fn test_fuzzy_resolve_kebab_case_from_speech() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("auth-helpers.ts");
        fs::write(&file_path, "test").unwrap();
        let entries = vec![file_path.clone()];

        // Speech: "auth helpers.ts" -> should match auth-helpers.ts
        let result = resolve_token("auth helpers.ts", dir.path(), &entries);
        assert_eq!(result, Some(file_path));
    }

    #[test]
    fn test_fuzzy_resolve_pascal_case_from_speech() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("AuthHelpers.ts");
        fs::write(&file_path, "test").unwrap();
        let entries = vec![file_path.clone()];

        // Speech: "auth helpers.ts" -> should match AuthHelpers.ts
        let result = resolve_token("auth helpers.ts", dir.path(), &entries);
        assert_eq!(result, Some(file_path));
    }

    #[test]
    fn test_fuzzy_resolve_without_extension() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("auth_helpers.ts");
        fs::write(&file_path, "test").unwrap();
        let entries = vec![file_path.clone()];

        // Speech without extension: "auth helpers" -> should match auth_helpers.ts
        let result = resolve_token("auth helpers", dir.path(), &entries);
        assert_eq!(result, Some(file_path));
    }

    #[test]
    fn test_fuzzy_resolve_ambiguous_returns_none() {
        let dir = TempDir::new().unwrap();
        let file1 = dir.path().join("auth_helpers.ts");
        let file2 = dir.path().join("authHelpers.ts");
        fs::write(&file1, "a").unwrap();
        fs::write(&file2, "b").unwrap();
        let entries = vec![file1, file2];

        // Both match "auth helpers" -> ambiguous, return None
        let result = resolve_token("auth helpers", dir.path(), &entries);
        assert_eq!(result, None);
    }

    #[test]
    fn test_fuzzy_resolve_extension_mismatch() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("auth_helpers.rs");
        fs::write(&file_path, "test").unwrap();
        let entries = vec![file_path];

        // Token says .ts but file is .rs -> no match
        let result = resolve_token("auth helpers.ts", dir.path(), &entries);
        assert_eq!(result, None);
    }

    #[test]
    fn test_fuzzy_resolve_minor_extension_typo() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("cargo.toml");
        fs::write(&file_path, "name = \"demo\"").unwrap();
        let entries = vec![file_path.clone()];

        let result = resolve_token("cargo.tomal", dir.path(), &entries);
        assert_eq!(result, Some(file_path));
    }

    #[test]
    fn test_fuzzy_resolve_filename_typo() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("pipeline.rs");
        fs::write(&file_path, "fn main() {}").unwrap();
        let entries = vec![file_path.clone()];

        let result = resolve_token("pipline.rs", dir.path(), &entries);
        assert_eq!(result, Some(file_path));
    }

    #[test]
    fn test_fuzzy_resolve_path_segment_typo() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("src-tauri").join("src").join("pipeline.rs");
        fs::create_dir_all(file_path.parent().unwrap()).unwrap();
        fs::write(&file_path, "fn main() {}").unwrap();
        let entries = vec![file_path.clone()];

        let result = resolve_token("src-tauri/scr/pipeline.rs", dir.path(), &entries);
        assert_eq!(result, Some(file_path));
    }

    #[test]
    fn test_exact_match_preferred_over_fuzzy() {
        let dir = TempDir::new().unwrap();
        let exact = dir.path().join("auth.ts");
        fs::write(&exact, "exact").unwrap();
        let entries = vec![exact.clone()];

        // Exact match should be returned
        let result = resolve_token("auth.ts", dir.path(), &entries);
        assert_eq!(result, Some(exact));
    }

    #[test]
    fn test_extract_snippet_line_count_cap() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("bigfile.rs");

        // Create file with 300 lines
        let mut content = String::new();
        for i in 0..300 {
            content.push_str(&format!("line {}\n", i));
        }
        fs::write(&file_path, &content).unwrap();

        let snippet = extract_snippet(&file_path, dir.path()).unwrap();

        // Should contain only first ~200 lines, not all 300
        let line_count = snippet.matches('\n').count();
        assert!(line_count <= 210); // 200 lines + headers + fence
                                    // Should NOT contain line 250
        assert!(!snippet.contains("line 250"));
        // Should contain early lines
        assert!(snippet.contains("line 0"));
    }

    #[test]
    fn test_extract_snippet_char_count_cap() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("largefile.txt");

        // Create file with 50KB content
        let large_content = "x".repeat(50_000);
        fs::write(&file_path, &large_content).unwrap();

        let snippet = extract_snippet(&file_path, dir.path()).unwrap();

        // Snippet should be under 25KB content + headers
        // (25KB is the max for code section)
        assert!(snippet.len() < 26_000);
    }

    #[test]
    fn test_extract_snippet_utf8_handling() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("unicode.rs");

        let content = "// Unicode test: 你好世界 🚀 Ñoño\nfn main() {}";
        fs::write(&file_path, content).unwrap();

        let snippet = extract_snippet(&file_path, dir.path());
        assert!(snippet.is_some());
        let snippet_str = snippet.unwrap();
        assert!(snippet_str.contains("你好世界"));
        assert!(snippet_str.contains("🚀"));
    }

    #[test]
    fn test_walk_workspace_respects_max_depth() {
        let dir = TempDir::new().unwrap();

        // Create deeply nested structure
        let mut path = dir.path().to_path_buf();
        for i in 0..15 {
            path.push(format!("level{}", i));
            fs::create_dir_all(&path).unwrap();
        }
        fs::write(path.join("deep.txt"), "deep file").unwrap();

        let entries = walk_workspace(dir.path());

        // Should respect max_depth(10) from walkdir config
        // Files deeper than 10 levels should not be included
        let has_deep = entries
            .iter()
            .any(|e| e.to_string_lossy().matches("level").count() > 10);
        assert!(!has_deep);
    }

    #[test]
    fn test_walk_workspace_respects_file_cap() {
        let dir = TempDir::new().unwrap();

        // Create many files
        for i in 0..1000 {
            let file_path = dir.path().join(format!("file_{:04}.txt", i));
            fs::write(&file_path, format!("content {}", i)).unwrap();
        }

        let entries = walk_workspace(dir.path());

        // Should be capped at 50K (we created 1000, so should be fine)
        assert!(entries.len() >= 900); // At least most of them
        assert!(entries.len() <= 50_000);
    }

    #[test]
    fn test_walk_workspace_multiple_skip_dirs() {
        let dir = TempDir::new().unwrap();

        // Create multiple skip directories
        for skip_dir in &[".git", "node_modules", "dist", "build", "target", ".venv"] {
            let path = dir.path().join(skip_dir);
            fs::create_dir_all(&path).unwrap();
            fs::write(path.join("skip.txt"), "skip").unwrap();
        }

        // Create a file in root
        fs::write(dir.path().join("keep.txt"), "keep").unwrap();

        let entries = walk_workspace(dir.path());

        // Should only find keep.txt, not files in skip dirs
        assert_eq!(entries.len(), 1);
        assert!(entries[0].ends_with("keep.txt"));
    }

    #[test]
    fn test_expand_at_refs_format_consistency() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("test.ts");
        fs::write(&file_path, "const x = 1;").unwrap();

        let text = "See @test.ts";
        let result = expand_at_refs(text, dir.path());

        assert_eq!(result, format!("See @{}", file_path.to_string_lossy()));
    }

    #[test]
    fn test_expand_at_refs_empty_text() {
        let dir = TempDir::new().unwrap();
        let text = "";
        let result = expand_at_refs(text, dir.path());
        assert_eq!(result, "");
    }

    #[test]
    fn test_expand_at_refs_only_token() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("test.ts");
        fs::write(&file_path, "code").unwrap();

        let text = "@test.ts";
        let result = expand_at_refs(text, dir.path());

        assert_eq!(result, format!("@{}", file_path.to_string_lossy()));
    }

    #[test]
    fn test_resolve_token_basename_vs_path_precedence() {
        let dir = TempDir::new().unwrap();

        // Create both a basename match and a path match for same file
        let file_path = dir.path().join("helpers.ts");
        fs::write(&file_path, "helpers").unwrap();

        let entries = vec![file_path.clone()];

        // Should match by basename
        let result1 = resolve_token("helpers.ts", dir.path(), &entries);
        assert_eq!(result1, Some(file_path.clone()));

        // Should also match by relative path
        let result2 = resolve_token("helpers.ts", dir.path(), &entries);
        assert_eq!(result2, Some(file_path));
    }
}
