use crate::util::ParagraphInspectWrite;
use crate::util::TrailingParagraph;
use crate::util::TrailingParagraphSend;
use std::io::Write;
use std::sync::LazyLock;
use std::sync::Mutex;

static WRITER: LazyLock<Mutex<Box<dyn TrailingParagraphSend>>> =
    LazyLock::new(|| Mutex::new(Box::new(ParagraphInspectWrite::new(std::io::stderr()))));

#[doc(hidden)]
pub struct _GlobalWriter;
impl Write for _GlobalWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut w = WRITER.lock().unwrap();
        w.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        let mut w = WRITER.lock().unwrap();
        w.flush()
    }
}

impl TrailingParagraph for _GlobalWriter {
    fn trailing_paragraph(&self) -> bool {
        let w = WRITER.lock().unwrap();
        w.trailing_paragraph()
    }

    fn trailing_newline_count(&self) -> usize {
        let w = WRITER.lock().unwrap();
        w.trailing_newline_count()
    }
}

/// Set the global writer
///
/// ```
/// bullet_stream::global::set_writer(std::io::stderr());
/// ```
///
/// # Panics
///
/// If you try to pass in a `_GlobalWriter`
pub fn set_writer<W>(new_writer: W)
where
    W: Write + Send + 'static,
{
    if std::any::Any::type_id(&new_writer) == std::any::TypeId::of::<_GlobalWriter>() {
        panic!("Cannot set the global writer to _GlobalWriter");
    } else {
        let mut writer = WRITER.lock().unwrap();
        *writer = Box::new(ParagraphInspectWrite::new(new_writer));
    }
}

#[cfg(feature = "global_functions")]
pub mod print {
    //! Print to a global writer without stateful protections
    //!
    //! The original [Print] struct provides maximum safety, which can cause max pain
    //! if you're trying to add pretty printing to a large codebase. If that's you,
    //! you can use these global functions to write output along with some help
    //! from the [crate::style] module.
    //!
    //! The downside is that there's no compilation guarantees for example: to prevent printing
    //! while a timer is running. Some basic consistency is still enforced such as newlines.
    //! If using this alongside of stateful output, use [Print::global] to ensure
    //! consistent newlines.
    //!
    //! Use [crate::global::set_writer] to configure the print output location.
    //!
    //! These functions are enabled by default. If you don't want the liability you can
    //! disable this feature in your cargo.toml file:
    //!
    //! ```toml
    //! bullet_stream = { default-features = false }
    //! ```

    use super::*;
    use crate::write;
    use crate::Print;
    use std::time::Instant;

    /// Output a h1 header to the global writer without state
    ///
    /// ```
    /// use bullet_stream::global::print;
    ///
    /// print::h1("I am a top level header");
    /// let duration = std::time::Instant::now();
    /// // ...
    /// print::all_done(&Some(duration));
    /// ```
    pub fn h1(s: impl AsRef<str>) {
        write::h1(&mut _GlobalWriter, s);
    }

    /// Output a h2 header to the global writer without state
    ///
    /// ```
    /// use bullet_stream::global::print;
    ///
    /// print::h1("I am a top level header");
    /// print::h2("I am a h2 header");
    /// let duration = std::time::Instant::now();
    /// // ...
    /// print::all_done(&Some(duration));
    /// ```
    pub fn h2(s: impl AsRef<str>) {
        write::h2(&mut _GlobalWriter, s);
    }

    /// Output a bullet point to the global writer without state
    ///
    /// ```
    /// use bullet_stream::global::print;
    ///
    /// print::bullet("Good point!");
    /// ```
    pub fn bullet(s: impl AsRef<str>) {
        write::bullet(&mut _GlobalWriter, s)
    }

    /// Output a subbullet point to the global writer without state
    ///
    /// ```
    /// use bullet_stream::global::print;
    ///
    /// print::bullet("Good point!");
    /// print::sub_bullet("Another good point!");
    /// ```
    pub fn sub_bullet(s: impl AsRef<str>) {
        write::sub_bullet(&mut _GlobalWriter, s);
    }

    /// Stream a command to the global writer without state
    ///
    /// ```no_run
    /// use bullet_stream::global::print;
    /// use fun_run::CommandWithName;
    ///
    /// let mut cmd = std::process::Command::new("bash");
    /// cmd.args(["-c", "echo 'hello world'"]);
    ///
    /// print::stream_with(format!("Running {}", cmd.name()), |stdout, stderr| {
    ///   cmd.stream_output(stdout, stderr)
    /// }).unwrap();
    /// ```
    pub fn stream_with<F, T>(s: impl AsRef<str>, f: F) -> T
    where
        F: FnMut(Box<dyn Write + Send + Sync>, Box<dyn Write + Send + Sync>) -> T,
        T: 'static,
    {
        write::stream_with(&mut _GlobalWriter, s, f)
    }

    /// Print dots to the global writer without state
    ///
    /// ```
    /// use bullet_stream::global::print;
    ///
    /// print::bullet("Ruby");
    /// let timer = print::start_timer("Installing");
    /// // ...
    /// timer.done();
    /// ```
    pub fn start_timer(s: impl AsRef<str>) -> Print<crate::state::Background<impl Write>> {
        write::start_timer(ParagraphInspectWrite::new(_GlobalWriter), Instant::now(), s)
    }

    /// Print an all done message with timing info to the UI
    ///
    /// ```
    /// use bullet_stream::global::print;
    ///
    /// print::h2("I am a h2 header");
    /// let duration = std::time::Instant::now();
    /// // ...
    /// print::all_done(&Some(duration));
    /// ```
    pub fn all_done(started: &Option<Instant>) {
        write::all_done(&mut _GlobalWriter, started);
    }

    /// Print a warning to the global writer without state
    ///
    /// ```
    /// use bullet_stream::global::print;
    ///
    /// print::warning("This town ain't big enough for the both of us");
    /// ```
    pub fn warning(s: impl AsRef<str>) {
        write::warning(&mut _GlobalWriter, s);
    }

    /// Print an error to the global writer without state
    ///
    /// ```
    /// use bullet_stream::global::print;
    ///
    /// print::error("Big problemo!");
    /// ```
    pub fn error(s: impl AsRef<str>) {
        write::error(&mut _GlobalWriter, s);
    }
}
