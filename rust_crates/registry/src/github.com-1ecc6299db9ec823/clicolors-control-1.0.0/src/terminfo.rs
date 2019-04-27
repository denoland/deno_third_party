//! Auxiliary terminal information.
//!
//! These are internal functions exported to come to the same conclusions as
//! clicolors-control about terminal and color support if that is wanted.

use common;

/// Returns `true` if colors are supported by this terminal.
pub fn supports_colors() -> bool {
    common::is_a_color_terminal()
}

/// Returns `true` if a terminal is connected.
///
/// This uses a best effort check
pub fn is_atty() -> bool {
    common::is_a_terminal()
}
