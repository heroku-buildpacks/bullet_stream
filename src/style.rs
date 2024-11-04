//! Helpers for formatting and colorizing your output.

use crate::ansi_escape::{self, ANSI};

/// Decorate a URL for the build output.
pub fn url(contents: impl AsRef<str>) -> String {
    ansi_escape::wrap_ansi_escape_each_line(&ANSI::BoldUnderlineCyan, contents)
}

/// Decorate the name of a command being run i.e. `bundle install`.
pub fn command(contents: impl AsRef<str>) -> String {
    value(ansi_escape::wrap_ansi_escape_each_line(
        &ANSI::BoldCyan,
        contents,
    ))
}

/// Decorate an important value i.e. `2.3.4`.
pub fn value(contents: impl AsRef<str>) -> String {
    let contents = ansi_escape::wrap_ansi_escape_each_line(&ANSI::Yellow, contents);
    format!("`{contents}`")
}

/// Decorate additional information at the end of a line.
pub fn details(contents: impl AsRef<str>) -> String {
    let contents = contents.as_ref();
    format!("({contents})")
}

/// Decorate important information.
///
/// ```
/// use bullet_stream::style;
///
/// let help = style::important("HELP:");
/// format!("{help} review the logs");
/// ```
pub fn important(contents: impl AsRef<str>) -> String {
    ansi_escape::wrap_ansi_escape_each_line(&ANSI::BoldCyan, contents)
}

// Style macros defined here, but due to the way that #[macro_export] works they're defined
// on the top level module
mod macros {
    /// Colorize important text literals
    ///
    /// Wraps "important" color around a plain string literal. The main purpose is to be used in
    /// constants when used with a string literal:
    ///
    /// ```rust
    /// use bullet_stream::important_lit;
    ///
    /// const DEBUG_INFO: &str = important_lit!("Debug info:");
    /// # assert_eq!(DEBUG_INFO, "\u{1b}[1;36mDebug info:\u{1b}[0m");
    /// ```
    ///
    /// It does NOT include any other logic such as preserving other colors, or handling newlines.
    /// If you need newlines in your constant you should use the concat! macro:
    ///
    /// ```rust
    /// use bullet_stream::important_lit;
    ///
    /// const DEBUG_INFO: &str = concat!(
    ///     important_lit!("Debug info:"), "\n",
    ///     important_lit!("This will also be colorized"), "\n"
    /// );
    /// # assert_eq!("\u{1b}[1;36mDebug info:\u{1b}[0m\n\u{1b}[1;36mThis will also be colorized\u{1b}[0m\n", DEBUG_INFO);
    /// ```
    ///
    /// Note, if you try to use it like `format!` by accident, it will return the wrapped literal
    /// and not embed the replaced values:
    ///
    /// ```rust
    /// use bullet_stream::important_lit;
    ///
    /// let url = "https://example.com";
    /// let message = important_lit!("Url {url}");
    ///
    /// // Does NOT interpolate it like "Url https://example.com"
    /// // Instead it contains the literal string input including visible curly brackets:
    /// assert!(message.contains("Url {url}"));
    ///
    /// // If you need to use this with a format string instead use:
    /// let message = bullet_stream::style::important(format!("Url {url}"));
    /// # assert!(message.contains("Url https://example.com"));
    /// ```
    #[macro_export]
    macro_rules! important_lit {
        ($input:literal) => {
            concat!("\x1B[1;36m", $input, "\x1B[0m")
        };
    }
}
