use crate::ansi_escape::ANSI;
use crate::background_printer::PrintGuard;
use crate::util::{
    format_stream_writer, mpsc_stream_to_output, prefix_first_rest_lines, prefix_lines,
    ParagraphInspectWrite, TrailingParagraph, TrailingParagraphSend,
};
use crate::{ansi_escape, background_printer, duration_format, state, style, Print};
use std::fmt::{Debug, Formatter};
use std::io::{self, Write};
use std::mem;
use std::sync::Arc;
use std::time::Instant;

pub(crate) fn h1<W: TrailingParagraph>(writer: &mut W, s: impl AsRef<str>) {
    if !writer.trailing_paragraph() {
        writeln!(writer).expect("writer open");
    }

    writeln!(
        writer,
        "{}",
        ansi_escape::wrap_ansi_escape_each_line(
            &ANSI::BoldPurple,
            format!("# {}", s.as_ref().trim()),
        ),
    )
    .expect("writer open");

    if !writer.trailing_paragraph() {
        writeln!(writer).expect("writer open");
    }
    writer.flush().expect("writer open");
}

pub(crate) fn h2<W: TrailingParagraph>(writer: &mut W, s: impl AsRef<str>) {
    if !writer.trailing_paragraph() {
        writeln!(writer).expect("writer open");
    }

    writeln!(
        writer,
        "{}",
        ansi_escape::wrap_ansi_escape_each_line(
            &ANSI::BoldPurple,
            format!("## {}", s.as_ref().trim()),
        ),
    )
    .expect("writer open");

    if !writer.trailing_paragraph() {
        writeln!(writer).expect("writer open");
    }
    writer.flush().expect("writer open");
}

pub(crate) fn h3<W: TrailingParagraph>(writer: &mut W, s: impl AsRef<str>) {
    if !writer.trailing_paragraph() {
        writeln!(writer).expect("writer open");
    }

    writeln!(
        writer,
        "{}",
        ansi_escape::wrap_ansi_escape_each_line(
            &ANSI::BoldPurple,
            format!("### {}", s.as_ref().trim()),
        ),
    )
    .expect("writer open");

    if !writer.trailing_paragraph() {
        writeln!(writer).expect("writer open");
    }
    writer.flush().expect("writer open");
}

pub(crate) fn bullet<W: Write>(writer: &mut W, s: impl AsRef<str>) {
    writeln!(
        writer,
        "{}",
        prefix_first_rest_lines("- ", "  ", s.as_ref().trim())
    )
    .expect("writer open");
    writer.flush().expect("writer open");
}

pub(crate) fn plain<W: Write>(writer: &mut W, s: impl AsRef<str>) {
    writeln!(writer, "{}", s.as_ref().trim_end()).expect("writer open");
    writer.flush().expect("writer open");
}

pub(crate) fn sub_bullet<W: Write>(writer: &mut W, s: impl AsRef<str>) {
    writeln!(writer, "{}", sub_bullet_prefix(s)).expect("writer open");
    writer.flush().expect("writer open");
}

pub(crate) fn sub_bullet_prefix(s: impl AsRef<str>) -> String {
    prefix_first_rest_lines("  - ", "    ", s.as_ref().trim())
}

#[cfg(feature = "fun_run")]
pub(crate) fn sub_stream_cmd<W: TrailingParagraphSend>(
    writer: &mut W,
    mut command: impl fun_run::CommandWithName,
) -> Result<fun_run::NamedOutput, fun_run::CmdError> {
    sub_stream_with(
        writer,
        crate::style::running_command(command.name()),
        |stdout, stderr| command.stream_output(stdout, stderr),
    )
}

#[cfg(feature = "fun_run")]
pub fn sub_time_cmd<W>(
    writer: ParagraphInspectWrite<W>,
    mut command: impl fun_run::CommandWithName,
) -> Result<fun_run::NamedOutput, fun_run::CmdError>
where
    W: Write + Send + Sync + 'static,
{
    let timer = sub_start_timer(
        writer,
        Instant::now(),
        style::running_command(command.name()),
    );
    let output = command.named_output();
    let _ = timer.done();
    output
}

pub(crate) fn sub_stream_with<W, T, F>(writer: &mut W, s: impl AsRef<str>, mut f: F) -> T
where
    W: TrailingParagraphSend,
    F: FnMut(Box<dyn Write + Send + Sync>, Box<dyn Write + Send + Sync>) -> T,
    T: 'static,
{
    sub_bullet(writer, s);
    writeln!(writer).expect("writer open");

    let duration = Instant::now();
    mpsc_stream_to_output(
        |sender| {
            f(
                // The Senders are boxed to hide the types from the caller so it can be changed
                // in the future. They only need to know they have a `Write + Send + Sync` type.
                Box::new(format_stream_writer(sender.clone())),
                Box::new(format_stream_writer(sender.clone())),
            )
        },
        move |recv| {
            // When it receives input, it writes it to the current `Write` value.
            //
            // When the senders close their channel this loop will exit
            for message in recv {
                writer.write_all(&message).expect("Writer to not be closed");
            }

            if !writer.trailing_paragraph() {
                writeln!(writer).expect("Writer to not be closed");
            }

            sub_bullet(
                writer,
                format!(
                    "Done {}",
                    style::details(duration_format::human(&duration.elapsed()))
                ),
            )
        },
    )
}

