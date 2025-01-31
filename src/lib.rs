#![doc = include_str!("../README.md")]

use crate::util::ParagraphInspectWrite;
use crate::write::line_mapped;
use std::fmt::Debug;
use std::io::Write;
use std::time::Instant;

mod ansi_escape;
mod background_printer;
mod duration_format;
pub mod global;
pub mod style;
mod util;
mod write;
pub use ansi_escape::strip_ansi;
use global::_GlobalWriter;
use style::CMD_INDENT;
use util::TrailingParagraph;

/// Use [`Print`] to output structured text as a buildpack/script executes. The output
/// is intended to be read by the application user.
///
/// ```rust
/// use bullet_stream::Print;
///
/// let mut output = Print::new(std::io::stdout())
///     .h2("Example Buildpack")
///     .warning("No Gemfile.lock found");
///
/// output = output
///     .bullet("Ruby version")
///     .done();
///
/// output.done();
/// ```
#[allow(clippy::module_name_repetitions)]
#[derive(Debug)]
pub struct Print<T> {
    pub(crate) started: Option<Instant>,
    pub(crate) state: T,
}

#[deprecated(
    since = "0.2.0",
    note = "bullet_stream::Output conflicts with std::io::Output, prefer Print"
)]
pub type Output<T> = Print<T>;

/// Various states for [`Print`] to contain.
///
/// The [`Print`] struct acts as an output state machine. These structs
/// represent the various states. See struct documentation for more details.
pub mod state {
    use crate::background_printer::PrintGuard;
    use crate::util::ParagraphInspectWrite;
    use crate::write::MappedWrite;
    use std::time::Instant;

    /// At the start of a stream you can output a header (h1) or subheader (h2).
    ///
    /// In this state, represented by `state::Header` the user hasn't seen any output yet.
    /// You can have multiple subheaders (h2) but only one header (h1), so as soon as
    /// h1 is called you the state will be transitioned to `state::Bullet`.
    ///
    /// If using for a buildpack output, consider that each buildpack is run via a top level
    /// context which could be considered H1. Therefore each buildpack should announce it's name
    /// via the `h2` function.
    ///
    /// Example:
    ///
    /// ```rust
    /// use bullet_stream::{Print, state::{Bullet, Header}};
    /// use std::io::Write;
    ///
    /// let mut not_started = Print::new(std::io::stdout());
    /// let output = start_buildpack(not_started);
    ///
    /// output.bullet("Ruby version").sub_bullet("Installing Ruby").done();
    ///
    /// fn start_buildpack<W>(mut output: Print<Header<W>>) -> Print<Bullet<W>>
    /// where W: Write + Send + Sync + 'static {
    ///     output.h2("Example Buildpack")
    ///}
    /// ```
    #[derive(Debug)]
    pub struct Header<W> {
        pub(crate) write: ParagraphInspectWrite<W>,
    }

    /// After the buildpack output has started, its top-level output will be represented by the
    /// `state::Bullet` type and is transitioned into a `state::SubBullet` to provide additional
    /// details.
    ///
    /// Example:
    ///
    /// ```rust
    /// use bullet_stream::{
    ///     state::{Bullet, Header, SubBullet},
    ///     Print,
    /// };
    /// use std::io::Write;
    /// use std::path::{Path, PathBuf};
    ///
    /// let mut output = Print::new(std::io::stdout()).h2("Example Buildpack");
    ///
    /// output = install_ruby(&PathBuf::from("/dev/null"), output)
    ///     .unwrap()
    ///     .done();
    ///
    /// fn install_ruby<W>(
    ///     path: &Path,
    ///     mut output: Print<Bullet<W>>,
    /// ) -> Result<Print<SubBullet<W>>, std::io::Error>
    /// where
    ///     W: Write + Send + Sync + 'static,
    /// {
    ///     let out = output.bullet("Ruby version").sub_bullet("Installing Ruby");
    ///     // ...
    ///     Ok(out)
    /// }
    /// ```
    #[derive(Debug)]
    pub struct Bullet<W> {
        pub(crate) write: ParagraphInspectWrite<W>,
    }

    /// The `state::SubBullet` is intended to provide additional details about the buildpack's
    /// actions. When a section is finished, it transitions back to a `state::Bullet` type.
    ///
    /// A streaming type can be started from a `state::Bullet`, usually to run and stream a
    /// `process::Command` to the end user.
    ///
    /// Example:
    ///
    /// ```rust
    /// use bullet_stream::{Print, state::{Bullet, SubBullet}};
    /// use std::io::Write;
    ///
    /// let mut output = Print::new(std::io::stdout())
    ///     .h2("Example Buildpack")
    ///     .bullet("Ruby version");
    ///
    /// install_ruby(output).done();
    ///
    /// fn install_ruby<W>(mut output: Print<SubBullet<W>>) -> Print<Bullet<W>>
    /// where W: Write + Send + Sync + 'static {
    ///     let output = output.sub_bullet("Installing Ruby");
    ///     // ...
    ///
    ///     output.done()
    ///}
    /// ```
    #[derive(Debug)]
    pub struct SubBullet<W> {
        pub(crate) write: ParagraphInspectWrite<W>,
    }

