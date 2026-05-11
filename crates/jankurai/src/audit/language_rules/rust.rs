use super::catalog::{
    ConfidencePolicy, Language, LanguageFinding, LanguageRule, Matcher, ProofWindow,
};
use crate::audit::helpers::{product_code_files, AuditContext};
use crate::audit::scan;
use once_cell::sync::Lazy;

const HLT_RULE_ID: &str = "HLT-029-RUST-BAD-BEHAVIOR";

const HARD_RULES: &[LanguageRule] = &[
    LanguageRule {
        id: "rust.unsafe.undocumented-block",
        language: Language::Rust,
        hlt_rule_id: HLT_RULE_ID,
        severity: "high",
        category: "security",
        lane: "fast",
        confidence: ConfidencePolicy::High,
        matcher: Matcher::ContainsAny(&["unsafe {", "unsafe{"]),
        proof_window: ProofWindow::NearbySafetyComment,
        problem: "unsafe block lacks a nearby local safety argument",
        fix: "add a precise `SAFETY:` comment or remove the unsafe block",
    },
    LanguageRule {
        id: "rust.unsafe.public-fn-missing-safety-doc",
        language: Language::Rust,
        hlt_rule_id: HLT_RULE_ID,
        severity: "high",
        category: "security",
        lane: "fast",
        confidence: ConfidencePolicy::High,
        matcher: Matcher::ContainsAny(&[
            "pub unsafe fn",
            "pub(crate) unsafe fn",
            "pub(super) unsafe fn",
        ]),
        proof_window: ProofWindow::NearbySafetyDocs,
        problem: "public unsafe API lacks a `# Safety` contract",
        fix: "document caller obligations with a `# Safety` section",
    },
    LanguageRule {
        id: "rust.unsafe.impl-send-sync",
        language: Language::Rust,
        hlt_rule_id: HLT_RULE_ID,
        severity: "high",
        category: "security",
        lane: "fast",
        confidence: ConfidencePolicy::High,
        matcher: Matcher::ContainsAny(&["unsafe impl send", "unsafe impl sync"]),
        proof_window: ProofWindow::NearbySafetyDocs,
        problem: "unsafe Send/Sync impl needs a thread-safety proof",
        fix: "remove the unsafe impl or document and prove the synchronization invariant",
    },
    LanguageRule {
        id: "rust.unsafe.transmute",
        language: Language::Rust,
        hlt_rule_id: HLT_RULE_ID,
        severity: "high",
        category: "security",
        lane: "fast",
        confidence: ConfidencePolicy::High,
        matcher: Matcher::ContainsAny(&["transmute("]),
        proof_window: ProofWindow::NearbySafetyComment,
        problem: "transmute hides layout, validity, or lifetime assumptions",
        fix: "replace transmute with an explicit conversion that proves layout and validity",
    },
    LanguageRule {
        id: "rust.unsafe.assume-init",
        language: Language::Rust,
        hlt_rule_id: HLT_RULE_ID,
        severity: "high",
        category: "security",
        lane: "fast",
        confidence: ConfidencePolicy::High,
        matcher: Matcher::ContainsAny(&["assume_init("]),
        proof_window: ProofWindow::NearbySafetyComment,
        problem: "assume_init can read uninitialized memory",
        fix: "initialize every field before converting from MaybeUninit",
    },
    LanguageRule {
        id: "rust.unsafe.zeroed",
        language: Language::Rust,
        hlt_rule_id: HLT_RULE_ID,
        severity: "high",
        category: "security",
        lane: "fast",
        confidence: ConfidencePolicy::High,
        matcher: Matcher::ContainsAny(&["zeroed("]),
        proof_window: ProofWindow::NearbySafetyComment,
        problem: "mem::zeroed can fabricate invalid values",
        fix: "construct the type with a valid initializer instead of zeroing it",
    },
    LanguageRule {
        id: "rust.unsafe.get-unchecked",
        language: Language::Rust,
        hlt_rule_id: HLT_RULE_ID,
        severity: "high",
        category: "security",
        lane: "fast",
        confidence: ConfidencePolicy::High,
        matcher: Matcher::ContainsAny(&["get_unchecked("]),
        proof_window: ProofWindow::NearbySafetyComment,
        problem: "unchecked indexing needs a mechanical bounds proof",
        fix: "replace unchecked access with a checked access path or prove the bounds locally",
    },
    LanguageRule {
        id: "rust.unsafe.unwrap-unchecked",
        language: Language::Rust,
        hlt_rule_id: HLT_RULE_ID,
        severity: "high",
        category: "security",
        lane: "fast",
        confidence: ConfidencePolicy::High,
        matcher: Matcher::ContainsAny(&["unwrap_unchecked("]),
        proof_window: ProofWindow::NearbySafetyComment,
        problem: "unwrap_unchecked bypasses the option/result proof",
        fix: "use a checked path or add a local proof that the value is always present",
    },
    LanguageRule {
        id: "rust.unsafe.unreachable-unchecked",
        language: Language::Rust,
        hlt_rule_id: HLT_RULE_ID,
        severity: "high",
        category: "security",
        lane: "fast",
        confidence: ConfidencePolicy::High,
        matcher: Matcher::ContainsAny(&["unreachable_unchecked("]),
        proof_window: ProofWindow::NearbySafetyComment,
        problem: "unreachable_unchecked assumes a proof that reviewers cannot infer",
        fix: "replace it with a checked branch or a documented invariant",
    },
    LanguageRule {
        id: "rust.unsafe.from-utf8-unchecked",
        language: Language::Rust,
        hlt_rule_id: HLT_RULE_ID,
        severity: "high",
        category: "security",
        lane: "fast",
        confidence: ConfidencePolicy::High,
        matcher: Matcher::ContainsAny(&["from_utf8_unchecked("]),
        proof_window: ProofWindow::NearbySafetyComment,
        problem: "unchecked UTF-8 conversion can fabricate invalid text",
        fix: "validate the bytes or keep the value as raw bytes",
    },
    LanguageRule {
        id: "rust.unsafe.raw-parts",
        language: Language::Rust,
        hlt_rule_id: HLT_RULE_ID,
        severity: "high",
        category: "security",
        lane: "fast",
        confidence: ConfidencePolicy::High,
        matcher: Matcher::ContainsAny(&[
            "from_raw_parts(",
            "Box::from_raw(",
            "Vec::from_raw_parts(",
            "CString::from_raw(",
        ]),
        proof_window: ProofWindow::NearbySafetyComment,
        problem: "raw ownership conversion needs exact provenance and allocator proof",
        fix: "use the matching constructor/destructor pair or add a documented ownership proof",
    },
    LanguageRule {
        id: "rust.unsafe.static-mut",
        language: Language::Rust,
        hlt_rule_id: HLT_RULE_ID,
        severity: "high",
        category: "security",
        lane: "fast",
        confidence: ConfidencePolicy::High,
        matcher: Matcher::ContainsAny(&["static mut"]),
        proof_window: ProofWindow::NearbySafetyComment,
        problem: "static mut exposes unsynchronized global mutation",
        fix: "replace the mutable static with explicit synchronization or scoped ownership",
    },
    LanguageRule {
        id: "rust.unsafe.repr-packed",
        language: Language::Rust,
        hlt_rule_id: HLT_RULE_ID,
        severity: "high",
        category: "security",
        lane: "fast",
        confidence: ConfidencePolicy::High,
        matcher: Matcher::ContainsAny(&["repr(packed)"]),
        proof_window: ProofWindow::NearbySafetyComment,
        problem: "repr(packed) makes alignment-sensitive references dangerous",
        fix: "remove packed layout or access fields through unaligned-safe primitives",
    },
    LanguageRule {
        id: "rust.supply.allow-warnings",
        language: Language::Rust,
        hlt_rule_id: HLT_RULE_ID,
        severity: "high",
        category: "security",
        lane: "fast",
        confidence: ConfidencePolicy::Medium,
        matcher: Matcher::ContainsAny(&[
            "allow(warnings)",
            "allow(clippy::all)",
            "allow(clippy::correctness)",
        ]),
        proof_window: ProofWindow::NearbySafetyDocs,
        problem: "broad lint suppression hides review-relevant issues",
        fix: "tighten the lint scope or remove the allow entirely",
    },
    LanguageRule {
        id: "rust.supply.rustflags-warn-suppression",
        language: Language::Rust,
        hlt_rule_id: HLT_RULE_ID,
        severity: "high",
        category: "security",
        lane: "fast",
        confidence: ConfidencePolicy::Medium,
        matcher: Matcher::ContainsAny(&["-a warnings", "-awarnings"]),
        proof_window: ProofWindow::NearbySafetyDocs,
        problem: "RUSTFLAGS warning suppression hides build diagnostics",
        fix: "remove warning suppression and fix the underlying warnings",
    },
    LanguageRule {
        id: "rust.security.shell-c-dynamic",
        language: Language::Rust,
        hlt_rule_id: HLT_RULE_ID,
        severity: "high",
        category: "security",
        lane: "fast",
        confidence: ConfidencePolicy::High,
        matcher: Matcher::ContainsAll(&["command::new(", ".arg(\"-c\")"]),
        proof_window: ProofWindow::NearbyAsyncContext,
        problem: "shell execution with dynamic command text is unsafe by default",
        fix: "pass argv values directly or use a bounded, allowlisted command path",
    },
    LanguageRule {
        id: "rust.async.unbounded-channel",
        language: Language::Rust,
        hlt_rule_id: HLT_RULE_ID,
        severity: "high",
        category: "security",
        lane: "fast",
        confidence: ConfidencePolicy::High,
        matcher: Matcher::ContainsAny(&["unbounded_channel(", "unbounded("]),
        proof_window: ProofWindow::NearbyAsyncContext,
        problem: "unbounded channel or task creation needs explicit backpressure proof",
        fix: "use a bounded channel or document the queue bound and shutdown path",
    },
    LanguageRule {
        id: "rust.async.block_on-in-async",
        language: Language::Rust,
        hlt_rule_id: HLT_RULE_ID,
        severity: "high",
        category: "security",
        lane: "fast",
        confidence: ConfidencePolicy::High,
        matcher: Matcher::ContainsAny(&["block_on("]),
        proof_window: ProofWindow::NearbyAsyncContext,
        problem: "blocking inside async context can stall the executor",
        fix: "move the work out of async context or use an async-native path",
    },
];

