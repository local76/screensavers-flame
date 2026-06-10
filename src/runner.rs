//! Cross-platform screensaver runtime.
//!
//! **Taxonomy Classification**: Lifecycle (Foreground) — the runtime
//! loop that owns the GDI screensaver window on Windows and the raw
//! terminal on Linux/macOS, dispatches the Screensaver::update /
//! Screensaver::draw cycle, and handles the screensaver CLI args
//! (`/s` run, `/c` configure, `/p HWND` preview).
//!
//! In library 4.1 this runtime lived in `screensavers/src/trance-core/` as a
//! separate crate that re-exported `library`. In library 4.2 the
//! runtime is consolidated here so the 10 r* effect crates in
//! screensavers/ can become 1-line shim binaries via the
//! [`screensaver_shim!`] macro:
//!
//! ```ignore
//! library::screensaver_shim!(glyphs, Glyphs, "glyphs");
//! ```
//!
//! The `screensaver-runtime` feature is **default-off** in library
//! (it pulls in `libc` on non-Windows for the raw-termios terminal
//! guard, and on Windows the full Win32 GDI windowing stack). The
//! 10 r* binary crates enable this feature; the 7 r* TUI apps and
//! other library consumers that don't need a screensaver host loop
//! do not.
//!
//! The Windows path here is currently a **scaffold-only stub**. It
//! parses the screensaver CLI args correctly but does not yet
//! implement the full Win32 GDI window loop (HWND creation, WndProc,
//! timeBeginPeriod(1), GDI BitBlt to HDC, per-monitor DPI awareness,
//! preview-mode static control subclassing). The full Windows port
//! is tracked as a follow-up to 4.2; the library surface here is
//! stable so the Linux path can be wired into the 10 shim binaries
//! immediately and the Windows path lands incrementally.

#[cfg(not(target_os = "windows"))]
use std::time::Duration;
#[cfg(not(target_os = "windows"))]
use std::io::Write;

#[cfg(not(target_os = "windows"))]
use library::core::TerminalCell;
use library::core::screensaver::Screensaver;

/// CLI mode detected from the screensaver args.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// `/s` — run the screensaver fullscreen
    Run,
    /// `/c` — open the configuration dialog
    Configure,
    /// `/p <HWND>` — render into the given preview HWND
    Preview,
    /// No args (or `-h` / `--help`) — print usage
    ShowUsage,
}

/// Parse the standard screensaver CLI args. On Windows the full
/// `HWND` is also extracted (for `/p:<hwnd>` and `/c:<hwnd>`); the
/// Linux/non-Windows stub doesn't need an HWND so it just classifies
/// the mode.
pub fn parse_args() -> Mode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    
    // Filter out our OpenRGB arguments so they don't trigger the usage/error parser
    let filtered_args: Vec<String> = args.into_iter()
        .filter(|arg| arg != "--enable-openrgb" && arg != "/rgb")
        .collect();

    if filtered_args.is_empty() {
        return Mode::ShowUsage;
    }
    for arg in &filtered_args {
        let lower = arg.to_lowercase();
        if lower == "/s" || lower == "-s" {
            return Mode::Run;
        }
        if lower.starts_with("/p") || lower.starts_with("-p") {
            return Mode::Preview;
        }
        if lower.starts_with("/c") || lower.starts_with("-c") {
            return Mode::Configure;
        }
        if lower == "/?" || lower == "-h" || lower == "--help" {
            return Mode::ShowUsage;
        }
        if arg.starts_with('/') || arg.starts_with('-') {
            return Mode::ShowUsage;
        }
    }
    Mode::ShowUsage
}

/// Print the standard screensaver usage banner for the given name.
pub fn print_usage(name: &str) {
    eprintln!("{name} — retro screensaver (library 4.2 screensaver_runtime)");
    eprintln!();
    eprintln!("Usage (Windows):");
    eprintln!("  {name}.scr /s           — run fullscreen");
    eprintln!("  {name}.scr /c           — open configuration dialog");
    eprintln!("  {name}.scr /p <HWND>    — preview inside the given parent HWND");
    eprintln!();
    eprintln!("Usage (Linux / other):");
    eprintln!("  {name}                  — run fullscreen in the current terminal");
    eprintln!("  {name} -h | --help      — this help");
    eprintln!();
    eprintln!("Options:");
    eprintln!("  --enable-openrgb | /rgb — enable keyboard/device RGB lighting controls via OpenRGB");
}