    /// This state is intended for streaming output from a process to the end user. It is
    /// started from a `state::SubBullet` and finished back to a `state::SubBullet`.
    ///
    /// The `Print<state::Stream<W>>` implements [`std::io::Write`], so you can stream
    /// from anything that accepts a [`std::io::Write`].
    ///
    /// ```rust
    /// use bullet_stream::{Print, state::{Bullet, SubBullet}};
    /// use std::io::Write;
    ///
    /// let mut output = Print::new(std::io::stdout())
    ///     .h2("Example Buildpack")
    ///     .bullet("Ruby version");
    ///
    /// install_ruby(output).done();
    ///
    /// fn install_ruby<W>(mut output: Print<SubBullet<W>>) -> Print<SubBullet<W>>
    /// where W: Write + Send + Sync + 'static {
    ///     let mut stream = output.sub_bullet("Installing Ruby")
    ///         .start_stream("Streaming stuff");
    ///
    ///     write!(&mut stream, "...").unwrap();
    ///
    ///     stream.done()
    ///}
    /// ```
    #[derive(Debug)]
    pub struct Stream<W: std::io::Write> {
        pub(crate) started: Instant,
        pub(crate) write: MappedWrite<ParagraphInspectWrite<W>>,
    }

    /// This state is intended for long-running tasks that do not stream but wish to convey progress
    /// to the end user. For example, while downloading a file.
    ///
    /// This state is started from a [`SubBullet`] and finished back to a [`SubBullet`].
    ///
    /// ```rust
    /// use bullet_stream::{Print, state::{Bullet, SubBullet}};
    /// use std::io::Write;
    ///
    /// let mut output = Print::new(std::io::stdout())
    ///     .h2("Example Buildpack")
    ///     .bullet("Ruby version");
    ///
    /// install_ruby(output).done();
    ///
    /// fn install_ruby<W>(mut output: Print<SubBullet<W>>) -> Print<SubBullet<W>>
    /// where W: Write + Send + Sync + 'static {
    ///     let mut timer = output.sub_bullet("Installing Ruby")
    ///         .start_timer("Installing");
    ///
    ///     /// ...
    ///
    ///     timer.done()
    ///}
    /// ```
    #[derive(Debug)]
    pub struct Background<W: std::io::Write + Send + 'static> {
        pub(crate) started: Instant,
        pub(crate) write: PrintGuard<ParagraphInspectWrite<W>>,
    }
}

/// Used for announcements such as warning and error states
trait AnnounceSupportedState {
    type Inner: Write;

    fn write_mut(&mut self) -> &mut ParagraphInspectWrite<Self::Inner>;
}

/// Used for announcements such as warning and error states
impl<W> AnnounceSupportedState for state::SubBullet<W>
where
    W: Write,
{
    type Inner = W;

    fn write_mut(&mut self) -> &mut ParagraphInspectWrite<Self::Inner> {
        &mut self.write
    }
}

/// Used for announcements such as warning and error states
impl<W> AnnounceSupportedState for state::Bullet<W>
where
    W: Write,
{
    type Inner = W;

    fn write_mut(&mut self) -> &mut ParagraphInspectWrite<Self::Inner> {
        &mut self.write
    }
}