const ADVISORY_RULES: &[LanguageRule] = &[
    LanguageRule {
        id: "rust.review.clone-overuse",
        language: Language::Rust,
        hlt_rule_id: HLT_RULE_ID,
        severity: "medium",
        category: "review",
        lane: "fast",
        confidence: ConfidencePolicy::Low,
        matcher: Matcher::ContainsAny(&[".clone(", "clone()"]),
        proof_window: ProofWindow::None,
        problem: "clone may be hiding ownership or performance drift",
        fix: "prefer borrowing or move semantics when the copy is not essential",
    },
    LanguageRule {
        id: "rust.review.arc-mutex-default",
        language: Language::Rust,
        hlt_rule_id: HLT_RULE_ID,
        severity: "medium",
        category: "review",
        lane: "fast",
        confidence: ConfidencePolicy::Low,
        matcher: Matcher::ContainsAny(&["Arc<Mutex<", "Arc<tokio::sync::Mutex<"]),
        proof_window: ProofWindow::None,
        problem: "Arc<Mutex<_>> may be a default ownership escape hatch",
        fix: "consider explicit ownership or a bounded message-passing boundary",
    },
    LanguageRule {
        id: "rust.review.rc-refcell-default",
        language: Language::Rust,
        hlt_rule_id: HLT_RULE_ID,
        severity: "medium",
        category: "review",
        lane: "fast",
        confidence: ConfidencePolicy::Low,
        matcher: Matcher::ContainsAny(&["Rc<RefCell<"]),
        proof_window: ProofWindow::None,
        problem: "Rc<RefCell<_>> may be hiding interior mutability",
        fix: "encode the ownership model more directly or document the single-threaded invariant",
    },
    LanguageRule {
        id: "rust.review.as-cast",
        language: Language::Rust,
        hlt_rule_id: HLT_RULE_ID,
        severity: "medium",
        category: "review",
        lane: "fast",
        confidence: ConfidencePolicy::Low,
        matcher: Matcher::ContainsAny(&[" as "]),
        proof_window: ProofWindow::None,
        problem: "broad as-casts can hide truncation or type dishonesty",
        fix: "use TryFrom, helper newtypes, or a proof-oriented conversion",
    },
    LanguageRule {
        id: "rust.review.repr-c",
        language: Language::Rust,
        hlt_rule_id: HLT_RULE_ID,
        severity: "medium",
        category: "review",
        lane: "fast",
        confidence: ConfidencePolicy::Low,
        matcher: Matcher::ContainsAny(&["repr(C)", "repr(C,"]),
        proof_window: ProofWindow::None,
        problem: "repr(C) is a contract surface that needs review",
        fix: "document the ABI and ownership story next to the type definition",
    },
    LanguageRule {
        id: "rust.review.pin",
        language: Language::Rust,
        hlt_rule_id: HLT_RULE_ID,
        severity: "medium",
        category: "review",
        lane: "fast",
        confidence: ConfidencePolicy::Low,
        matcher: Matcher::ContainsAny(&["Pin<", "Pin::new_unchecked", "pin_project"]),
        proof_window: ProofWindow::None,
        problem: "Pin usage deserves a locality proof even when it is valid",
        fix: "document the pinning contract and keep the projection code small",
    },
    LanguageRule {
        id: "rust.review.atomics",
        language: Language::Rust,
        hlt_rule_id: HLT_RULE_ID,
        severity: "medium",
        category: "review",
        lane: "fast",
        confidence: ConfidencePolicy::Low,
        matcher: Matcher::ContainsAny(&["Atomic", "Ordering::Relaxed"]),
        proof_window: ProofWindow::None,
        problem: "atomics need an explicit ordering story",
        fix: "document the memory-order invariant next to the atomic",
    },
    LanguageRule {
        id: "rust.review.generic-unwrap",
        language: Language::Rust,
        hlt_rule_id: HLT_RULE_ID,
        severity: "medium",
        category: "review",
        lane: "fast",
        confidence: ConfidencePolicy::Low,
        matcher: Matcher::ContainsAny(&["unwrap(", "expect("]),
        proof_window: ProofWindow::None,
        problem: "generic unwrap/expect in non-test Rust code deserves a proof check",
        fix: "replace the panic path with a typed error or a documented invariant",
    },
];

