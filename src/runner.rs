//! Cross-platform screensaver runtime.
//!
//! **Taxonomy Classification**: Lifecycle (Foreground) — the runtime
//! loop that owns the console terminal on Windows, Linux, and macOS,
//! dispatches the Screensaver::update / Screensaver::draw cycle, and
//! handles screensaver CLI args (`/s` run, `/c` configure, `/p HWND` preview).

use std::time::Duration;
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
/// `HWND` is also classified (for `/p:<hwnd>` and `/c:<hwnd>`).
pub fn parse_args() -> Mode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    
    // Filter out our OpenRGB arguments so they don't trigger the usage/error parser
    let filtered_args: Vec<String> = args.into_iter()
        .filter(|arg| arg != "--enable-openrgb" && arg != "/rgb")
        .collect();

    if filtered_args.is_empty() {
        return Mode::Run; // Default to Run (fullscreen demo) when double-clicked or run without args
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
    eprintln!("{name} — retro screensaver (screensaver_runtime)");
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

/// Run the screensaver with the given effect.
pub fn run_main<S: Screensaver + 'static>(saver: S, name: &str) {
    let mode = parse_args();
    match mode {
        Mode::Run => {
            let code = run_fullscreen(saver);
            std::process::exit(code as i32);
        }
        Mode::Configure => {
            // Configuration dialog stub
            eprintln!("({name}) configuration dialog: not yet implemented.");
            std::process::exit(0);
        }
        Mode::Preview => {
            // Windows-only preview window stub. Fall back to fullscreen terminal run on other OS.
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
// Windows Console / Terminal support
// ---------------------------------------------------------------------------

#[cfg(target_os = "windows")]
use windows_sys::Win32::System::Console::{
    GetStdHandle, GetConsoleMode, SetConsoleMode, GetConsoleScreenBufferInfo,
    PeekConsoleInputW, CONSOLE_SCREEN_BUFFER_INFO, INPUT_RECORD, KEY_EVENT,
    STD_INPUT_HANDLE, STD_OUTPUT_HANDLE, ENABLE_ECHO_INPUT, ENABLE_LINE_INPUT,
    ENABLE_PROCESSED_INPUT, ENABLE_VIRTUAL_TERMINAL_PROCESSING, AllocConsole,
};

#[cfg(target_os = "windows")]
#[repr(C)]
struct RECT {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

#[cfg(target_os = "windows")]
extern "system" {
    fn timeBeginPeriod(uPeriod: u32) -> u32;
    fn timeEndPeriod(uPeriod: u32) -> u32;
    fn GetDC(hwnd: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
    fn ReleaseDC(hwnd: *mut std::ffi::c_void, hdc: *mut std::ffi::c_void) -> i32;
    fn GetDeviceCaps(hdc: *mut std::ffi::c_void, index: i32) -> i32;
    fn GetCursorPos(lpPoint: *mut windows_sys::Win32::Foundation::POINT) -> i32;
    fn GetAsyncKeyState(vKey: i32) -> i16;
    fn GetConsoleWindow() -> windows_sys::Win32::Foundation::HWND;
    fn GetWindowLongW(hWnd: windows_sys::Win32::Foundation::HWND, nIndex: i32) -> i32;
    fn SetWindowLongW(hWnd: windows_sys::Win32::Foundation::HWND, nIndex: i32, dwNewLong: i32) -> i32;
    fn SetWindowPos(
        hWnd: windows_sys::Win32::Foundation::HWND,
        hWndInsertAfter: windows_sys::Win32::Foundation::HWND,
        X: i32,
        Y: i32,
        cx: i32,
        cy: i32,
        uFlags: u32,
    ) -> i32;
    fn GetSystemMetrics(nIndex: i32) -> i32;
    fn ShowWindow(hWnd: windows_sys::Win32::Foundation::HWND, nCmdShow: i32) -> i32;
    fn GetWindowRect(hWnd: windows_sys::Win32::Foundation::HWND, lpRect: *mut RECT) -> i32;
}

#[cfg(target_os = "windows")]
struct RawTerminalGuard {
    stdin_handle: windows_sys::Win32::Foundation::HANDLE,
    stdout_handle: windows_sys::Win32::Foundation::HANDLE,
    original_in_mode: u32,
    original_out_mode: u32,
    hwnd: windows_sys::Win32::Foundation::HWND,
    original_style: i32,
    original_rect: [i32; 4],
}

#[cfg(target_os = "windows")]
impl RawTerminalGuard {
    fn enable() -> Option<Self> {
        unsafe {
            // Automatically allocate console if we were compiled under windows_subsystem and have none
            AllocConsole();

            let stdin_handle = GetStdHandle(STD_INPUT_HANDLE);
            let stdout_handle = GetStdHandle(STD_OUTPUT_HANDLE);
            if stdin_handle == windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE ||
               stdout_handle == windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE {
                return None;
            }
            
            let mut original_in_mode = 0;
            if GetConsoleMode(stdin_handle, &mut original_in_mode) == 0 {
                return None;
            }
            
            let mut original_out_mode = 0;
            let _ = GetConsoleMode(stdout_handle, &mut original_out_mode);

            // Disable line input, echo input, processed input (similar to cfmakeraw)
            let raw_in_mode = original_in_mode & !(ENABLE_LINE_INPUT | ENABLE_ECHO_INPUT | ENABLE_PROCESSED_INPUT);
            if SetConsoleMode(stdin_handle, raw_in_mode) == 0 {
                return None;
            }

            // Enable virtual terminal processing (ANSI escape sequences) on Windows 10+
            let raw_out_mode = original_out_mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING;
            let _ = SetConsoleMode(stdout_handle, raw_out_mode);

            // Set high-resolution sleep timer
            let _ = timeBeginPeriod(1);

            // Fullscreen console window setup
            let hwnd = GetConsoleWindow();
            let mut original_style = 0;
            let mut original_rect = [0; 4];
            if hwnd != 0 {
                original_style = GetWindowLongW(hwnd, -16); // GWL_STYLE = -16
                let mut rect: RECT = std::mem::zeroed();
                if GetWindowRect(hwnd, &mut rect) != 0 {
                    original_rect = [
                        rect.left,
                        rect.top,
                        rect.right - rect.left,
                        rect.bottom - rect.top,
                    ];
                }

                // Remove borders and title bar
                let new_style = original_style & !(0x00C00000 | 0x00040000 | 0x00020000 | 0x00010000 | 0x00080000); // WS_CAPTION | WS_THICKFRAME | WS_MINIMIZEBOX | WS_MAXIMIZEBOX | WS_SYSMENU
                SetWindowLongW(hwnd, -16, new_style);

                let width = GetSystemMetrics(0); // SM_CXSCREEN = 0
                let height = GetSystemMetrics(1); // SM_CYSCREEN = 1
                SetWindowPos(hwnd, 0, 0, 0, width, height, 0x0020 | 0x0040); // SWP_FRAMECHANGED = 0x0020, SWP_SHOWWINDOW = 0x0040
                ShowWindow(hwnd, 3); // SW_MAXIMIZE = 3
            }

            Some(Self {
                stdin_handle,
                stdout_handle,
                original_in_mode,
                original_out_mode,
                hwnd,
                original_style,
                original_rect,
            })
        }
    }
}

#[cfg(target_os = "windows")]
impl Drop for RawTerminalGuard {
    fn drop(&mut self) {
        unsafe {
            let _ = SetConsoleMode(self.stdin_handle, self.original_in_mode);
            let _ = SetConsoleMode(self.stdout_handle, self.original_out_mode);
            
            // Restore window style and position
            if self.hwnd != 0 {
                SetWindowLongW(self.hwnd, -16, self.original_style);
                let x = self.original_rect[0];
                let y = self.original_rect[1];
                let w = self.original_rect[2];
                let h = self.original_rect[3];
                SetWindowPos(self.hwnd, 0, x, y, w, h, 0x0020 | 0x0040); // SWP_FRAMECHANGED | SWP_SHOWWINDOW
            }

            // End high-resolution timer period
            let _ = timeEndPeriod(1);
        }
    }
}

#[cfg(target_os = "windows")]
fn get_monitor_refresh_rate() -> u32 {
    unsafe {
        let hdc = GetDC(std::ptr::null_mut());
        if !hdc.is_null() {
            let rate = GetDeviceCaps(hdc, 116); // 116 = VREFRESH
            ReleaseDC(std::ptr::null_mut(), hdc);
            if rate > 0 {
                return rate as u32;
            }
        }
    }
    60
}

#[cfg(not(target_os = "windows"))]
fn get_monitor_refresh_rate() -> u32 {
    60
}

#[cfg(target_os = "windows")]
fn check_mouse_activity(initial_pos: &mut Option<(i32, i32)>) -> bool {
    unsafe {
        let mut pt: windows_sys::Win32::Foundation::POINT = std::mem::zeroed();
        if GetCursorPos(&mut pt) != 0 {
            if let Some((init_x, init_y)) = *initial_pos {
                let dx = (pt.x - init_x).abs();
                let dy = (pt.y - init_y).abs();
                if dx > 10 || dy > 10 {
                    return true;
                }
            } else {
                *initial_pos = Some((pt.x, pt.y));
            }
        }

        // Check mouse clicks: VK_LBUTTON = 0x01, VK_RBUTTON = 0x02, VK_MBUTTON = 0x04
        if GetAsyncKeyState(0x01) != 0 || GetAsyncKeyState(0x02) != 0 || GetAsyncKeyState(0x04) != 0 {
            return true;
        }
    }
    false
}

#[cfg(not(target_os = "windows"))]
fn check_mouse_activity(_initial_pos: &mut Option<(i32, i32)>) -> bool {
    false
}

#[cfg(target_os = "windows")]
fn get_terminal_size() -> (usize, usize) {
    unsafe {
        let stdout_handle = GetStdHandle(STD_OUTPUT_HANDLE);
        let mut info: CONSOLE_SCREEN_BUFFER_INFO = std::mem::zeroed();
        if GetConsoleScreenBufferInfo(stdout_handle, &mut info) != 0 {
            let cols = (info.srWindow.Right - info.srWindow.Left + 1) as usize;
            let rows = (info.srWindow.Bottom - info.srWindow.Top + 1) as usize;
            (cols, rows)
        } else {
            (80, 24)
        }
    }
}

#[cfg(target_os = "windows")]
fn check_keypress() -> bool {
    unsafe {
        let stdin_handle = GetStdHandle(STD_INPUT_HANDLE);
        let mut num_events = 0;
        if windows_sys::Win32::System::Console::GetNumberOfConsoleInputEvents(stdin_handle, &mut num_events) == 0 {
            return false;
        }
        if num_events == 0 {
            return false;
        }

        let mut buffer: [INPUT_RECORD; 128] = std::mem::zeroed();
        let mut read = 0;
        if PeekConsoleInputW(stdin_handle, buffer.as_mut_ptr(), num_events.min(128), &mut read) != 0 {
            for i in 0..read as usize {
                let rec = &buffer[i];
                if rec.EventType == KEY_EVENT as u16 {
                    let key_event = &rec.Event.KeyEvent;
                    if key_event.bKeyDown != 0 {
                        // Consume the event
                        let mut temp: [INPUT_RECORD; 1] = std::mem::zeroed();
                        let mut num_read = 0;
                        let _ = windows_sys::Win32::System::Console::ReadConsoleInputW(stdin_handle, temp.as_mut_ptr(), 1, &mut num_read);
                        return true;
                    }
                }
            }
        }
        false
    }
}

#[cfg(target_os = "windows")]
fn run_preview_stub<S: Screensaver + 'static>(_saver: S) -> isize {
    eprintln!("Windows preview mode is not supported in console mode.");
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

#[cfg(not(target_os = "windows"))]
fn command_exists(cmd: &str) -> bool {
    std::process::Command::new("sh")
        .arg("-c")
        .arg(format!("command -v {} >/dev/null 2>&1", cmd))
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

// ---------------------------------------------------------------------------
// Common Fullscreen Animation Loop
// ---------------------------------------------------------------------------

fn run_fullscreen<S: Screensaver + 'static>(mut saver: S) -> isize {
    #[cfg(not(target_os = "windows"))]
    {
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
    }

    let _raw_mode = RawTerminalGuard::enable();
    let (mut cols, mut rows) = get_terminal_size();
    saver.init(cols, rows);

    let mut renderer = Renderer::new(cols, rows);
    let mut grid = vec![TerminalCell::default(); cols * rows];
    let mut last_frame = std::time::Instant::now();

    // Query actual monitor refresh rate
    let target_fps = get_monitor_refresh_rate();
    let frame_duration = Duration::from_secs_f32(1.0 / target_fps as f32);

    let mut initial_mouse_pos = None;
    let start_time = std::time::Instant::now();

    loop {
        if check_keypress() {
            break;
        }

        // Prevent instant exit on startup due to initial mouse shake or clicks
        if start_time.elapsed() > Duration::from_millis(500) {
            if check_mouse_activity(&mut initial_mouse_pos) {
                break;
            }
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
// Differential renderer (only redraws cells that changed)
// ---------------------------------------------------------------------------

struct Renderer {
    _width: usize,
    _height: usize,
    prev_grid: Vec<TerminalCell>,
}

impl Renderer {
    fn new(width: usize, height: usize) -> Self {
        use std::io::Write;
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

impl Drop for Renderer {
    fn drop(&mut self) {
        use std::io::Write;
        // Reset colors, show cursor, clear screen, and go home
        print!("\x1b[0m\x1b[?25h\x1b[2J\x1b[H");
        let _ = std::io::stdout().flush();
    }
}
