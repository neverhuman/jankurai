//! The `guard run` launcher. It opens a PTY pair, spawns the agent command into
//! the slave (its own session/controlling tty), and relays I/O between our
//! terminal and the PTY master. Our terminal is put in raw mode for the
//! duration and restored on exit. SIGINT/SIGTERM are forwarded to the child and
//! SIGWINCH triggers a PTY resize. The watcher backend runs in a background
//! thread so the guard's audit feedback is injected inline.

use crate::audit_client::AuditClient;
use crate::feedback::DenialBus;
use crate::layout::GuardLayout;
use crate::policy::GuardPolicy;
use crate::pty::injector::BannerInjector;
use crate::watch::{WatcherBackend, WatcherHandle};
use anyhow::{anyhow, Context, Result};
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::ffi::OsString;
use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// What to launch and how.
pub struct LaunchSpec {
    /// Resolved guard layout for the repository.
    pub layout: GuardLayout,
    /// Effective guard policy.
    pub policy: GuardPolicy,
    /// The agent command and its arguments (the part after the trailing `--`).
    pub agent: Vec<OsString>,
    /// Whether the watcher backend should use PollWatcher instead of the native watcher.
    pub poll: bool,
    /// A stable session id, exported to the agent as `JANKURAI_GUARD_SESSION`.
    pub session_id: String,
}

/// The result of a finished agent session.
pub struct AgentSession {
    /// The agent's process exit code.
    pub exit_code: i32,
}

/// Runs the agent under a guarded PTY. Blocks until the agent exits, then
/// returns its exit code. The watcher backend and banner injector are torn down
/// before returning.
pub fn run_agent(spec: LaunchSpec, audit: Arc<dyn AuditClient>) -> Result<AgentSession> {
    if spec.agent.is_empty() {
        return Err(anyhow!("no agent command was provided after `--`"));
    }
    spec.layout
        .ensure_dirs()
        .context("preparing guard state directories")?;

    let bus = Arc::new(DenialBus::new());

    // Start the watcher backend in the background so writes are guarded while
    // the agent runs. Snapshots are primed first so existing files can revert.
    let watcher_handle = WatcherHandle::default();
    let backend = WatcherBackend::new(
        spec.layout.clone(),
        spec.policy.clone(),
        Arc::clone(&audit),
        Arc::clone(&bus),
    )?;
    let mut primed = backend;
    primed
        .prime_snapshots()
        .context("priming last-good snapshots")?;
    let watcher_thread = {
        let handle = watcher_handle.clone();
        let poll = spec.poll;
        std::thread::spawn(move || {
            let _ = primed.run(handle, poll);
        })
    };

    let pty_size = current_pty_size();
    let pair = native_pty_system()
        .openpty(pty_size)
        .map_err(|e| anyhow!("opening pty: {e}"))?;

    let mut cmd = CommandBuilder::from_argv(spec.agent.clone());
    cmd.cwd(&spec.layout.mount);
    cmd.env("JANKURAI_GUARD_SESSION", &spec.session_id);
    let mut child = pair
        .slave
        .spawn_command(cmd)
        .map_err(|e| anyhow!("spawning agent: {e}"))?;
    // The slave handle is no longer needed by us once the child holds it.
    drop(pair.slave);

    let killer = child.clone_killer();
    install_signal_state();

    let injector = BannerInjector::spawn(
        &bus,
        pair.master
            .take_writer()
            .map_err(|e| anyhow!("pty writer for injector: {e}"))?,
    );

    let raw_guard = RawModeGuard::enter();
    let exit_code = relay_loop(&*pair.master, child.as_mut(), killer)?;
    drop(raw_guard);

    // Tearing down: stop the watcher, then drop the master so the injector's
    // receiver loop ends, then join everything.
    watcher_handle.stop();
    drop(pair.master);
    drop(bus);
    injector.join();
    let _ = watcher_thread.join();

    Ok(AgentSession { exit_code })
}

/// Runs the bidirectional relay until the child exits. Returns the exit code.
fn relay_loop(
    master: &dyn portable_pty::MasterPty,
    child: &mut dyn portable_pty::Child,
    mut killer: Box<dyn portable_pty::ChildKiller + Send + Sync>,
) -> Result<i32> {
    let mut reader = master
        .try_clone_reader()
        .map_err(|e| anyhow!("pty reader: {e}"))?;
    let mut writer = master
        .take_writer()
        .map_err(|e| anyhow!("pty writer: {e}"))?;

    // master -> our stdout
    let out_thread = std::thread::spawn(move || {
        let mut buf = [0u8; 8192];
        let mut stdout = std::io::stdout();
        loop {
            match reader.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    if stdout.write_all(&buf[..n]).is_err() || stdout.flush().is_err() {
                        break;
                    }
                }
            }
        }
    });

    // our stdin -> master
    let stop_in = Arc::new(AtomicBool::new(false));
    let in_stop = Arc::clone(&stop_in);
    let in_thread = std::thread::spawn(move || {
        let mut buf = [0u8; 8192];
        let mut stdin = std::io::stdin();
        while !in_stop.load(Ordering::SeqCst) {
            match stdin.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    if writer.write_all(&buf[..n]).is_err() || writer.flush().is_err() {
                        break;
                    }
                }
            }
        }
    });

    let status = loop {
        if let Some(signal) = take_pending_signal() {
            // Forward an interrupt/terminate to the child; SIGWINCH resizes.
            if signal == libc::SIGWINCH {
                let _ = master.resize(current_pty_size());
            } else {
                let _ = killer.kill();
            }
        }
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => std::thread::sleep(std::time::Duration::from_millis(40)),
            Err(e) => return Err(anyhow!("waiting on agent: {e}")),
        }
    };

    stop_in.store(true, Ordering::SeqCst);
    let _ = out_thread.join();
    let _ = in_thread.join();
    Ok(status.exit_code() as i32)
}