pub fn catalog() -> &'static [LanguageRule] {
    static RULES: Lazy<Vec<LanguageRule>> = Lazy::new(|| {
        let mut rules = Vec::new();
        rules.extend_from_slice(HARD_RULES);
        rules.extend_from_slice(ADVISORY_RULES);
        rules
    });
    RULES.as_slice()
}

#[derive(Debug, Clone, Copy, Default)]
pub struct RustSummary {
    pub hard_findings: usize,
    pub advisory_signals: usize,
}

pub fn summary(ctx: &AuditContext) -> RustSummary {
    RustSummary {
        hard_findings: findings(ctx).len(),
        advisory_signals: advisory_signals(ctx).len(),
    }
}

pub fn findings(ctx: &AuditContext) -> Vec<LanguageFinding> {
    let mut out = hard_findings(ctx);
    out.sort_by(|a, b| {
        a.path
            .cmp(&b.path)
            .then(a.line.unwrap_or(0).cmp(&b.line.unwrap_or(0)))
            .then(a.matched_term.cmp(b.matched_term))
    });
    out
}

pub fn advisory_signals(ctx: &AuditContext) -> Vec<LanguageFinding> {
    let mut out = Vec::new();
    for file in rust_files(ctx) {
        for (idx, line) in file.text.lines().enumerate() {
            if let Some(hit) = advisory_hit_for_line(&file, idx + 1, line) {
                out.push(hit);
            }
        }
    }
    out
}