/// Run the screensaver with the given effect. Picks the platform
/// implementation based on `cfg(target_os)`. The 10 r* effect
/// binaries in screensavers/ all call this.
pub fn run_main<S: Screensaver + 'static>(saver: S, name: &str) {
    let mode = parse_args();
    match mode {
        Mode::Run => {
            let code = run_fullscreen(saver);
            std::process::exit(code as i32);
        }
        Mode::Configure => {
            // Configuration dialogs live in library 4.3; for 4.2 the
            // configure mode just prints a notice and exits 0.
            eprintln!("({name}) configuration dialog: not yet implemented in library 4.2.");
            eprintln!("Edit `HKEY_CURRENT_USER\\Software\\Windows-Screensavers\\{name}`");
            eprintln!("directly to change options. (4.3 will add a CLI/TUI configurator.)");
            std::process::exit(0);
        }
        Mode::Preview => {
            // Windows-only in 4.2. On non-Windows we run fullscreen
            // anyway as a fallback so the dev/CI path works.
            #[cfg(target_os = "windows")]
            {
                let code = run_preview_stub(saver);
                std::process::exit(code as i32);
            }
            #[cfg(not(target_os = "windows"))]
            {
                let code = run_fullscreen(saver);
                std::process::exit(code as i32);
            }
        }
        Mode::ShowUsage => {
            print_usage(name);
            std::process::exit(0);
        }
    }
}

// ---------------------------------------------------------------------------
// Windows scaffold
// ---------------------------------------------------------------------------

#[cfg(target_os = "windows")]
fn run_fullscreen<S: Screensaver + 'static>(_saver: S) -> isize {
    // library 4.2 scaffold: the full Win32 GDI screensaver window loop
    // (HWND + WndProc + timeBeginPeriod(1) + BitBlt + per-monitor DPI
    // awareness + preview-mode static control subclassing) is the
    // follow-up work tracked in the 4.3 milestone. The 4.2 runtime
    // ships the CLI parser + arg routing + the public run_main API
    // surface so the 10 r* effect shim binaries can be wired up.
    eprintln!("(library 4.2 screensaver_runtime) Windows GDI loop is a 4.3 TODO.");
    eprintln!("Falling back to no-op for the 4.2 release. Use the screensavers");
    eprintln!("0.1.x / 1.x r* effect crate binaries on Windows until 4.3 lands.");
    0
}

#[cfg(target_os = "windows")]
fn run_preview_stub<S: Screensaver + 'static>(_saver: S) -> isize {
    eprintln!("(library 4.2 screensaver_runtime) Windows preview mode is a 4.3 TODO.");
    0
}

// ---------------------------------------------------------------------------
// Linux / other (raw-termios terminal)
// ---------------------------------------------------------------------------