/// Reads the controlling terminal's size, defaulting to a sane value when the
/// terminal size cannot be determined.
fn current_pty_size() -> PtySize {
    // Construct winsize with explicit zero fields; ioctl may overwrite them.
    let mut ws = libc::winsize {
        ws_row: 0,
        ws_col: 0,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    // SAFETY: STDIN_FILENO is always open; `ws` is a valid, aligned buffer.
    // TIOCGWINSZ writes all fields; failures are detected via the return value.
    let ok = unsafe { libc::ioctl(libc::STDIN_FILENO, libc::TIOCGWINSZ, &mut ws) } == 0;
    if ok && ws.ws_col > 0 && ws.ws_row > 0 {
        PtySize {
            rows: ws.ws_row,
            cols: ws.ws_col,
            pixel_width: ws.ws_xpixel,
            pixel_height: ws.ws_ypixel,
        }
    } else {
        PtySize::default()
    }
}

/// A pending-signal flag set by the installed handlers.
static PENDING_SIGNAL: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(0);

/// The signal handler: it only records the signal number; the relay loop acts
/// on it. This keeps the handler async-signal-safe.
extern "C" fn handle_signal(sig: libc::c_int) {
    PENDING_SIGNAL.store(sig, Ordering::SeqCst);
}

/// Installs handlers for SIGINT, SIGTERM and SIGWINCH.
fn install_signal_state() {
    for sig in [libc::SIGINT, libc::SIGTERM, libc::SIGWINCH] {
        // SAFETY: `handle_signal` has the correct `extern "C"` signal-handler
        // signature and only performs an atomic store (async-signal-safe).
        unsafe {
            libc::signal(sig, handle_signal as *const () as libc::sighandler_t);
        }
    }
}

/// Returns and clears the most recently received signal, if any.
fn take_pending_signal() -> Option<libc::c_int> {
    let sig = PENDING_SIGNAL.swap(0, Ordering::SeqCst);
    if sig == 0 {
        None
    } else {
        Some(sig)
    }
}

/// Puts the controlling terminal into raw mode and restores the original
/// settings when dropped.
struct RawModeGuard {
    original: Option<libc::termios>,
}

impl RawModeGuard {
    /// Enters raw mode. When stdin is not a tty this is a no-op guard.
    fn enter() -> Self {
        // SAFETY: `isatty` is always safe to call on any file descriptor;
        // it returns 0 when the fd is not a terminal rather than causing UB.
        if unsafe { libc::isatty(libc::STDIN_FILENO) } != 1 {
            return Self { original: None };
        }
        // Use MaybeUninit to allocate storage without an invalid intermediate state:
        // tcgetattr writes every field before we read any of them.
        let mut uninit = std::mem::MaybeUninit::<libc::termios>::uninit();
        // SAFETY: `STDIN_FILENO` is always open, and `uninit.as_mut_ptr()` is
        // a valid, properly-aligned pointer to a `libc::termios`-sized buffer.
        // On error we detect it via the return value and skip raw mode.
        if unsafe { libc::tcgetattr(libc::STDIN_FILENO, uninit.as_mut_ptr()) } != 0 {
            return Self { original: None };
        }
        // SAFETY: tcgetattr returned 0, guaranteeing every field is initialized.
        // ptr::read copies the now-valid bytes into an owned termios value.
        let mut termios = unsafe { std::ptr::read(uninit.as_ptr()) };
        let original = termios;
        // SAFETY: `cfmakeraw` modifies the `termios` struct in place using
        // well-defined POSIX semantics; `termios` was populated by `tcgetattr`.
        unsafe { libc::cfmakeraw(&mut termios) };
        // SAFETY: `STDIN_FILENO` is open and is a tty (confirmed by `isatty`
        // above). `TCSANOW` applies the change immediately.  `termios` was
        // produced by `cfmakeraw` on a valid attribute set from `tcgetattr`.
        unsafe {
            libc::tcsetattr(libc::STDIN_FILENO, libc::TCSANOW, &termios);
        }
        Self {
            original: Some(original),
        }
    }
}

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        if let Some(original) = self.original {
            // SAFETY: `original` is from a successful `tcgetattr` in `enter`,
            // so every field is valid.  STDIN_FILENO is always open.
            unsafe {
                libc::tcsetattr(libc::STDIN_FILENO, libc::TCSANOW, &original);
            }
        }
    }
}