/// Used for announcements such as warning and error states
#[allow(private_bounds)]
impl<S> Print<S>
where
    S: AnnounceSupportedState,
{
    /// Emit an error and end the build output.
    ///
    /// When an unrecoverable situation is encountered, you can emit an error message to the user.
    /// This associated function will consume the build output, so you may only emit one error per
    /// build output.
    ///
    /// An error message should describe what went wrong and why the buildpack cannot continue.
    /// It is best practice to include debugging information in the error message. For example,
    /// if a file is missing, consider showing the user the contents of the directory where the
    /// file was expected to be and the full path of the file.
    ///
    /// If you are confident about what action needs to be taken to fix the error, you should include
    /// that in the error message. Do not write a generic suggestion like "try again later" unless
    /// you are certain that the error is transient.
    ///
    /// If you detect something problematic but not bad enough to halt buildpack execution, consider
    /// using a [`Print::warning`] instead.
    ///
    pub fn error(mut self, s: impl AsRef<str>) {
        write::error(self.state.write_mut(), s);
    }

    /// Emit a warning message to the end user.
    ///
    /// A warning should be used to emit a message to the end user about a potential problem.
    ///
    /// Multiple warnings can be emitted in sequence. The buildpack author should take care not to
    /// overwhelm the end user with unnecessary warnings.
    ///
    /// When emitting a warning, describe the problem to the user, if possible, and tell them how
    /// to fix it or where to look next.
    ///
    /// Warnings should often come with some disabling mechanism, if possible. If the user can turn
    /// off the warning, that information should be included in the warning message. If you're
    /// confident that the user should not be able to turn off a warning, consider using a
    /// [`Print::error`] instead.
    ///
    /// Warnings will be output in a multi-line paragraph style. A warning can be emitted from any
    /// state except for [`state::Header`].
    #[must_use]
    pub fn warning(mut self, s: impl AsRef<str>) -> Print<S> {
        write::warning(self.state.write_mut(), s);
        self
    }

    /// Emit an important message to the end user.
    ///
    /// When something significant happens but is not inherently negative, you can use an important
    /// message. For example, if a buildpack detects that the operating system or architecture has
    /// changed since the last build, it might not be a problem, but if something goes wrong, the
    /// user should know about it.
    ///
    /// Important messages should be used sparingly and only for things the user should be aware of
    /// but not necessarily act on. If the message is actionable, consider using a
    /// [`Print::warning`] instead.
    #[must_use]
    pub fn important(mut self, s: impl AsRef<str>) -> Print<S> {
        write::important(self.state.write_mut(), s);
        self
    }
}

impl Print<state::Header<_GlobalWriter>> {
    /// Create an output struct that uses the configured global writer
    ///
    /// To modify the global writer call [global::set_writer]
    pub fn global() -> Print<state::Header<_GlobalWriter>> {
        Print {
            state: state::Header {
                write: ParagraphInspectWrite {
                    inner: _GlobalWriter,
                    was_paragraph: _GlobalWriter.trailing_paragraph(),
                    newlines_since_last_char: _GlobalWriter.trailing_newline_count(),
                },
            },
            started: None,
        }
    }
}

impl<W> Print<state::Header<W>>
where
    W: Write + Send + Sync + 'static,
{
    /// Create a buildpack output struct, but do not announce the buildpack's start.
    ///
    /// See the [`Print::h1`] and [`Print::h2`] methods for more details.
    #[must_use]
    pub fn new(io: W) -> Self {
        Self {
            state: state::Header {
                write: ParagraphInspectWrite::new(io),
            },
            started: None,
        }
    }

    /// Announce the start of the buildpack.
    ///
    /// The input should be the human-readable name of your buildpack. Most buildpack names include
    /// the feature they provide.
    ///
    /// It is common to use a title case for the buildpack name and to include the word "Buildpack" at the end.
    /// For example, `Ruby Buildpack`. Do not include a period at the end of the name.
    ///
    /// Avoid starting your buildpack with "Heroku" unless you work for Heroku. If you wish to express that your
    /// buildpack is built to target only Heroku; you can include that in the description of the buildpack.
    ///
    /// This function will transition your buildpack output to [`state::Bullet`].
    #[must_use]
    pub fn h1(mut self, s: impl AsRef<str>) -> Print<state::Bullet<W>> {
        write::h1(&mut self.state.write, s);

        self.without_header()
    }

    /// Announce the start of the buildpack.
    ///
    /// The input should be the human-readable name of your buildpack. Most buildpack names include
    /// the feature they provide.
    ///
    /// It is common to use a title case for the buildpack name and to include the word "Buildpack" at the end.
    /// For example, `Ruby Buildpack`. Do not include a period at the end of the name.
    ///
    /// Avoid starting your buildpack with "Heroku" unless you work for Heroku. If you wish to express that your
    /// buildpack is built to target only Heroku; you can include that in the description of the buildpack.
    ///
    /// This function will transition your buildpack output to [`state::Bullet`].
    #[must_use]
    pub fn h2(mut self, s: impl AsRef<str>) -> Print<state::Bullet<W>> {
        write::h2(&mut self.state.write, s);

        self.without_header()
    }

    /// Start a buildpack output without announcing the name.
    #[must_use]
    pub fn without_header(self) -> Print<state::Bullet<W>> {
        Print {
            started: Some(Instant::now()),
            state: state::Bullet {
                write: self.state.write,
            },
        }
    }
}

