//! Build script: embed the Windows `.exe` icon so `ParamEx.exe` shows the ParamEx
//! mark in Explorer/taskbar instead of the generic exe icon.
//!
//! Build-time only; no effect on runtime behavior or non-Windows hosts. A
//! release build must ship the icon, so a failed embed fails the build there.
//! Debug builds degrade to a cargo warning (the exe just lacks the icon) so a
//! missing resource compiler never blocks local development.

fn main() {
    // Re-run if the icon changes (path is relative to this crate's manifest dir).
    println!("cargo:rerun-if-changed=../../packaging/windows/paramex.ico");
    println!("cargo:rerun-if-env-changed=PROFILE");

    #[cfg(windows)]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("../../packaging/windows/paramex.ico");
        if let Err(err) = res.compile() {
            let release = std::env::var("PROFILE").is_ok_and(|profile| profile == "release");
            if release {
                panic!(
                    "release build cannot embed the exe icon (packaging/windows/paramex.ico): {err}"
                );
            }
            println!("cargo:warning=failed to embed exe icon (paramex.ico): {err}");
        }
    }
}
