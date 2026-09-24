use std::sync::atomic::{AtomicI32, Ordering};

use crate::error::CommandError;

static SIGNAL: AtomicI32 = AtomicI32::new(0);

pub fn install() -> Result<(), String> {
    install_platform()
}

#[cfg(unix)]
extern "C" fn record_signal(signal: std::ffi::c_int) {
    let _ = SIGNAL.compare_exchange(0, signal, Ordering::SeqCst, Ordering::SeqCst);
}

#[cfg(unix)]
fn install_platform() -> Result<(), String> {
    use nix::sys::signal::{SaFlags, SigAction, SigHandler, SigSet, Signal, sigaction};

    let action = SigAction::new(
        SigHandler::Handler(record_signal),
        SaFlags::SA_RESTART,
        SigSet::empty(),
    );
    // SAFETY: record_signal performs one lock-free atomic compare-exchange and has the required C ABI.
    unsafe {
        sigaction(Signal::SIGINT, &action).map_err(|error| error.to_string())?;
        sigaction(Signal::SIGTERM, &action).map_err(|error| error.to_string())?;
    }
    Ok(())
}

#[cfg(not(unix))]
fn install_platform() -> Result<(), String> {
    ctrlc::set_handler(|| {
        let _ = SIGNAL.compare_exchange(0, 2, Ordering::SeqCst, Ordering::SeqCst);
    })
    .map_err(|error| error.to_string())
}

pub fn check() -> crate::error::Result<()> {
    let signal = SIGNAL.load(Ordering::SeqCst);
    if signal == 0 {
        Ok(())
    } else {
        Err(CommandError::cancelled(signal))
    }
}