fn hard_findings(ctx: &AuditContext) -> Vec<LanguageFinding> {
    let mut out = Vec::new();
    for file in rust_files(ctx) {
        for (idx, line) in file.text.lines().enumerate() {
            if let Some(hit) = hard_hit_for_line(&file, idx + 1, line, &file.text) {
                out.push(hit);
            }
        }
    }
    out.extend(lint_suppression_hits(ctx));
    out.sort_by(|a, b| {
        a.path
            .cmp(&b.path)
            .then(a.line.unwrap_or(0).cmp(&b.line.unwrap_or(0)))
            .then(a.matched_term.cmp(b.matched_term))
    });
    out
}

fn rust_files(ctx: &AuditContext) -> Vec<crate::model::FileInfo> {
    let zone_paths = crate::audit::helpers::generated_zone_paths(ctx);
    product_code_files(ctx)
        .into_iter()
        .filter(|file| {
            let rel = file.rel_path.to_ascii_lowercase();
            file.suffix == ".rs"
                && !scan::is_generated_or_reference_path(&file.rel_path)
                && !scan::is_test_or_example_path(&file.rel_path)
                && !rel.starts_with("crates/jankurai/")
                && !rel.starts_with("crates/jankurai-proofbind/")
                && !rel.starts_with("crates/jankurai-proofmark/")
                && !zone_paths
                    .iter()
                    .any(|zone| crate::audit::helpers::path_matches_prefix(&file.rel_path, zone))
        })
        .collect()
}

