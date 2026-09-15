//! Windows platform glue: log-file location, file logging, the single-instance
//! mutex, and the native startup-error box. OS calls are `cfg(windows)` with
//! non-Windows fallbacks so the crate compiles and tests anywhere.

use std::path::{Path, PathBuf};

/// `<base>/ParamEx/app.log`. Pure; the caller resolves `%LOCALAPPDATA%`.
fn log_path_under(base: &Path) -> PathBuf {
    base.join("ParamEx").join("app.log")
}

/// `%LOCALAPPDATA%\ParamEx\app.log`, falling back to the home directory when
/// `LOCALAPPDATA` is unset, then to the working directory.
fn resolve_log_path() -> PathBuf {
    let base = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .or_else(dirs_home)
        .unwrap_or_else(|| PathBuf::from("."));
    log_path_under(&base)
}

/// Home directory from the environment (std only; avoids the `dirs` crate).
/// Windows: `USERPROFILE`; otherwise `HOME`.
fn dirs_home() -> Option<PathBuf> {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
}

/// Initialize `tracing` into the app-log file (append). Returns the log path
/// so the caller can name it in a startup-error box, or `None` when the file
/// could not be created (logging is then disabled). `Mutex<File>` is a
/// `MakeWriter`, so no `tracing-appender` dependency is needed.
pub fn init_logging() -> Option<PathBuf> {
    use std::fs::OpenOptions;
    use std::sync::Mutex;

    let path = resolve_log_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .ok()?;
    // try_init: never panic if something already set a subscriber.
    let _ = tracing_subscriber::fmt()
        .with_writer(Mutex::new(file))
        .with_ansi(false)
        .with_target(true)
        .try_init();
    Some(path)
}

/// True if another ParamEx instance already holds the single-instance mutex.
/// Non-Windows: always `false` (single-instance is a Windows-only affordance).
///
/// Call at most once, at startup: the first call acquires the named mutex and
/// holds it for the process lifetime.
pub fn already_running() -> bool {
    already_running_impl()
}

#[cfg(not(windows))]
fn already_running_impl() -> bool {
    false
}

#[cfg(windows)]
fn already_running_impl() -> bool {
    use windows::core::w;
    use windows::Win32::Foundation::{GetLastError, ERROR_ALREADY_EXISTS};
    use windows::Win32::System::Threading::CreateMutexW;

    // `Local\` scopes the mutex to the caller's Terminal-Server session, which
    // is the per-session single-instance intent. `Global\` needs
    // SeCreateGlobalPrivilege (the create silently fails for a standard
    // non-admin user, defeating the guard) and over-blocks a legitimate second
    // RDP session for admins.
    //
    // SAFETY: `CreateMutexW` takes no security attributes (None), a `false`
    // initial-owner flag, and a NUL-terminated wide-string literal that lives
    // for the whole program; there are no pointers to caller memory that could
    // dangle.
    match unsafe { CreateMutexW(None, false, w!("Local\\ParamEx_SingleInstance")) } {
        Ok(handle) => {
            // CreateMutexW returns Ok even when the mutex already exists; the
            // only "already running" signal is the thread's last-error value,
            // which any intervening call could overwrite.
            //
            // SAFETY: `GetLastError` has no preconditions; it reads the calling
            // thread's last-error slot, which this thread set on the line above.
            let already = unsafe { GetLastError() } == ERROR_ALREADY_EXISTS;
            // A named mutex object lives only while at least one handle is
            // open, so the handle is deliberately never closed. `HANDLE` is a
            // plain `Copy` wrapper without `Drop` (the bindings never close
            // handles on drop), so the `forget` is a no-op the compiler flags;
            // it stays to document the leak.
            #[allow(forgetting_copy_types)]
            std::mem::forget(handle);
            already
        }
        // A genuine OS failure: behave as the first instance rather than
        // refuse to start.
        Err(_) => false,
    }
}

/// Native blocking error box naming the log path, or stating that no log file
/// could be written. Non-Windows: stderr.
pub fn show_startup_error(log_path: Option<&Path>) {
    show_startup_error_impl(&startup_error_body(log_path));
}

fn startup_error_body(log_path: Option<&Path>) -> String {
    match log_path {
        Some(path) => format!(
            "ParamEx failed to start. See the log at:\n{}",
            path.display()
        ),
        None => "ParamEx failed to start. No log file could be written.".to_string(),
    }
}

#[cfg(not(windows))]
fn show_startup_error_impl(body: &str) {
    eprintln!("{body}");
}

#[cfg(windows)]
fn show_startup_error_impl(body: &str) {
    use windows::core::HSTRING;
    use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK};

    let title = HSTRING::from("ParamEx");
    let body = HSTRING::from(body);
    // MB_ICONERROR: the only native box the app shows is a fatal startup
    // failure, which warrants the error icon.
    //
    // SAFETY: `MessageBoxW` takes no owner window (None) and two `HSTRING`s
    // that outlive the blocking call; it reads them and returns.
    unsafe {
        let _ = MessageBoxW(None, &body, &title, MB_OK | MB_ICONERROR);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_path_is_appdata_paramex_app_log() {
        let base = PathBuf::from(r"C:\Users\x\AppData\Local");
        assert_eq!(
            log_path_under(&base),
            PathBuf::from(r"C:\Users\x\AppData\Local\ParamEx\app.log"),
        );
    }

    #[test]
    fn startup_error_names_the_log_or_says_none_was_written() {
        let path = PathBuf::from(r"C:\logs\app.log");
        assert!(startup_error_body(Some(&path)).ends_with(r"C:\logs\app.log"));
        assert_eq!(
            startup_error_body(None),
            "ParamEx failed to start. No log file could be written."
        );
    }
}