pub(crate) fn sub_start_timer<W>(
    writer: ParagraphInspectWrite<W>,
    started: Instant,
    s: impl AsRef<str>,
) -> Print<state::Background<W>>
where
    W: Write + Send + Sync + 'static,
{
    let guard = sub_start_print_interval(writer, s);

    Print {
        started: Some(started),
        state: state::Background {
            started: Instant::now(),
            write: guard,
        },
    }
}

pub(crate) fn sub_start_print_interval<W: Write + Send + Sync + 'static>(
    mut writer: W,
    s: impl AsRef<str>,
) -> PrintGuard<W> {
    // Do not emit a newline after the message
    write!(&mut writer, "{}", sub_bullet_prefix(s)).expect("writer not to be closed");
    writer.flush().expect("Output error: UI writer closed");

    background_printer::print_interval(
        writer,
        std::time::Duration::from_secs(1),
        ansi_escape::wrap_ansi_escape_each_line(&ANSI::Dim, " ."),
        ansi_escape::wrap_ansi_escape_each_line(&ANSI::Dim, "."),
        ansi_escape::wrap_ansi_escape_each_line(&ANSI::Dim, ". "),
        "(Error)".to_string(),
    )
}

pub(crate) fn all_done<W: Write>(writer: &mut W, started: &Option<Instant>) {
    if let Some(started) = started {
        bullet(
            writer,
            format!(
                "Done (finished in {})",
                duration_format::human(&started.elapsed())
            ),
        );
    } else {
        bullet(writer, "Done");
    }
}

pub(crate) fn write_paragraph<W: TrailingParagraph>(io: &mut W, color: &ANSI, s: impl AsRef<str>) {
    let contents = s.as_ref().trim();

    if !io.trailing_paragraph() {
        writeln!(io).expect("writer open");
    }

    writeln!(
        io,
        "{}",
        ansi_escape::wrap_ansi_escape_each_line(
            color,
            prefix_lines(contents, |_, line| {
                // Avoid adding trailing whitespace to the line, if there was none already.
                // The `\n` case is required since `prefix_lines` uses `str::split_inclusive`,
                // which preserves any trailing newline characters if present.
                if line.is_empty() || line == "\n" {
                    String::from("!")
                } else {
                    String::from("! ")
                }
            }),
        ),
    )
    .expect("writer open");
    writeln!(io).expect("writer open");
    io.flush().expect("writer open");
}

pub(crate) fn warning<W: TrailingParagraph>(writer: &mut W, s: impl AsRef<str>) {
    write_paragraph(writer, &ANSI::Yellow, s);
}

pub(crate) fn error<W: TrailingParagraph>(writer: &mut W, s: impl AsRef<str>) {
    write_paragraph(writer, &ANSI::Red, s);
}

pub(crate) fn important<W: TrailingParagraph>(writer: &mut W, s: impl AsRef<str>) {
    write_paragraph(writer, &ANSI::BoldCyan, s);
}

/// Constructs a writer that buffers written data until given marker byte is encountered and
/// then applies the given mapping function to the data before passing the result to the wrapped
/// writer.
pub fn mapped<W: io::Write, F: (Fn(Vec<u8>) -> Vec<u8>) + Sync + Send + 'static>(
    w: W,
    marker_byte: u8,
    f: F,
) -> MappedWrite<W> {
    MappedWrite::new(w, marker_byte, f)
}

/// Constructs a writer that buffers written data until an ASCII/UTF-8 newline byte (`b'\n'`) is
/// encountered and then applies the given mapping function to the data before passing the result to
/// the wrapped writer.
pub fn line_mapped<W: io::Write, F: (Fn(Vec<u8>) -> Vec<u8>) + Sync + Send + 'static>(
    w: W,
    f: F,
) -> MappedWrite<W> {
    mapped(w, b'\n', f)
}

/// A mapped writer that was created with the [`mapped`] or [`line_mapped`] function.
#[derive(Clone)]
pub struct MappedWrite<W: io::Write> {
    // To support unwrapping the inner `Write` while also implementing `Drop` for final cleanup, we need to wrap the
    // `W` value so we can replace it in memory during unwrap. Without the wrapping `Option` we'd need to have a way
    // to construct a bogus `W` value which would require additional trait bounds for `W`. `Clone` and/or `Default`
    // come to mind. Not only would this clutter the API, but for most values that implement `Write`, `Clone` or
    // `Default` are hard to implement correctly as they most often involve system resources such as file handles.
    //
    // This semantically means that a `MappedWrite` can exist without an inner `Write`, but users of `MappedWrite` can
    // never construct such a `MappedWrite` as it only represents a state that happens during `MappedWrite::unwrap`.
    //
    // See: https://rustwiki.org/en/error-index/#E0509
    inner: Option<W>,
    marker_byte: u8,
    buffer: Vec<u8>,
    mapping_fn: Arc<dyn (Fn(Vec<u8>) -> Vec<u8>) + Sync + Send>,
}