fn hard_hit_for_line(
    file: &crate::model::FileInfo,
    line_no: usize,
    line: &str,
    full_text: &str,
) -> Option<LanguageFinding> {
    let lower = line.to_ascii_lowercase();

    if (lower.contains("pub unsafe fn")
        || lower.contains("pub(crate) unsafe fn")
        || lower.contains("pub(super) unsafe fn"))
        && !scan::public_unsafe_has_safety_docs(full_text, line_no)
    {
        return Some(finding(
            "rust.unsafe.public-fn-missing-safety-doc",
            "pub unsafe fn",
            file,
            line_no,
            line,
            "public unsafe API lacks a `# Safety` contract",
            "missing `# Safety` docs above the public unsafe item",
            "document caller obligations with a `# Safety` section",
            "NearbySafetyDocs",
        ));
    }

    if lower.contains("unsafe impl send") || lower.contains("unsafe impl sync") {
        return Some(finding(
            "rust.unsafe.impl-send-sync",
            "unsafe impl",
            file,
            line_no,
            line,
            "unsafe Send/Sync impl needs a thread-safety proof",
            "thread-safety proof is missing",
            "remove the unsafe impl or document and prove the synchronization invariant",
            "NearbySafetyDocs",
        ));
    }

    if (lower.contains("unsafe {") || lower.contains("unsafe{"))
        && !scan::line_has_nearby_safety_comment(full_text, line_no)
        && !scan::public_unsafe_has_safety_docs(full_text, line_no)
    {
        return Some(finding(
            "rust.unsafe.undocumented-block",
            "unsafe {",
            file,
            line_no,
            line,
            "unsafe block lacks a nearby local safety argument",
            "no nearby SAFETY comment was found",
            "add a precise `SAFETY:` comment or remove the unsafe block",
            "NearbySafetyComment",
        ));
    }

    if lower.contains("transmute(") {
        return Some(finding(
            "rust.unsafe.transmute",
            "transmute",
            file,
            line_no,
            line,
            "transmute hides layout, validity, or lifetime assumptions",
            "unsafe transmute appears without a local proof",
            "replace transmute with an explicit conversion that proves layout and validity",
            "NearbySafetyComment",
        ));
    }
    if lower.contains("assume_init(") {
        return Some(finding(
            "rust.unsafe.assume-init",
            "assume_init",
            file,
            line_no,
            line,
            "assume_init can read uninitialized memory",
            "MaybeUninit proof is missing",
            "initialize every field before converting from MaybeUninit",
            "NearbySafetyComment",
        ));
    }
    if lower.contains("zeroed(") {
        return Some(finding(
            "rust.unsafe.zeroed",
            "zeroed",
            file,
            line_no,
            line,
            "mem::zeroed can fabricate invalid values",
            "all-zero validity was not proven",
            "construct the type with a valid initializer instead of zeroing it",
            "NearbySafetyComment",
        ));
    }
    if lower.contains("get_unchecked(") {
        return Some(finding(
            "rust.unsafe.get-unchecked",
            "get_unchecked",
            file,
            line_no,
            line,
            "unchecked indexing needs a mechanical bounds proof",
            "bounds proof is missing",
            "replace unchecked access with a checked access path or prove the bounds locally",
            "NearbySafetyComment",
        ));
    }
    if lower.contains("unwrap_unchecked(") {
        return Some(finding(
            "rust.unsafe.unwrap-unchecked",
            "unwrap_unchecked",
            file,
            line_no,
            line,
            "unwrap_unchecked bypasses the option/result proof",
            "presence proof is missing",
            "use a checked path or add a local proof that the value is always present",
            "NearbySafetyComment",
        ));
    }
    if lower.contains("unreachable_unchecked(") {
        return Some(finding(
            "rust.unsafe.unreachable-unchecked",
            "unreachable_unchecked",
            file,
            line_no,
            line,
            "unreachable_unchecked assumes a proof that reviewers cannot infer",
            "control-flow proof is missing",
            "replace it with a checked branch or a documented invariant",
            "NearbySafetyComment",
        ));
    }
    if lower.contains("from_utf8_unchecked(") {
        return Some(finding(
            "rust.unsafe.from-utf8-unchecked",
            "from_utf8_unchecked",
            file,
            line_no,
            line,
            "unchecked UTF-8 conversion can fabricate invalid text",
            "UTF-8 validity is missing",
            "validate the bytes or keep the value as raw bytes",
            "NearbySafetyComment",
        ));
    }
    if lower.contains("box::from_raw(") {
        return Some(finding(
            "rust.unsafe.raw-parts",
            "Box::from_raw",
            file,
            line_no,
            line,
            "raw ownership conversion needs exact provenance and allocator proof",
            "ownership provenance is missing",
            "use the matching constructor/destructor pair or add a documented ownership proof",
            "NearbySafetyComment",
        ));
    }
    if lower.contains("vec::from_raw_parts(") {
        return Some(finding(
            "rust.unsafe.raw-parts",
            "Vec::from_raw_parts",
            file,
            line_no,
            line,
            "raw ownership conversion needs exact provenance and allocator proof",
            "ownership provenance is missing",
            "use the matching constructor/destructor pair or add a documented ownership proof",
            "NearbySafetyComment",
        ));
    }
    if lower.contains("cstring::from_raw(") {
        return Some(finding(
            "rust.unsafe.raw-parts",
            "CString::from_raw",
            file,
            line_no,
            line,
            "raw ownership conversion needs exact provenance and allocator proof",
            "ownership provenance is missing",
            "use the matching constructor/destructor pair or add a documented ownership proof",
            "NearbySafetyComment",
        ));
    }
    if lower.contains("from_raw_parts(") {
        return Some(finding(
            "rust.unsafe.raw-parts",
            "from_raw_parts",
            file,
            line_no,
            line,
            "raw ownership conversion needs exact provenance and allocator proof",
            "ownership provenance is missing",
            "use the matching constructor/destructor pair or add a documented ownership proof",
            "NearbySafetyComment",
        ));
    }
    if lower.contains("static mut") {
        return Some(finding(
            "rust.unsafe.static-mut",
            "static mut",
            file,
            line_no,
            line,
            "static mut exposes unsynchronized global mutation",
            "global mutation proof is missing",
            "replace the mutable static with explicit synchronization or scoped ownership",
            "NearbySafetyComment",
        ));
    }
    if lower.contains("repr(packed)") {
        return Some(finding(
            "rust.unsafe.repr-packed",
            "repr(packed)",
            file,
            line_no,
            line,
            "repr(packed) makes alignment-sensitive references dangerous",
            "alignment proof is missing",
            "remove packed layout or access fields through unaligned-safe primitives",
            "NearbySafetyComment",
        ));
    }
    if shell_command_is_dynamic(&lower) && !scan::is_fixed_safe_command_invocation(line) {
        return Some(finding(
            "rust.security.shell-c-dynamic",
            "shell execution",
            file,
            line_no,
            line,
            "shell execution with dynamic command text is unsafe by default",
            "shell command text is not proven safe",
            "pass argv values directly or use a bounded, allowlisted command path",
            "NearbyAsyncContext",
        ));
    }
    if lower.contains("unbounded_channel(") || lower.contains("unbounded(") {
        return Some(finding(
            "rust.async.unbounded-channel",
            "unbounded_channel",
            file,
            line_no,
            line,
            "unbounded channel or task creation needs explicit backpressure proof",
            "backpressure proof is missing",
            "use a bounded channel or document the queue bound and shutdown path",
            "NearbyAsyncContext",
        ));
    }
    if lower.contains("block_on(") && scan::function_context_contains_async(full_text, line_no) {
        return Some(finding(
            "rust.async.block_on-in-async",
            "block_on",
            file,
            line_no,
            line,
            "blocking inside async context can stall the executor",
            "async context proof is missing",
            "move the work out of async context or use an async-native path",
            "NearbyAsyncContext",
        ));
    }

    None
}