impl<W> Print<state::Bullet<W>>
where
    W: Write + Send + Sync + 'static,
{
    /// A top-level bullet point section
    ///
    /// A section should be a noun, e.g., 'Ruby version'. Anything emitted within the section
    /// should be in the context of this output.
    ///
    /// If the following steps can change based on input, consider grouping shared information
    /// such as version numbers and sources in the section name e.g.,
    /// 'Ruby version ``3.1.3`` from ``Gemfile.lock``'.
    ///
    /// This function will transition your buildpack output to [`state::SubBullet`].
    #[must_use]
    pub fn bullet(mut self, s: impl AsRef<str>) -> Print<state::SubBullet<W>> {
        write::bullet(&mut self.state.write, s);

        Print {
            started: self.started,
            state: state::SubBullet {
                write: self.state.write,
            },
        }
    }

    /// Outputs an H2 header
    #[must_use]
    pub fn h2(mut self, s: impl AsRef<str>) -> Print<state::Bullet<W>> {
        write::h2(&mut self.state.write, s);
        self
    }

    /// Announce that your buildpack has finished execution successfully.
    pub fn done(mut self) -> W {
        write::all_done(&mut self.state.write, &self.started);

        self.state.write.inner
    }
}

impl<W> Print<state::Background<W>>
where
    W: Write + Send + Sync + 'static,
{
    /// Interrupt a timer with a message explaining why
    ///
    /// ```rust
    /// use bullet_stream::Print;
    ///
    /// let mut output = Print::new(Vec::new())
    ///     .h2("Example Buildpack");
    ///
    /// let mut bullet = output.bullet("Example timer cancel");
    /// let mut timer = bullet.start_timer("Installing Ruby");
    /// std::thread::sleep(std::time::Duration::from_millis(1));
    ///
    /// bullet = timer.cancel("Interrupted");
    /// timer = bullet.start_timer("Retrying");
    /// std::thread::sleep(std::time::Duration::from_millis(1));
    /// bullet = timer.done();
    /// output = bullet.done();
    ///
    /// use indoc::formatdoc;
    /// use bullet_stream::strip_ansi;
    /// assert_eq!(
    ///     formatdoc!
    ///         {"## Example Buildpack
    ///
    ///           - Example timer cancel
    ///             - Installing Ruby ... (Interrupted)
    ///             - Retrying ... (< 0.1s)
    ///           - Done (finished in < 0.1s)
    ///         "}.trim(),
    ///     strip_ansi(String::from_utf8_lossy(&output.done())).trim()
    /// );
    /// ```
    pub fn cancel(self, why_details: impl AsRef<str>) -> Print<state::SubBullet<W>> {
        let mut io = match self.state.write.stop() {
            Ok(io) => io,
            // Stdlib docs recommend using `resume_unwind` to resume the thread panic
            // <https://doc.rust-lang.org/std/thread/type.Result.html>
            Err(e) => std::panic::resume_unwind(e),
        };

        writeln_now(&mut io, style::details(why_details));
        Print {
            started: self.started,
            state: state::SubBullet { write: io },
        }
    }

    /// Finalize a timer's output.
    ///
    /// Once you're finished with your long running task, calling this function
    /// finalizes the timer's output and transitions back to a [`state::SubBullet`].
    #[must_use]
    pub fn done(self) -> Print<state::SubBullet<W>> {
        let duration = self.state.started.elapsed();
        let mut io = match self.state.write.stop() {
            Ok(io) => io,
            // Stdlib docs recommend using `resume_unwind` to resume the thread panic
            // <https://doc.rust-lang.org/std/thread/type.Result.html>
            Err(e) => std::panic::resume_unwind(e),
        };

        writeln_now(&mut io, style::details(duration_format::human(&duration)));
        Print {
            started: self.started,
            state: state::SubBullet { write: io },
        }
    }
}

