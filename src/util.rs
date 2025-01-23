//! Util.
//!
//! This module contains things that don't fit elsewhere.

pub(crate) fn set_window_title(title: &str) {
    print!("\x1b]0;{title}\x1b\\");
    let _ = std::io::Write::flush(&mut std::io::stdout());
}
