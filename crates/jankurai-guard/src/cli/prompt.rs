//! First-run FUSE availability notice. Shown once, then suppressed via a
//! marker file at `~/.jankurai/guard-fuse-prompted`.

use crate::layout::jankurai_home;

/// Shows a platform-specific FUSE setup notice on the first guard invocation
/// when FUSE is not available in this build. Writes a marker file so the
/// notice appears only once regardless of subcommand.
pub fn maybe_show_fuse_prompt() {
    let home = match jankurai_home() {
        Ok(h) => h,
        Err(_) => return,
    };
    let marker = home.join("guard-fuse-prompted");
    if marker.exists() {
        return;
    }
    if !crate::fuse::fuse_available() {
        print_fuse_notice();
    }
    let _ = std::fs::create_dir_all(&home);
    let _ = std::fs::write(&marker, "");
}

/// Removes the first-run marker so the notice is shown again on the next
/// guard invocation. Useful after FUSE is installed and the user wants
/// confirmation the binary detects it.
#[allow(dead_code)]
pub fn reset_fuse_prompt() {
    if let Ok(home) = jankurai_home() {
        let _ = std::fs::remove_file(home.join("guard-fuse-prompted"));
    }
}

fn print_fuse_notice() {
    #[cfg(target_os = "macos")]
    eprintln!(
        "\n\
         ┌─────────────────────────────────────────────────────────────┐\n\
         │  jankurai guard — FUSE not installed                        │\n\
         │                                                              │\n\
         │  For pre-write blocking on macOS, install macFUSE:          │\n\
         │    brew install --cask macfuse                               │\n\
         │                                                              │\n\
         │  After install: System Settings → Privacy & Security        │\n\
         │  → approve the macFUSE kernel extension, then restart.      │\n\
         │                                                              │\n\
         │  Watcher mode is active (detects and reverts writes).       │\n\
         │  This notice appears once. Re-run after FUSE install.       │\n\
         └─────────────────────────────────────────────────────────────┘\n"
    );
    #[cfg(target_os = "linux")]
    eprintln!(
        "\n\
         ┌─────────────────────────────────────────────────────────────┐\n\
         │  jankurai guard — FUSE dev library not found                │\n\
         │                                                              │\n\
         │  For pre-write blocking, install libfuse3 and rebuild:      │\n\
         │    sudo apt-get install libfuse3-dev                         │\n\
         │    cargo install jankurai --features guard-fuse             │\n\
         │                                                              │\n\
         │  Watcher mode is active (detects and reverts writes).       │\n\
         │  This notice appears once. Re-run after rebuild.            │\n\
         └─────────────────────────────────────────────────────────────┘\n"
    );
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    eprintln!(
        "\n\
         ┌──────────────────────────────────────────────────────────────┐\n\
         │  jankurai guard — FUSE not available on this platform        │\n\
         │  Watcher mode is active (detects and reverts writes).        │\n\
         │  This notice appears once.                                   │\n\
         └──────────────────────────────────────────────────────────────┘\n"
    );
}
