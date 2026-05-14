//! Language-aware poisoning of rejected candidate files.
//!
//! When a write is blocked, the guard overwrites the on-disk file (watcher mode)
//! or serves an overlay (FUSE mode) consisting of a poison header followed by
//! the original rejected bytes wrapped between `JANKURAI-POISON-BEGIN` and
//! `JANKURAI-POISON-END` sentinels. The header is language-aware so the host
//! toolchain immediately fails: the agent cannot silently ignore the block.
//!
//! [`strip`] is the inverse: it removes a recognized poison header and the
//! sentinel wrapper, recovering the original bytes. It is idempotent — calling
//! it on non-poisoned bytes returns them unchanged.

pub mod payloads;

use crate::GuardError;
use payloads::{Lang, PoisonContent, BEGIN_SENTINEL, END_SENTINEL, MARKER};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub use payloads::PoisonContent as Content;

/// Returns the language family for a path based on its file extension.
pub fn lang_for_path(path: &Path) -> Lang {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    Lang::from_extension(&ext)
}

/// Wraps `original` rejected bytes in a poison header plus the sentinel-delimited
/// original content. The result is what the agent sees on disk for a blocked
/// file.
pub fn wrap(path: &Path, original: &[u8], content: &PoisonContent) -> Vec<u8> {
    let lang = lang_for_path(path);
    let header = payloads::header_for(lang, content);
    let (begin, end) = lang.comment_style().sentinels();
    let mut out = Vec::with_capacity(original.len() + header.len() + begin.len() + end.len() + 8);
    out.extend_from_slice(header.as_bytes());
    out.extend_from_slice(begin.as_bytes());
    out.push(b'\n');
    out.extend_from_slice(original);
    // A separator newline is always inserted before the END sentinel, even when
    // `original` already ends with one. `strip` removes exactly this one
    // separator, so the round trip recovers `original` byte-for-byte regardless
    // of whether it had a trailing newline.
    out.push(b'\n');
    out.extend_from_slice(end.as_bytes());
    out.push(b'\n');
    out
}

/// Removes a recognized poison header and sentinel wrapper, returning the inner
/// original bytes. If `bytes` is not poisoned it is returned unchanged. The
/// operation is idempotent: `strip(strip(x)) == strip(x)`.
pub fn strip(bytes: &[u8]) -> Vec<u8> {
    let text = match std::str::from_utf8(bytes) {
        Ok(text) => text,
        Err(_) => return bytes.to_vec(),
    };
    if !text.contains(MARKER) {
        return bytes.to_vec();
    }
    let begin_idx = match find_sentinel_line(text, BEGIN_SENTINEL) {
        Some(idx) => idx,
        None => return bytes.to_vec(),
    };
    let end_idx = match find_sentinel_line(text, END_SENTINEL) {
        Some(idx) => idx,
        None => return bytes.to_vec(),
    };
    if end_idx <= begin_idx {
        return bytes.to_vec();
    }
    // The inner content lies between the line after BEGIN and the line before END.
    let after_begin = match text[begin_idx..].find('\n') {
        Some(rel) => begin_idx + rel + 1,
        None => return bytes.to_vec(),
    };
    if after_begin > end_idx {
        return bytes.to_vec();
    }
    let inner = &text[after_begin..end_idx];
    // The wrap step guarantees a trailing newline before the END sentinel; drop
    // exactly that one separator so a round trip recovers the original bytes.
    let inner = inner.strip_suffix('\n').unwrap_or(inner);
    inner.as_bytes().to_vec()
}

/// Finds the byte offset of the line that contains `sentinel` as its trailing
/// token. The sentinel always appears at the end of a comment line.
fn find_sentinel_line(text: &str, sentinel: &str) -> Option<usize> {
    let mut offset = 0;
    for line in text.split_inclusive('\n') {
        if line.trim_end().ends_with(sentinel) {
            return Some(offset);
        }
        offset += line.len();
    }
    None
}