#[cfg(not(target_os = "windows"))]
fn command_exists(cmd: &str) -> bool {
    std::process::Command::new("sh")
        .arg("-c")
        .arg(format!("command -v {} >/dev/null 2>&1", cmd))
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

#[cfg(not(target_os = "windows"))]
fn run_fullscreen<S: Screensaver + 'static>(mut saver: S) -> isize {
    // If running under xscreensaver, embed the terminal window inside the target X11 window.
    if let Ok(win_id) = std::env::var("XSCREENSAVER_WINDOW") {
        if std::env::var("TRANCE_EMBEDDED").is_err() {
            if command_exists("xterm") {
                let self_exe = std::env::current_exe().unwrap_or_default();
                let mut cmd = std::process::Command::new("xterm");
                cmd.args(["-into", &win_id, "-geometry", "120x40", "-e"])
                    .arg(&self_exe)
                    .arg("/s")
                    .env("TRANCE_EMBEDDED", "1");
                if let Ok(mut child) = cmd.spawn() {
                    let _ = child.wait();
                    return 0;
                }
            }
        }
    }

    let _raw_mode = RawTerminalGuard::enable();
    let (mut cols, mut rows) = get_terminal_size();
    // The Screensaver trait's update/draw are the only required
    // methods; init() is optional with a default no-op via
    // ScreensaverState. Call it if the saver opted in.
    saver.init(cols, rows);

    let mut renderer = Renderer::new(cols, rows);
    let mut grid = vec![TerminalCell::default(); cols * rows];
    let mut last_frame = std::time::Instant::now();

    // Target ~60 FPS for terminal smooth animations
    let target_fps = 60u32;
    let frame_duration = Duration::from_secs_f32(1.0 / target_fps as f32);

    loop {
        if check_keypress() {
            break;
        }

        // Handle terminal resize dynamically
        let (new_cols, new_rows) = get_terminal_size();
        if new_cols != cols || new_rows != rows {
            cols = new_cols;
            rows = new_rows;
            grid = vec![TerminalCell::default(); cols * rows];
            saver.init(cols, rows);
            renderer = Renderer::new(cols, rows);
        }

        let now = std::time::Instant::now();
        let dt = now.duration_since(last_frame);
        last_frame = now;

        saver.update(dt, cols, rows);

        for cell in &mut grid {
            *cell = TerminalCell::default();
        }
        saver.draw(&mut grid, cols, rows);
        renderer.render_grid(&grid, cols, rows, saver.has_scanlines());

        let elapsed = now.elapsed();
        if elapsed < frame_duration {
            std::thread::sleep(frame_duration - elapsed);
        }
    }
    0
}

// ---------------------------------------------------------------------------
// Linux raw-termios terminal plumbing
// ---------------------------------------------------------------------------

#[cfg(not(target_os = "windows"))]
struct RawTerminalGuard {
    original_termios: libc::termios,
}

#[cfg(not(target_os = "windows"))]
impl RawTerminalGuard {
    fn enable() -> Option<Self> {
        unsafe {
            let mut termios: libc::termios = std::mem::zeroed();
            if libc::tcgetattr(libc::STDIN_FILENO, &mut termios) != 0 {
                return None;
            }
            let original_termios = termios;
            libc::cfmakeraw(&mut termios);
            if libc::tcsetattr(libc::STDIN_FILENO, libc::TCSANOW, &termios) != 0 {
                return None;
            }
            Some(Self { original_termios })
        }
    }
}