impl<W> MappedWrite<W>
where
    W: io::Write,
{
    fn new<F: (Fn(Vec<u8>) -> Vec<u8>) + Sync + Send + 'static>(
        w: W,
        marker_byte: u8,
        f: F,
    ) -> MappedWrite<W> {
        MappedWrite {
            inner: Some(w),
            marker_byte,
            buffer: Vec::new(),
            mapping_fn: Arc::new(f),
        }
    }

    pub fn unwrap(mut self) -> W {
        // See `Drop` implementation. This logic cannot be de-duplicated (i.e. by using unwrap in `Drop`) as we would
        // end up in illegal states.
        if self.inner.is_some() {
            let _result = self.map_and_write_current_buffer();
        }

        if let Some(inner) = self.inner.take() {
            inner
        } else {
            // Since `unwrap` is the only function that will cause `self.inner` to be `None` and `unwrap` itself
            // consumes the `MappedWrite`, we can be sure that this case never happens.
            unreachable!("self.inner will never be None")
        }
    }

    fn map_and_write_current_buffer(&mut self) -> io::Result<()> {
        match self.inner {
            Some(ref mut inner) => inner.write_all(&(self.mapping_fn)(mem::take(&mut self.buffer))),
            None => Ok(()),
        }
    }
}

impl<W: io::Write> io::Write for MappedWrite<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        for byte in buf {
            self.buffer.push(*byte);

            if *byte == self.marker_byte {
                self.map_and_write_current_buffer()?;
            }
        }

        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        match self.inner {
            Some(ref mut inner) => inner.flush(),
            None => Ok(()),
        }
    }
}

impl<W: io::Write> Drop for MappedWrite<W> {
    fn drop(&mut self) {
        // Drop implementations must not panic. We intentionally ignore the potential error here.
        let _result = self.map_and_write_current_buffer();
    }
}

impl<W: io::Write + Debug> Debug for MappedWrite<W> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MappedWrite")
            .field("inner", &self.inner)
            .field("marker_byte", &self.marker_byte)
            .field("buffer", &self.buffer)
            .field("mapping_fn", &"Fn()")
            .finish()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{strip_ansi, util::LockedWriter, write::line_mapped};
    use indoc::formatdoc;
    use pretty_assertions::assert_eq;
    use std::process::Command;

    #[test]
    fn plain_ending_newline() {
        let writer = LockedWriter::new(Vec::new());
        let reader = writer.clone();
        let mut writer = ParagraphInspectWrite::new(writer);

        let input = formatdoc! {"
            Accidental newline
        "};

        assert!(input.ends_with("\n"));
        plain(&mut writer, input);
        h2(&mut writer, "Then a header");
        drop(writer);

        assert_eq!(
            formatdoc! {"
                Accidental newline

                ## Then a header

            "},
            strip_ansi(String::from_utf8_lossy(&reader.unwrap()))
        );
    }

    #[test]
    fn test_mapped_write() {
        let mut output = Vec::new();

        let mut input = "foo\nbar\nbaz".as_bytes();
        std::io::copy(
            &mut input,
            &mut line_mapped(&mut output, |line| line.repeat(2)),
        )
        .unwrap();

        assert_eq!(output, "foo\nfoo\nbar\nbar\nbazbaz".as_bytes());
    }

    #[test]
    fn test_stream_cmd() {
        let writer = LockedWriter::new(Vec::new());
        let reader = writer.clone();
        self::sub_stream_cmd(
            &mut ParagraphInspectWrite::new(writer),
            Command::new("bash").arg("-c").arg("echo hello"),
        )
        .unwrap();

        let expected = formatdoc! {"
            - Running `bash -c \"echo hello\"`

                hello

            - Done (< 0.1s)

        "};
        assert_eq!(
            expected
                .trim_start()
                .lines()
                .map(|line| if line.is_empty() {
                    String::new()
                } else {
                    format!("  {line}")
                })
                .collect::<Vec<String>>()
                .join("\n"),
            strip_ansi(String::from_utf8_lossy(&reader.unwrap()))
        )
    }

    #[test]
    fn test_time_cmd() {
        let writer = LockedWriter::new(Vec::new());
        let reader = writer.clone();
        self::sub_time_cmd(
            ParagraphInspectWrite::new(writer),
            Command::new("bash").arg("-c").arg("echo hello"),
        )
        .unwrap();

        let expected = "- Running `bash -c \"echo hello\"` ... (< 0.1s)";
        assert_eq!(
            expected.trim(),
            strip_ansi(String::from_utf8_lossy(&reader.unwrap())).trim()
        )
    }
}