/// Returns `true` when `bytes` carry a recognizable poison header.
pub fn is_poisoned(bytes: &[u8]) -> bool {
    match std::str::from_utf8(bytes) {
        Ok(text) => {
            text.contains(MARKER)
                && find_sentinel_line(text, BEGIN_SENTINEL).is_some()
                && find_sentinel_line(text, END_SENTINEL).is_some()
        }
        Err(_) => false,
    }
}

/// The active poisoned view for each blocked path, persisted under
/// `state/<repo>/poison/`. Each entry maps a repo-relative path to the exact
/// bytes the agent should see for that file until the block is cleared.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PoisonState {
    /// Active poisoned views keyed by repo-relative path.
    entries: BTreeMap<String, String>,
}

impl PoisonState {
    /// Creates an empty poison state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Records the poisoned view bytes for `rel_path`.
    pub fn insert(&mut self, rel_path: &Path, view: &[u8]) {
        self.entries.insert(
            rel_path.to_string_lossy().replace('\\', "/"),
            String::from_utf8_lossy(view).into_owned(),
        );
    }

    /// Returns the poisoned view bytes for `rel_path`, if one is active.
    pub fn get(&self, rel_path: &Path) -> Option<Vec<u8>> {
        let key = rel_path.to_string_lossy().replace('\\', "/");
        self.entries.get(&key).map(|s| s.clone().into_bytes())
    }

    /// Clears the poisoned view for `rel_path`. Returns `true` when an entry was
    /// removed.
    pub fn clear(&mut self, rel_path: &Path) -> bool {
        let key = rel_path.to_string_lossy().replace('\\', "/");
        self.entries.remove(&key).is_some()
    }

    /// Returns `true` when no poisoned views are active.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Returns the repo-relative paths with an active poisoned view.
    pub fn poisoned_paths(&self) -> Vec<PathBuf> {
        self.entries.keys().map(PathBuf::from).collect()
    }

    /// Loads the poison state from `dir/poison-state.json`, returning an empty
    /// state when the file is absent.
    pub fn load(dir: &Path) -> Result<Self, GuardError> {
        let path = dir.join("poison-state.json");
        match std::fs::read_to_string(&path) {
            Ok(text) => serde_json::from_str(&text)
                .map_err(|e| GuardError::State(format!("{}: {e}", path.display()))),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::new()),
            Err(e) => Err(GuardError::State(format!("{}: {e}", path.display()))),
        }
    }

    /// Persists the poison state to `dir/poison-state.json`.
    pub fn save(&self, dir: &Path) -> Result<(), GuardError> {
        std::fs::create_dir_all(dir)?;
        let path = dir.join("poison-state.json");
        let text = serde_json::to_string_pretty(self)
            .map_err(|e| GuardError::State(format!("serialize poison state: {e}")))?;
        std::fs::write(&path, text)?;
        Ok(())
    }
}

/// Re-export so callers can name the comment style without reaching into
/// `payloads`.
pub use payloads::CommentStyle as WrapCommentStyle;

#[cfg(test)]
mod tests {
    use super::*;
    use payloads::CommentStyle;

    fn content() -> PoisonContent {
        PoisonContent {
            path: "src/foo.rs".to_string(),
            rule_ids: vec!["HLT-029".to_string()],
            problems: vec!["unbounded recursion".to_string()],
            fix_steps: vec!["add a base case".to_string()],
            rerun_command: "jankurai audit-file . --path src/foo.rs --candidate -".to_string(),
            report_path: ".jankurai/guard/LAST_FAILURE.md".to_string(),
        }
    }

    #[test]
    fn wrap_then_strip_recovers_original() {
        let original = b"fn main() { recurse() }\n";
        let path = Path::new("src/foo.rs");
        let poisoned = wrap(path, original, &content());
        assert!(is_poisoned(&poisoned));
        assert_eq!(strip(&poisoned), original);
        // idempotent
        assert_eq!(strip(&strip(&poisoned)), original.to_vec());
    }

    #[test]
    fn strip_non_poisoned_is_identity() {
        let plain = b"just some text\n";
        assert_eq!(strip(plain), plain.to_vec());
    }

    #[test]
    fn comment_style_routes_by_lang() {
        assert_eq!(
            lang_for_path(Path::new("a.py")).comment_style(),
            CommentStyle::Hash
        );
    }
}