#[cfg(not(target_os = "windows"))]
impl Drop for RawTerminalGuard {
    fn drop(&mut self) {
        unsafe {
            libc::tcsetattr(libc::STDIN_FILENO, libc::TCSANOW, &self.original_termios);
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn get_terminal_size() -> (usize, usize) {
    unsafe {
        let mut ws: libc::winsize = std::mem::zeroed();
        if libc::ioctl(libc::STDOUT_FILENO, libc::TIOCGWINSZ, &mut ws) == 0 {
            (ws.ws_col as usize, ws.ws_row as usize)
        } else {
            (80, 24)
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn check_keypress() -> bool {
    unsafe {
        let mut fd_set: libc::fd_set = std::mem::zeroed();
        libc::FD_SET(libc::STDIN_FILENO, &mut fd_set);

        let mut timeout = libc::timeval {
            tv_sec: 0,
            tv_usec: 0,
        };

        let result = libc::select(
            libc::STDIN_FILENO + 1,
            &mut fd_set,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            &mut timeout,
        );

        result > 0
    }
}

// ---------------------------------------------------------------------------
// Differential renderer (only redraws cells that changed)
// ---------------------------------------------------------------------------

#[cfg(not(target_os = "windows"))]
struct Renderer {
    _width: usize,
    _height: usize,
    prev_grid: Vec<TerminalCell>,
}

#[cfg(not(target_os = "windows"))]
impl Renderer {
    fn new(width: usize, height: usize) -> Self {
        // Hide cursor and clear screen
        print!("\x1b[?25l\x1b[2J");
        let _ = std::io::stdout().flush();
        Self {
            _width: width,
            _height: height,
            prev_grid: vec![TerminalCell::default(); width * height],
        }
    }

    fn render_grid(&mut self, grid: &[TerminalCell], cols: usize, rows: usize, _has_scanlines: bool) {
        use std::io::Write;
        let mut stdout = std::io::stdout().lock();
        let mut current_fg: Option<(u8, u8, u8)> = None;
        let mut current_bg: Option<(u8, u8, u8)> = None;
        let mut current_bold = false;

        for r in 0..rows {
            for c in 0..cols {
                let idx = r * cols + c;
                if idx >= grid.len() || idx >= self.prev_grid.len() {
                    continue;
                }

                let new_cell = grid[idx];
                let old_cell = self.prev_grid[idx];

                if new_cell != old_cell {
                    let _ = write!(stdout, "\x1b[{};{}H", r + 1, c + 1);

                    if Some(new_cell.fg) != current_fg {
                        let _ = write!(stdout, "\x1b[38;2;{};{};{}m", new_cell.fg.0, new_cell.fg.1, new_cell.fg.2);
                        current_fg = Some(new_cell.fg);
                    }

                    if Some(new_cell.bg) != current_bg {
                        let _ = write!(stdout, "\x1b[48;2;{};{};{}m", new_cell.bg.0, new_cell.bg.1, new_cell.bg.2);
                        current_bg = Some(new_cell.bg);
                    }

                    if new_cell.bold != current_bold {
                        if new_cell.bold {
                            let _ = write!(stdout, "\x1b[1m");
                        } else {
                            let _ = write!(stdout, "\x1b[22m");
                        }
                        current_bold = new_cell.bold;
                    }

                    let _ = write!(stdout, "{}", new_cell.ch);
                    self.prev_grid[idx] = new_cell;
                }
            }
        }
        let _ = stdout.flush();
    }
}

#[cfg(not(target_os = "windows"))]
impl Drop for Renderer {
    fn drop(&mut self) {
        // Reset colors, show cursor, clear screen, and go home
        print!("\x1b[0m\x1b[?25h\x1b[2J\x1b[H");
        let _ = std::io::stdout().flush();
    }
}

/// Generate a Windows-screensaver-compatible `main()` for the given
/// scene. Place at the root of `src/main.rs` (or any file you point
/// Cargo's `[[bin]] path` at).
///
/// Collapses the 4-line `windows_subsystem` cfg-attr + `fn main()`
/// boilerplate that each of the 10 `screensavers-*` shim binaries
/// needs into a single macro invocation.
///
/// # Example
///
/// In `screensavers-glyphs/src/screensaver_shim.rs`:
///
/// ```rust,ignore
/// library::screensaver_shim!(glyphs, Glyphs, "glyphs");
/// ```
///
/// This expands to a 2-line module:
///
/// ```rust,ignore
/// #[cfg_attr(
///     all(not(debug_assertions), target_os = "windows"),
///     windows_subsystem = "windows"
/// )]
/// fn main() {
///     library::screensavers::runtime::run_main(
///         library::screensavers::glyphs::Glyphs::new(),
///         "glyphs",
///     );
/// }
/// ```
///
/// The `windows_subsystem` attribute is attached to `fn main()` as an
/// **outer** attribute (not a crate-level inner attribute) so the
/// macro is parseable in any module context, not just at the crate
/// root. The linker still picks it up the same way because the
/// `fn main()` of a `[[bin]]` is the entry point.
///
/// The scene module must export `<SceneTy>::new()` returning a type
/// that implements `Screensaver + 'static`. All 10 scenes in
/// `library::screensavers` (`Beams`, `Bounce`, `Bursts`, `Chaos`,
/// `Cosmos`, `Disco`, `Flame`, `Glyphs`, `Gnats`, `Storm`) do.
#[macro_export]
macro_rules! screensaver_shim {
    ($scene_mod:ident, $scene_ty:ident, $name:literal) => {
        #[cfg_attr(
            all(not(debug_assertions), target_os = "windows"),
            windows_subsystem = "windows"
        )]
        fn main() {
            $crate::screensavers::runtime::run_main(
                $crate::screensavers::$scene_mod::$scene_ty::new(),
                $name,
            );
        }
    };
}
