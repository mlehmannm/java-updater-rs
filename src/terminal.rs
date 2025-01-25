//! Terminal.
//!
//! This module contains terminal related things.

// https://learn.microsoft.com/en-us/windows/console/console-virtual-terminal-sequences
#[doc(hidden)]
pub(crate) fn set_window_title(title: &str) {
    print!("\x1b]0;{title}\x1b\\");
    let _ = std::io::Write::flush(&mut std::io::stdout());
}

// https://github.com/rust-lang/cargo/blob/cbd05082547daf4f10044bb2fc8a8eb8696a05d8/src/cargo/util/progress.rs#L163
#[doc(hidden)]
pub(crate) fn set_windows_progress(progress: Option<usize>) {
    let (state, progress) = if let Some(progress) = progress { (1, progress as f64) } else { (0, 0.0) };
    print!("\x1b]9;4;{state};{progress:.0}\x1b\\");
    let _ = std::io::Write::flush(&mut std::io::stdout());
}
