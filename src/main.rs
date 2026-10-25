#[cfg(target_os = "macos")]
mod native;
#[cfg(any(target_os = "macos", test))]
mod worker;

#[cfg(target_os = "macos")]
use std::{
    process::Command,
    sync::{
        OnceLock,
        mpsc::{self, SyncSender},
    },
};

#[cfg(target_os = "macos")]
static EVENTS: OnceLock<SyncSender<()>> = OnceLock::new();

#[cfg(target_os = "macos")]
fn notify() {
    // One pending event is sufficient: commands should read the current appearance.
    if let Some(sender) = EVENTS.get() {
        let _ = sender.try_send(());
    }
}

#[cfg(target_os = "macos")]
fn main() {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args
        .first()
        .is_some_and(|arg| arg == "--help" || arg == "-h")
    {
        println!(
            "Usage: theme-listener COMMAND [ARG ...]\nRun on startup, macOS appearance changes, and wake. Retry failures with backoff."
        );
        return;
    }
    if args.is_empty() {
        eprintln!("Usage: theme-listener COMMAND [ARG ...]");
        std::process::exit(2);
    }
    let (sender, receiver) = mpsc::sync_channel(1);
    EVENTS.set(sender).unwrap();
    std::thread::spawn(move || {
        worker::run(
            receiver,
            || match Command::new(&args[0]).args(&args[1..]).status() {
                Ok(status) if status.success() => true,
                result => {
                    eprintln!("Theme command failed ({result:?}); scheduling retry.");
                    false
                }
            },
            std::time::Duration::from_secs(5),
            std::time::Duration::from_secs(300),
        );
    });
    native::run();
}

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("theme-listener requires macOS");
    std::process::exit(1);
}