fn lint_suppression_hits(ctx: &AuditContext) -> Vec<LanguageFinding> {
    let mut out = Vec::new();
    for file in rust_files(ctx) {
        let lower = file.text.to_ascii_lowercase();
        let mut matched_term = None;
        if lower.contains("#![allow(warnings)]")
            || lower.contains("#[allow(warnings)]")
            || lower.contains("allow(warnings)")
        {
            matched_term = Some("rust.supply.allow-warnings");
        } else if lower.contains("allow(clippy::all)") {
            matched_term = Some("rust.supply.allow-clippy-all");
        } else if lower.contains("allow(clippy::correctness)") {
            matched_term = Some("rust.supply.allow-clippy-correctness");
        } else if lower.contains("rustflags") && lower.contains("-a warnings") {
            matched_term = Some("rust.supply.rustflags-warn-suppression");
        }
        if let Some(term) = matched_term {
            let line = file
                .text
                .lines()
                .position(|candidate| {
                    candidate
                        .to_ascii_lowercase()
                        .contains(term.split('.').next_back().unwrap_or(term))
                })
                .map(|idx| idx + 1);
            let text = line
                .and_then(|line_no| {
                    file.text
                        .lines()
                        .nth(line_no - 1)
                        .map(|l| l.trim().to_string())
                })
                .unwrap_or_else(|| file.text.lines().next().unwrap_or("").trim().to_string());
            out.push(finding(
                "rust.supply.lint-suppression",
                term,
                &file,
                line,
                &text,
                "broad lint suppression hides review-relevant issues",
                "lint suppression proof is missing",
                "tighten the lint scope or remove the allow entirely",
                "NearbySafetyDocs",
            ));
            break;
        }
    }
    out
}