impl<W> Print<state::SubBullet<W>>
where
    W: Write + Send + Sync + 'static,
{
    /// Emit a sub bullet point step in the output under a bullet point.
    ///
    /// A step should be a verb, i.e., 'Downloading'. Related verbs should be nested under a single section.
    ///
    /// Some example verbs to use:
    ///
    /// - Downloading
    /// - Writing
    /// - Using
    /// - Reading
    /// - Clearing
    /// - Skipping
    /// - Detecting
    /// - Compiling
    /// - etc.
    ///
    /// Steps should be short and stand-alone sentences within the context of the section header.
    ///
    /// In general, if the buildpack did something different between two builds, it should be
    /// observable by the user through the buildpack output. For example, if a cache needs to be
    /// cleared, emit that your buildpack is clearing it and why.
    ///
    /// Multiple steps are allowed within a section. This function returns to the same [`state::SubBullet`].
    #[must_use]
    pub fn sub_bullet(mut self, s: impl AsRef<str>) -> Print<state::SubBullet<W>> {
        write::sub_bullet(&mut self.state.write, s);
        self
    }

    /// Stream output to the end user.
    ///
    /// The most common use case is to stream the output of a running `std::process::Command` to the
    /// end user. Streaming lets the end user know that something is happening and provides them with
    /// the output of the process.
    ///
    /// The result of this function is a `Print<state::Stream<W>>` which implements [`std::io::Write`].
    ///
    /// If you do not wish the end user to view the output of the process, consider using a `step` instead.
    ///
    /// This function will transition your buildpack output to [`state::Stream`].
    #[must_use]
    pub fn start_stream(mut self, s: impl AsRef<str>) -> Print<state::Stream<W>> {
        write::sub_bullet(&mut self.state.write, s);
        writeln_now(&mut self.state.write, "");

        Print {
            started: self.started,
            state: state::Stream {
                started: Instant::now(),
                write: line_mapped(self.state.write, |mut line| {
                    // Avoid adding trailing whitespace to the line, if there was none already.
                    // The `[b'\n']` case is required since `line` includes the trailing newline byte.
                    if line.is_empty() || line == [b'\n'] {
                        line
                    } else {
                        let mut result: Vec<u8> = CMD_INDENT.into();
                        result.append(&mut line);
                        result
                    }
                }),
            },
        }
    }

    /// Output periodic timer updates to the end user.
    ///
    /// If a buildpack author wishes to start a long-running task that does not stream, starting a timer
    /// will let the user know that the buildpack is performing work and that the UI is not stuck.
    ///
    /// One common use case is when downloading a file. Emitting periodic output when downloading is especially important for the local
    /// buildpack development experience where the user's network may be unexpectedly slow, such as
    /// in a hotel or on a plane.
    ///
    /// This function will transition your buildpack output to [`state::Background`].
    #[allow(clippy::missing_panics_doc)]
    #[must_use]
    #[allow(unused_mut)]
    pub fn start_timer(mut self, s: impl AsRef<str>) -> Print<state::Background<W>> {
        write::sub_start_timer(self.state.write, Instant::now(), s)
    }

    /// Print command name and run it quietly (don't stream) while emitting timing dots
    ///
    /// Provides convience and standardization. If you want to stream the output
    /// see [Self::stream_cmd].
    ///
    /// ```no_run
    /// use bullet_stream::{style, Print};
    /// use fun_run::CommandWithName;
    /// use std::process::Command;
    ///
    /// let mut output = Print::new(std::io::stdout())
    ///     .h2("Example Buildpack")
    ///     .bullet("Streaming");
    ///
    /// // Use the result of the timed command
    /// let result = output.time_cmd(
    ///     Command::new("echo")
    ///         .arg("hello world")
    /// );
    ///
    /// output.done().done();
    /// ```
    #[cfg(feature = "fun_run")]
    #[allow(unused_mut)]
    pub fn time_cmd(
        &mut self,
        mut command: impl fun_run::CommandWithName,
    ) -> Result<fun_run::NamedOutput, fun_run::CmdError> {
        util::mpsc_stream_to_output(
            |sender| {
                let start = Instant::now();
                let background =
                    write::sub_start_print_interval(sender, style::running_command(command.name()));
                let output = command.named_output();
                writeln_now(
                    &mut background.stop().expect("constructed with valid state"),
                    style::details(duration_format::human(&start.elapsed())),
                );
                output
            },
            move |recv| {
                for message in recv {
                    self.state.write.write_all(&message).expect("Writeable");
                }
            },
        )
    }

    /// Stream two inputs without consuming
    ///
    /// The `start_stream` returns a single writer, but running a command often requires two.
    /// This function allows you to stream both stdout and stderr to the end user using a single writer.
    ///
    /// It takes a step string that will be advertized and a closure that takes two writers and returns a value.
    /// The return value is returned from the function.
    ///
    /// Example:
    ///
    /// ```no_run
    /// use bullet_stream::{style, Print};
    /// use fun_run::CommandWithName;
    /// use std::process::Command;
    ///
    /// let mut output = Print::new(std::io::stdout())
    ///     .h2("Example Buildpack")
    ///     .bullet("Streaming");
    ///
    /// let mut cmd = Command::new("echo");
    /// cmd.arg("hello world");
    ///
    /// // Use the result of the Streamed command
    /// let result = output.stream_with(
    ///     format!("Running {}", style::command(cmd.name())),
    ///     |stdout, stderr| cmd.stream_output(stdout, stderr),
    /// );
    ///
    /// output.done().done();
    /// ```
    #[allow(clippy::missing_panics_doc)]
    pub fn stream_with<F, T>(&mut self, s: impl AsRef<str>, f: F) -> T
    where
        F: FnMut(Box<dyn Write + Send + Sync>, Box<dyn Write + Send + Sync>) -> T,
        T: 'static,
    {
        write::sub_stream_with(&mut self.state.write, s, f)
    }

    /// Announce and run a command while streaming its output
    ///
    /// Provides convience and standardization. To run without streaming see [Self::time_cmd].
    ///
    /// Example:
    ///
    /// ```no_run
    /// use bullet_stream::{style, Print};
    /// use fun_run::CommandWithName;
    /// use std::process::Command;
    ///
    /// let mut output = Print::new(std::io::stdout())
    ///     .h2("Example Buildpack")
    ///     .bullet("Streaming");
    ///
    /// // Use the result of the Streamed command
    /// let result = output.stream_cmd(
    ///     Command::new("echo")
    ///         .arg("hello world")
    /// );
    ///
    /// output.done().done();
    /// ```
    #[cfg(feature = "fun_run")]
    pub fn stream_cmd(
        &mut self,
        command: impl fun_run::CommandWithName,
    ) -> Result<fun_run::NamedOutput, fun_run::CmdError> {
        write::sub_stream_cmd(&mut self.state.write, command)
    }

    /// Finish a section and transition back to [`state::Bullet`].
    #[must_use]
    pub fn done(self) -> Print<state::Bullet<W>> {
        Print {
            started: self.started,
            state: state::Bullet {
                write: self.state.write,
            },
        }
    }
}

