use crate::util::ParagraphInspectWrite;
use crate::util::TrailingParagraph;
use crate::util::TrailingParagraphSend;
use std::io::Write;
use std::sync::LazyLock;
use std::sync::Mutex;

static WRITER: LazyLock<Mutex<Box<dyn TrailingParagraphSend>>> =
    LazyLock::new(|| Mutex::new(Box::new(ParagraphInspectWrite::new(std::io::stderr()))));

/// A marker struct for writing to a global writer
///
/// Use [set_writer] to change the destination.
///
/// It is okay to rely on this struct as a `W` for the [crate::Print::global()]
/// return type like `Print<SubBullet<GlobalWriter>>`. You shouldn't use it
/// much outside of that. Its behavior may change.
///
/// To avoid this struct from showing up in your interfaces you can use
/// generics `W: Write + Send + Sync + 'static` instead.
pub struct GlobalWriter;

#[deprecated(since = "0.5.0", note = "_GlobalWriter use GlobalWriter instead")]
pub type _GlobalWriter = GlobalWriter;

impl Write for GlobalWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut w = WRITER.lock().unwrap();
        w.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        let mut w = WRITER.lock().unwrap();
        w.flush()
    }
}

impl TrailingParagraph for GlobalWriter {
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
    if std::any::Any::type_id(&new_writer) == std::any::TypeId::of::<GlobalWriter>() {
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
    #[doc = include_str!("./docs/global_setup.rs")]
    ///
    /// print::h1("I am a top level header");
    /// let duration = std::time::Instant::now();
    /// // ...
    /// print::all_done(&Some(duration));
    ///
    #[doc = include_str!("./docs/global_done_one.rs")]
    /// ## I am a top level header
    ///
    /// - Done (finished in < 0.1s)
    #[doc = include_str!("./docs/global_done_two.rs")]
    /// ```
    pub fn h1(s: impl AsRef<str>) {
        write::h1(&mut GlobalWriter, s);
    }

    /// Output a h2 header to the global writer without state
    ///
    /// ```
    #[doc = include_str!("./docs/global_setup.rs")]
    ///
    /// print::h1("I am a top level header");
    /// print::h2("I am an h2 header");
    /// let duration = std::time::Instant::now();
    /// // ...
    /// print::all_done(&Some(duration));
    ///
    #[doc = include_str!("./docs/global_done_one.rs")]
    /// ## I am a top level header
    ///
    /// ### I am an h2 header
    ///
    /// - Done (finished in < 0.1s)
    #[doc = include_str!("./docs/global_done_two.rs")]
    /// ```
    pub fn h2(s: impl AsRef<str>) {
        write::h2(&mut GlobalWriter, s);
    }

    /// Output plain text
    ///
    /// Like `println!` but it writes to the shared global
    /// writer.
    ///
    /// ```
    #[doc = include_str!("./docs/global_setup.rs")]
    ///
    /// print::plain("This almost seems silly.");
    /// print::plain("But it auto-flushes IO.");
    /// print::plain("Which is nice.");
    #[doc = include_str!("./docs/global_done_one.rs")]
    /// This almost seems silly.
    /// But it auto-flushes IO.
    /// Which is nice.
    #[doc = include_str!("./docs/global_done_two.rs")]
    /// ```
    pub fn plain(s: impl AsRef<str>) {
        write::plain(&mut GlobalWriter, s)
    }

    /// Output a bullet point to the global writer without state
    ///
    /// ```
    #[doc = include_str!("./docs/global_setup.rs")]
    ///
    /// print::bullet("Good point!");
    #[doc = include_str!("./docs/global_done_one.rs")]
    /// - Good point!
    #[doc = include_str!("./docs/global_done_two.rs")]
    /// ```
    pub fn bullet(s: impl AsRef<str>) {
        write::bullet(&mut GlobalWriter, s)
    }

    /// Output a sub-bullet point to the global writer without state
    ///
    /// ```
    #[doc = include_str!("./docs/global_setup.rs")]
    ///
    /// print::bullet("Good point!");
    /// print::sub_bullet("Another good point!");
    ///
    #[doc = include_str!("./docs/global_done_one.rs")]
    /// - Good point!
    ///   - Another good point!
    #[doc = include_str!("./docs/global_done_two.rs")]
    /// ```
    pub fn sub_bullet(s: impl AsRef<str>) {
        write::sub_bullet(&mut GlobalWriter, s);
    }

    /// Print a sub-bullet and stream a command to the global writer without state
    ///
    /// ```no_run
    #[doc = include_str!("./docs/global_setup.rs")]
    /// use fun_run::CommandWithName;
    ///
    /// let mut cmd = std::process::Command::new("bash");
    /// cmd.args(["-c", "echo 'hello world'"]);
    ///
    /// print::sub_stream_with(format!("Running {}", cmd.name()), |stdout, stderr| {
    ///   cmd.stream_output(stdout, stderr)
    /// }).unwrap();
    /// ```
    pub fn sub_stream_with<F, T>(s: impl AsRef<str>, f: F) -> T
    where
        F: FnMut(Box<dyn Write + Send + Sync>, Box<dyn Write + Send + Sync>) -> T,
        T: 'static,
    {
        write::sub_stream_with(&mut GlobalWriter, s, f)
    }

    /// Print the name of a command then stream it
    ///
    /// This provieds convienence and standardization over [sub_stream_with]
    ///
    /// ```no_run
    #[doc = include_str!("./docs/global_setup.rs")]
    /// use fun_run::CommandWithName;
    ///
    /// print::sub_stream_cmd(
    ///     std::process::Command::new("bash")
    ///         .args(["-c", "echo 'hello world'"])
    /// ).unwrap();
    /// ```
    #[cfg(feature = "fun_run")]
    pub fn sub_stream_cmd(
        command: impl fun_run::CommandWithName,
    ) -> Result<fun_run::NamedOutput, fun_run::CmdError> {
        write::sub_stream_cmd(&mut GlobalWriter, command)
    }

    /// Print a sub-bullet and then emmit dots to the global writer without state
    ///
    /// ```
    #[doc = include_str!("./docs/global_setup.rs")]
    ///
    /// print::bullet("Ruby");
    /// let timer = print::sub_start_timer("Installing");
    /// // ...
    /// timer.done();
    #[doc = include_str!("./docs/global_done_one.rs")]
    /// - Ruby
    ///   - Installing ... (< 0.1s)
    #[doc = include_str!("./docs/global_done_two.rs")]
    /// ```
    pub fn sub_start_timer(s: impl AsRef<str>) -> Print<crate::state::Background<impl Write>> {
        write::sub_start_timer(ParagraphInspectWrite::new(GlobalWriter), Instant::now(), s)
    }

    /// Prints the name of a command and times (with dots) it in the background
    ///
    /// Does not show the output of the command. If you need that use [sub_stream_cmd]
    ///
    /// Provides convience and standardization over [sub_start_timer].
    ///
    /// ```no_run
    #[doc = include_str!("./docs/global_setup.rs")]
    /// use fun_run::CommandWithName;
    ///
    /// print::sub_time_cmd(
    ///     std::process::Command::new("bash")
    ///         .args(["-c", "echo 'hello world'"])
    /// ).unwrap();
    /// ```
    ///
    #[cfg(feature = "fun_run")]
    pub fn sub_time_cmd(
        command: impl fun_run::CommandWithName,
    ) -> Result<fun_run::NamedOutput, fun_run::CmdError> {
        write::sub_time_cmd(ParagraphInspectWrite::new(GlobalWriter), command)
    }

    /// Print an all done message with timing info to the UI
    ///
    /// ```
    #[doc = include_str!("./docs/global_setup.rs")]
    ///
    /// print::h2("I am an h2 header");
    /// let duration = std::time::Instant::now();
    /// // ...
    /// print::all_done(&Some(duration));
    ///
    #[doc = include_str!("./docs/global_done_one.rs")]
    /// ### I am an h2 header
    ///
    /// - Done (finished in < 0.1s)
    #[doc = include_str!("./docs/global_done_two.rs")]
    /// ```
    pub fn all_done(started: &Option<Instant>) {
        write::all_done(&mut GlobalWriter, started);
    }

    /// Print a warning to the global writer without state
    ///
    /// ```
    #[doc = include_str!("./docs/global_setup.rs")]
    ///
    /// print::warning("This town ain't\nbig enough\nfor the both of us");
    #[doc = include_str!("./docs/global_done_one.rs")]
    ///
    /// ! This town ain't
    /// ! big enough
    /// ! for the both of us
    ///
    #[doc = include_str!("./docs/global_done_two.rs")]
    /// ```
    pub fn warning(s: impl AsRef<str>) {
        write::warning(&mut GlobalWriter, s);
    }

    /// Print an error to the global writer without state
    ///
    /// ```
    #[doc = include_str!("./docs/global_setup.rs")]
    /// use indoc::formatdoc;
    ///
    /// print::error(formatdoc! {"
    ///     It's at times like this, when I'm trapped in a Vogon
    ///     airlock with a man from Betelgeuse, and about to die of asphyxiation
    ///     in deep space that I really wish I'd listened to what my mother told
    ///     me when I was young
    /// "});
    ///
    #[doc = include_str!("./docs/global_done_one.rs")]
    ///
    /// ! It's at times like this, when I'm trapped in a Vogon
    /// ! airlock with a man from Betelgeuse, and about to die of asphyxiation
    /// ! in deep space that I really wish I'd listened to what my mother told
    /// ! me when I was young
    ///
    #[doc = include_str!("./docs/global_done_two.rs")]
    /// ```
    pub fn error(s: impl AsRef<str>) {
        write::error(&mut GlobalWriter, s);
    }
}