fn advisory_hit_for_line(
    file: &crate::model::FileInfo,
    line_no: usize,
    line: &str,
) -> Option<LanguageFinding> {
    let lower = line.to_ascii_lowercase();
    if lower.contains("clone(") || lower.contains(".clone()") {
        return Some(finding(
            "rust.review.clone-overuse",
            "clone",
            file,
            line_no,
            line,
            "clone may be hiding ownership or performance drift",
            "clone usage is review-worthy",
            "prefer borrowing or move semantics when the copy is not essential",
            "None",
        ));
    }
    if lower.contains("arc<mutex<") || lower.contains("arc<tokio::sync::mutex<") {
        return Some(finding(
            "rust.review.arc-mutex-default",
            "Arc<Mutex<_>>",
            file,
            line_no,
            line,
            "Arc<Mutex<_>> may be a default ownership escape hatch",
            "shared mutable state is review-worthy",
            "consider explicit ownership or a bounded message-passing boundary",
            "None",
        ));
    }
    if lower.contains("rc<refcell<") {
        return Some(finding(
            "rust.review.rc-refcell-default",
            "Rc<RefCell<_>>",
            file,
            line_no,
            line,
            "Rc<RefCell<_>> may be hiding interior mutability",
            "interior mutability is review-worthy",
            "encode the ownership model more directly or document the single-threaded invariant",
            "None",
        ));
    }
    if lower.contains(" as ") {
        return Some(finding(
            "rust.review.as-cast",
            "as cast",
            file,
            line_no,
            line,
            "broad as-casts can hide truncation or type dishonesty",
            "cast proof is review-worthy",
            "use TryFrom, helper newtypes, or a proof-oriented conversion",
            "None",
        ));
    }
    if lower.contains("repr(c)") || lower.contains("repr(c,") {
        return Some(finding(
            "rust.review.repr-c",
            "repr(C)",
            file,
            line_no,
            line,
            "repr(C) is a contract surface that needs review",
            "ABI layout proof is review-worthy",
            "document the ABI and ownership story next to the type definition",
            "None",
        ));
    }
    if lower.contains("pin<")
        || lower.contains("pin::new_unchecked")
        || lower.contains("pin_project")
    {
        return Some(finding(
            "rust.review.pin",
            "Pin",
            file,
            line_no,
            line,
            "Pin usage deserves a locality proof even when it is valid",
            "pinning proof is review-worthy",
            "document the pinning contract and keep the projection code small",
            "None",
        ));
    }
    if lower.contains("atomic") || lower.contains("ordering::relaxed") {
        return Some(finding(
            "rust.review.atomics",
            "atomics",
            file,
            line_no,
            line,
            "atomics need an explicit ordering story",
            "memory-order proof is review-worthy",
            "document the memory-order invariant next to the atomic",
            "None",
        ));
    }
    if lower.contains("unwrap(") || lower.contains("expect(") {
        return Some(finding(
            "rust.review.generic-unwrap",
            "unwrap/expect",
            file,
            line_no,
            line,
            "generic unwrap/expect in non-test Rust code deserves a proof check",
            "panic path is review-worthy",
            "replace the panic path with a typed error or a documented invariant",
            "None",
        ));
    }
    None
}