impl<W> Print<state::Stream<W>>
where
    W: Write + Send + Sync + 'static,
{
    /// Finalize a stream's output
    ///
    /// Once you're finished streaming to the output, calling this function
    /// finalizes the stream's output and transitions back to a [`state::Bullet`].
    #[must_use]
    pub fn done(self) -> Print<state::SubBullet<W>> {
        let duration = self.state.started.elapsed();

        let mut output = Print {
            started: self.started,
            state: state::SubBullet {
                write: self.state.write.unwrap(),
            },
        };

        if !output.state.write_mut().was_paragraph {
            writeln_now(&mut output.state.write, "");
        }

        output.sub_bullet(format!(
            "Done {}",
            style::details(duration_format::human(&duration))
        ))
    }
}

impl<W> Write for Print<state::Stream<W>>
where
    W: Write,
{
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.state.write.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.state.write.flush()
    }
}

/// Internal helper, ensures that all contents are always flushed (never buffered).
fn writeln_now<D: Write>(destination: &mut D, msg: impl AsRef<str>) {
    writeln!(destination, "{}", msg.as_ref()).expect("Output error: UI writer closed");

    destination.flush().expect("Output error: UI writer closed");
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::util::LockedWriter;
    use ansi_escape::strip_ansi;
    use fun_run::CommandWithName;
    use indoc::formatdoc;
    use libcnb_test::assert_contains;
    use pretty_assertions::assert_eq;
    use std::{cell::RefCell, fs::File, process::Command};

    #[test]
    fn double_h2_h2_newlines() {
        let writer = Vec::new();
        let output = Print::new(writer).h2("Header 2").h2("Header 2");

        let io = output.done();
        let expected = formatdoc! {"

            ## Header 2

            ## Header 2

            - Done (finished in < 0.1s)
        "};

        assert_eq!(expected, strip_ansi(String::from_utf8_lossy(&io)))
    }

    #[test]
    fn double_h1_h2_newlines() {
        let writer = Vec::new();
        let output = Print::new(writer).h1("Header 1").h2("Header 2");

        let io = output.done();
        let expected = formatdoc! {"

            # Header 1

            ## Header 2

            - Done (finished in < 0.1s)
        "};

        assert_eq!(expected, strip_ansi(String::from_utf8_lossy(&io)))
    }

    #[test]
    fn stream_with() {
        let writer = Vec::new();
        let mut output = Print::new(writer)
            .h2("Example Buildpack")
            .bullet("Streaming");
        let mut cmd = std::process::Command::new("echo");
        cmd.arg("hello world");

        let _result = output.stream_with(
            format!("Running {}", style::command(cmd.name())),
            |stdout, stderr| cmd.stream_output(stdout, stderr),
        );

        let io = output.done().done();
        let expected = formatdoc! {"

            ## Example Buildpack

            - Streaming
              - Running `echo \"hello world\"`

                  hello world

              - Done (< 0.1s)
            - Done (finished in < 0.1s)
        "};

        assert_eq!(expected, strip_ansi(String::from_utf8_lossy(&io)));
    }

    #[test]
    fn background_timer() {
        let io = Print::new(Vec::new())
            .without_header()
            .bullet("Background")
            .start_timer("Installing")
            .done()
            .done()
            .done();

        // Test human readable timer output
        let expected = formatdoc! {"
            - Background
              - Installing ... (< 0.1s)
            - Done (finished in < 0.1s)
        "};

        assert_eq!(expected, strip_ansi(String::from_utf8_lossy(&io)));

        // Test timer dot colorization
        let expected = formatdoc! {"
            - Background
              - Installing\u{1b}[2;1m .\u{1b}[0m\u{1b}[2;1m.\u{1b}[0m\u{1b}[2;1m. \u{1b}[0m(< 0.1s)
            - Done (finished in < 0.1s)
        "};

        assert_eq!(expected, String::from_utf8_lossy(&io));
    }

    #[test]
    fn background_timer_dropped() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("output.txt");
        let timer = Print::new(File::create(&path).unwrap())
            .without_header()
            .bullet("Background")
            .start_timer("Installing");
        drop(timer);

        // Test human readable timer output
        let expected = formatdoc! {"
            - Background
              - Installing ... (Error)
        "};

        assert_eq!(expected, strip_ansi(std::fs::read_to_string(path).unwrap()));
    }

    #[test]
    fn write_paragraph_empty_lines() {
        let io = Print::new(Vec::new())
            .h1("Example Buildpack\n\n")
            .warning("\n\nhello\n\n\t\t\nworld\n\n")
            .bullet("Version\n\n")
            .sub_bullet("Installing\n\n")
            .done()
            .done();

        let tab_char = '\t';
        let expected = formatdoc! {"

            # Example Buildpack

            ! hello
            !
            ! {tab_char}{tab_char}
            ! world

            - Version
              - Installing
            - Done (finished in < 0.1s)
        "};

        assert_eq!(expected, strip_ansi(String::from_utf8_lossy(&io)));
    }

    #[test]
    fn paragraph_color_codes() {
        let tmpdir = tempfile::tempdir().unwrap();
        let path = tmpdir.path().join("output.txt");

        Print::new(File::create(&path).unwrap())
            .h1("Buildpack Header is Bold Purple")
            .important("Important is bold cyan")
            .warning("Warnings are yellow")
            .error("Errors are red");

        let expected = formatdoc! {"

            \u{1b}[1;35m# Buildpack Header is Bold Purple\u{1b}[0m

            \u{1b}[1;36m! Important is bold cyan\u{1b}[0m

            \u{1b}[0;33m! Warnings are yellow\u{1b}[0m

            \u{1b}[0;31m! Errors are red\u{1b}[0m

        "};

        assert_eq!(expected, std::fs::read_to_string(path).unwrap());
    }

    #[test]
    fn test_important() {
        let writer = Vec::new();
        let io = Print::new(writer)
            .h1("Heroku Ruby Buildpack")
            .important("This is important")
            .done();

        let expected = formatdoc! {"

            # Heroku Ruby Buildpack

            ! This is important

            - Done (finished in < 0.1s)
        "};

        assert_eq!(expected, strip_ansi(String::from_utf8_lossy(&io)));
    }

    #[test]
    fn test_error() {
        let tmpdir = tempfile::tempdir().unwrap();
        let path = tmpdir.path().join("output.txt");

        Print::new(File::create(&path).unwrap())
            .h1("Heroku Ruby Buildpack")
            .error("This is an error");

        let expected = formatdoc! {"

            # Heroku Ruby Buildpack

            ! This is an error

        "};

        assert_eq!(expected, strip_ansi(std::fs::read_to_string(path).unwrap()));
    }

    #[test]
    fn test_captures() {
        let writer = Vec::new();
        let mut first_stream = Print::new(writer)
            .h1("Heroku Ruby Buildpack")
            .bullet("Ruby version `3.1.3` from `Gemfile.lock`")
            .done()
            .bullet("Hello world")
            .start_stream("Streaming with no newlines");

        writeln!(&mut first_stream, "stuff").unwrap();

        let mut second_stream = first_stream
            .done()
            .start_stream("Streaming with blank lines and a trailing newline");

        writeln!(&mut second_stream, "foo\nbar\n\n\t\nbaz\n").unwrap();

        let io = second_stream.done().done().done();

        let tab_char = '\t';
        let expected = formatdoc! {"

            # Heroku Ruby Buildpack

            - Ruby version `3.1.3` from `Gemfile.lock`
            - Hello world
              - Streaming with no newlines

                  stuff

              - Done (< 0.1s)
              - Streaming with blank lines and a trailing newline

                  foo
                  bar

                  {tab_char}
                  baz

              - Done (< 0.1s)
            - Done (finished in < 0.1s)
        "};

        assert_eq!(expected, strip_ansi(String::from_utf8_lossy(&io)));
    }

    #[test]
    fn test_streaming_a_command() {
        let writer = Vec::new();
        let mut stream = Print::new(writer)
            .h1("Streaming buildpack demo")
            .bullet("Command streaming")
            .start_stream("Streaming stuff");

        let locked_writer = LockedWriter::new(stream);

        std::process::Command::new("echo")
            .arg("hello world")
            .stream_output(locked_writer.clone(), locked_writer.clone())
            .unwrap();

        stream = locked_writer.unwrap();

        let io = stream.done().done().done();

        let actual = strip_ansi(String::from_utf8_lossy(&io));

        assert_contains!(actual, "      hello world\n");
    }

    #[test]
    fn warning_after_buildpack() {
        let writer = Vec::new();
        let io = Print::new(writer)
            .h1("RCT")
            .warning("It's too crowded here\nI'm tired")
            .bullet("Guest thoughts")
            .sub_bullet("The jumping fountains are great")
            .sub_bullet("The music is nice here")
            .done()
            .done();

        let expected = formatdoc! {"

            # RCT

            ! It's too crowded here
            ! I'm tired

            - Guest thoughts
              - The jumping fountains are great
              - The music is nice here
            - Done (finished in < 0.1s)
        "};

        assert_eq!(expected, strip_ansi(String::from_utf8_lossy(&io)));
    }

    #[test]
    fn warning_step_padding() {
        let writer = Vec::new();
        let io = Print::new(writer)
            .h1("RCT")
            .bullet("Guest thoughts")
            .sub_bullet("The scenery here is wonderful")
            .warning("It's too crowded here\nI'm tired")
            .sub_bullet("The jumping fountains are great")
            .sub_bullet("The music is nice here")
            .done()
            .done();

        let expected = formatdoc! {"

            # RCT

            - Guest thoughts
              - The scenery here is wonderful

            ! It's too crowded here
            ! I'm tired

              - The jumping fountains are great
              - The music is nice here
            - Done (finished in < 0.1s)
        "};

        assert_eq!(expected, strip_ansi(String::from_utf8_lossy(&io)));
    }

    thread_local! {
        static THREAD_LOCAL_WRITER: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
    }

    struct V8ThreadedWriter;
    impl V8ThreadedWriter {
        fn take() -> Vec<u8> {
            THREAD_LOCAL_WRITER.take()
        }
    }
    impl Write for V8ThreadedWriter {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            THREAD_LOCAL_WRITER.with_borrow_mut(|writer| writer.write(buf))
        }

        fn flush(&mut self) -> std::io::Result<()> {
            THREAD_LOCAL_WRITER.with_borrow_mut(|writer| writer.flush())
        }
    }

    #[test]
    fn global_preserves_newline() {
        global::set_writer(V8ThreadedWriter);

        Print::global()
            .h1("Genuine Joes")
            .bullet("Dodge")
            .sub_bullet("A ball")
            .error("A wrench");

        Print::global()
            .without_header()
            .error("It's a bold strategy, Cotton.\nLet's see if it pays off for 'em.");

        let io = V8ThreadedWriter::take();
        let expected = formatdoc! {"

            # Genuine Joes

            - Dodge
              - A ball

            ! A wrench

            ! It's a bold strategy, Cotton.
            ! Let's see if it pays off for 'em.

        "};

        assert_eq!(expected, strip_ansi(String::from_utf8_lossy(&io)));
    }

    #[test]
    fn double_warning_step_padding() {
        let writer = Vec::new();
        let output = Print::new(writer)
            .h1("RCT")
            .bullet("Guest thoughts")
            .sub_bullet("The scenery here is wonderful");

        let io = output
            .warning("It's too crowded here")
            .warning("I'm tired")
            .sub_bullet("The jumping fountains are great")
            .sub_bullet("The music is nice here")
            .done()
            .done();

        let expected = formatdoc! {"

            # RCT

            - Guest thoughts
              - The scenery here is wonderful

            ! It's too crowded here

            ! I'm tired

              - The jumping fountains are great
              - The music is nice here
            - Done (finished in < 0.1s)
        "};

        assert_eq!(expected, strip_ansi(String::from_utf8_lossy(&io)));
    }

    #[test]
    fn test_cmd() {
        let writer = Vec::new();
        let mut bullet = Print::new(writer)
            .h2("You must obey the dance commander")
            .bullet("Giving out the order for fun");

        bullet
            .stream_cmd(
                Command::new("bash")
                    .arg("-c")
                    .arg("echo it would be awesome"),
            )
            .unwrap();

        bullet
            .time_cmd(Command::new("bash").arg("-c").arg("echo if we could dance"))
            .unwrap();

        let io = bullet.done().done();
        let expected = formatdoc! {"

            ## You must obey the dance commander

            - Giving out the order for fun
              - Running `bash -c \"echo it would be awesome\"`

                  it would be awesome

              - Done (< 0.1s)
              - Running `bash -c \"echo if we could dance\"` ... (< 0.1s)
            - Done (finished in < 0.1s)
        "};
        assert_eq!(expected, strip_ansi(String::from_utf8_lossy(&io)));
    }
}
