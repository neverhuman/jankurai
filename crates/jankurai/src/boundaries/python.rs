pub const ALLOWED_PRODUCT_TRUTH_PATHS: &[&str] = &["python/ai-service"];

pub fn allowed_product_truth_path(path: &str) -> bool {
    ALLOWED_PRODUCT_TRUTH_PATHS
        .iter()
        .any(|prefix| path == *prefix || path.starts_with(&format!("{prefix}/")))
}