fn shell_command_is_dynamic(lower: &str) -> bool {
    (lower.contains("command::new(\"sh\"")
        || lower.contains("command::new(\"bash\"")
        || lower.contains("command::new(\"/bin/sh\"")
        || lower.contains("command::new(\"/bin/bash\""))
        && lower.contains(".arg(\"-c\")")
        && (lower.contains("user_input")
            || lower.contains("command_text")
            || lower.contains("shell_command")
            || lower.contains("format!(")
            || lower.contains("script")
            || lower.contains("cmd"))
}

// Rust rule findings keep detector, source, proof-window, and repair text explicit.
#[allow(clippy::too_many_arguments)]
fn finding(
    matched_term: &'static str,
    detector_id: &'static str,
    file: &crate::model::FileInfo,
    line_no: impl Into<Option<usize>>,
    line: &str,
    problem: &str,
    reason: &str,
    agent_fix: &str,
    proof_window: &'static str,
) -> LanguageFinding {
    let snippet = line.trim().chars().take(160).collect::<String>();
    LanguageFinding::new(
        HLT_RULE_ID,
        matched_term,
        file.rel_path.clone(),
        line_no.into(),
        snippet.clone(),
        problem,
        reason,
        agent_fix,
        vec![
            format!("detector={detector_id}"),
            format!("proof-window={proof_window}"),
            format!("snippet={snippet}"),
        ],
    )
}
