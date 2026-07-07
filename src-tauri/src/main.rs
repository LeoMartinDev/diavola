// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Self-re-exec watchdog mode (Unix only). See `src/watchdog.rs`. Intercept
    // before any Tauri initialization so the supervisor process is cheap and
    // silent. Windows does not use it — Job Objects already cover parent-death.
    #[cfg(unix)]
    {
        let mut args = std::env::args_os();
        if args.nth(1).is_some_and(|arg| arg == "--diavola-watchdog") {
            let rest: Vec<String> = args.map(|a| a.to_string_lossy().into_owned()).collect();
            let code = diavola_lib::watchdog::run(&rest);
            std::process::exit(code);
        }
    }

    diavola_lib::run()
}
