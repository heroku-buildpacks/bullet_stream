//! Helpers for formatting and colorizing your output.

use crate::ansi_escape::{self, ANSI};
pub(crate) const CMD_INDENT: &str = "      ";

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

/// Verb-ify command output i.e. "Running `bundle install`".
#[cfg(feature = "fun_run")]
pub(crate) fn running_command(contents: impl AsRef<str>) -> String {
    format!("Running {}", command(contents))
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
