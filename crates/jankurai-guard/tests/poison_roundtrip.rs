//! Tests for `jankurai_guard::poison`: per-extension poison generation embeds
//! the rule ids and report path, `strip` is idempotent and recovers the
//! original bytes, and an unknown extension falls back to a banner-comment
//! header.

use jankurai_guard::poison::{self, Content};
use std::path::Path;

/// Builds representative poison content for tests.
fn content() -> Content {
    Content {
        path: "src/thing.ext".to_string(),
        rule_ids: vec!["HLT-029".to_string(), "HLT-007".to_string()],
        problems: vec!["unbounded recursion".to_string()],
        fix_steps: vec!["add a base case".to_string(), "bound the depth".to_string()],
        rerun_command: "jankurai audit-file . --path src/thing.ext --candidate -".to_string(),
        report_path: ".jankurai/guard/failures/20260514T120000Z.md".to_string(),
    }
}

/// Every extension's poison header must embed the rule ids and the report path.
#[test]
fn every_extension_embeds_rule_ids_and_report_path() {
    let extensions = [
        "rs", "ts", "tsx", "js", "jsx", "mjs", "cjs", "py", "go", "json", "toml", "yaml", "yml",
        "ini", "cfg", "sh", "bash", "sql", "md", "txt",
    ];
    let original = b"ORIGINAL BYTES";
    for ext in extensions {
        let path = format!("src/thing.{ext}");
        let poisoned = poison::wrap(Path::new(&path), original, &content());
        let text = String::from_utf8(poisoned.clone()).unwrap();
        assert!(
            text.contains("HLT-029"),
            "{ext}: poison header missing rule id"
        );
        assert!(
            text.contains(".jankurai/guard/failures/20260514T120000Z.md"),
            "{ext}: poison header missing report path"
        );
        assert!(
            text.contains("JANKURAI SAVE BLOCKED"),
            "{ext}: poison header missing marker"
        );
        // The original bytes survive inside the wrapper.
        assert_eq!(
            poison::strip(&poisoned),
            original,
            "{ext}: strip lost bytes"
        );
    }
}

/// `strip` recovers the exact original bytes and is idempotent.
#[test]
fn strip_recovers_original_and_is_idempotent() {
    let original = b"fn main() {\n    println!(\"hi\");\n}\n";
    let poisoned = poison::wrap(Path::new("src/main.rs"), original, &content());
    let once = poison::strip(&poisoned);
    assert_eq!(once, original);
    // Stripping already-clean bytes returns them unchanged.
    let twice = poison::strip(&once);
    assert_eq!(twice, once);
}

/// Stripping bytes that were never poisoned returns them unchanged.
#[test]
fn strip_non_poisoned_is_identity() {
    let plain = b"nothing to see here\n";
    assert_eq!(poison::strip(plain), plain.to_vec());
    let empty: &[u8] = b"";
    assert_eq!(poison::strip(empty), empty.to_vec());
}

/// An unknown extension uses the banner-comment header form.
#[test]
fn unknown_extension_uses_banner_form() {
    let poisoned = poison::wrap(Path::new("data/blob.weirdext"), b"raw data", &content());
    let text = String::from_utf8(poisoned.clone()).unwrap();
    // The text/banner form opens with a row of `=` separators.
    assert!(text.starts_with("======"));
    assert!(text.contains("JANKURAI SAVE BLOCKED"));
    assert_eq!(poison::strip(&poisoned), b"raw data");
}

/// A file with no extension at all also gets the banner form and round-trips.
#[test]
fn no_extension_round_trips() {
    let poisoned = poison::wrap(Path::new("Makefile"), b"all:\n\techo hi\n", &content());
    assert!(poison::is_poisoned(&poisoned));
    assert_eq!(poison::strip(&poisoned), b"all:\n\techo hi\n");
}

/// The Rust header uses `compile_error!` so the file fails to compile.
#[test]
fn rust_header_is_compile_error() {
    let poisoned = poison::wrap(Path::new("src/x.rs"), b"x", &content());
    let text = String::from_utf8(poisoned).unwrap();
    assert!(text.starts_with("compile_error!(r#\""));
}

/// The Python header raises at import time.
#[test]
fn python_header_raises() {
    let poisoned = poison::wrap(Path::new("x.py"), b"x", &content());
    let text = String::from_utf8(poisoned).unwrap();
    assert!(text.starts_with("raise RuntimeError("));
}
