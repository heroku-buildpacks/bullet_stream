use crate::util::ParagraphInspectWrite;
use crate::util::TrailingParagraph;
use crate::util::TrailingParagraphSend;
use std::any::Any;
use std::cell::Cell;
use std::io::Write;
use std::panic::catch_unwind;
use std::panic::resume_unwind;
use std::panic::AssertUnwindSafe;
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
        let mut w = WRITER.lock().map_err(|_| {
            std::io::Error::other("GlobalWriter lock poisoned - cannot guarantee data consistency")
        })?;
        w.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        let mut w = WRITER.lock().map_err(|_| {
            std::io::Error::other("GlobalWriter lock poisoned - cannot guarantee data consistency")
        })?;
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
/// - If you try to pass in a `GlobalWriter`
/// - If you try to call `set_writer` inside of `with_locked_writer`
pub fn set_writer<W>(new_writer: W)
where
    W: Write + Send + 'static,
{
    if std::any::Any::type_id(&new_writer) == std::any::TypeId::of::<GlobalWriter>() {
        panic!("Cannot set the global writer to GlobalWriter");
    }

    let _global_lock = WITH_WRITER_GLOBAL_LOCK
        .try_lock()
        .expect("Cannot call `set_writer` inside of `with_locked_writer`");

    let mut writer = WRITER.lock().unwrap();
    *writer = Box::new(ParagraphInspectWrite::new(new_writer));
}

static WITH_WRITER_GLOBAL_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| ().into());
thread_local! {
    static WITH_WRITER_REENTRANT_CHECK: Cell<bool> = const { Cell::new(false) };
}

/// RAII guard for preventing reentrant calls to `with_locked_writer`
///
/// This guard automatically resets the reentrant check flag when dropped,
/// ensuring proper cleanup even if the guarded code panics.
struct ReentrantGuard;

impl ReentrantGuard {
    /// Creates a new guard, panicking if already set (indicating reentrant call)
    fn new() -> Self {
        WITH_WRITER_REENTRANT_CHECK.with(|only_once| {
            if only_once.get() {
                panic!("Cannot call this function recursively!");
            }
            only_once.set(true);
        });
        ReentrantGuard
    }
}

impl Drop for ReentrantGuard {
    fn drop(&mut self) {
        WITH_WRITER_REENTRANT_CHECK.with(|only_once| {
            only_once.set(false);
        });
    }
}

/// Sets the global writer for the duration of the provided closure
///
/// This is meant to be used in tests where the order of writes are important.
///
/// ```rust
/// use bullet_stream::global::{self, print};
///
/// let out = global::with_locked_writer(Vec::<u8>::new(), || {
///   print::bullet("Hello world");
/// });
/// assert_eq!("- Hello world\n".to_string(), String::from_utf8_lossy(&out));
///
/// let out = global::with_locked_writer(Vec::<u8>::new(), || {
///   print::bullet("Knock, knock, Neo");
/// });
/// assert_eq!("- Knock, knock, Neo\n".to_string(), String::from_utf8_lossy(&out));
/// ```
///
/// Guarantees that only one invocation of this is called at a time. Returns the provided
/// writer on completion. Panics if called recursively in the same thread.
///
/// # Panics
///
/// - If you mutate the global writer via another mechanism (such as calling `global::set_writer`)
///   from within this thread.
///
/// ```should_panic
/// use bullet_stream::global;
///
/// global::with_locked_writer(Vec::<u8>::new(), || {
///     global::set_writer(Vec::<u8>::new());
/// });
/// ```
///
/// - If you try to call the function recursively in the same thread:
///
/// ```should_panic
/// use bullet_stream::global;
///
/// global::with_locked_writer(Vec::<u8>::new(), || {
///     global::with_locked_writer(Vec::<u8>::new(), || {
///         //
///     });
/// });
/// ```
///
/// - If you try to pass in a `GlobalWriter`
///
/// ```should_panic
/// use bullet_stream::global;
///
/// global::with_locked_writer(global::GlobalWriter, || {
///     //
/// });
/// ```
pub fn with_locked_writer<W, F>(new_writer: W, f: F) -> W
where
    W: Write + Send + Any + 'static,
    F: FnOnce(),
{
    if std::any::Any::type_id(&new_writer) == std::any::TypeId::of::<GlobalWriter>() {
        panic!("Cannot set the global writer to GlobalWriter");
    }
    // Ensure all locks are dropped on panic, this supports test assertion failures
    // without poisoning locks
    let writer_or_panic = {
        // Panic if called recursively, must come before lock to prevent deadlock
        let _reentrant_guard = ReentrantGuard::new();
        let _global_lock = WITH_WRITER_GLOBAL_LOCK.lock().unwrap();
        let old_writer = {
            let mut write_lock = WRITER.lock().unwrap();
            std::mem::replace(
                &mut *write_lock,
                Box::new(ParagraphInspectWrite::new(new_writer)),
            )
        };

        let f_panic = catch_unwind(AssertUnwindSafe(f));

        let new_writer = {
            let mut write_lock = WRITER.lock().unwrap();
            std::mem::replace(&mut *write_lock, old_writer)
        };

        if let Ok(original) = (new_writer as Box<dyn Any>).downcast::<ParagraphInspectWrite<W>>() {
            f_panic.map(|_| original.inner)
        } else {
            panic!("Could not downcast to original type. Writer was mutated unexpectedly")
        }
    };
    writer_or_panic.unwrap_or_else(|payload| resume_unwind(payload))
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
    /// # use pretty_assertions::assert_eq;
    /// #
    /// # let output = bullet_stream::global::with_locked_writer(Vec::<u8>::new(), ||{
    ///
    /// let duration = std::time::Instant::now();
    /// print::h1("I am a top level header");
    ///
    /// print::all_done(&Some(duration));
    /// # });
    ///
    /// let expected = indoc::formatdoc!{"
    ///
    ///   ## I am a top level header
    ///
    ///   - Done (finished in < 0.1s)
    /// "};
    /// assert_eq!(expected, bullet_stream::strip_ansi(String::from_utf8_lossy(&output)));
    /// ```
    pub fn h1(s: impl AsRef<str>) {
        write::h1(&mut GlobalWriter, s);
    }

    /// Output a h2 header to the global writer without state
    ///
    /// ```
    /// use bullet_stream::global::print;
    /// # use pretty_assertions::assert_eq;
    /// #
    /// # let output = bullet_stream::global::with_locked_writer(Vec::<u8>::new(), ||{
    ///
    /// print::h1("I am a top level header");
    /// print::h2("I am an h2 header");
    /// let duration = std::time::Instant::now();
    /// // ...
    /// print::all_done(&Some(duration));
    /// # });
    ///
    /// let expected = indoc::formatdoc!{"
    ///
    ///   ## I am a top level header
    ///
    ///   ### I am an h2 header
    ///
    ///   - Done (finished in < 0.1s)
    /// "};
    /// assert_eq!(expected, bullet_stream::strip_ansi(String::from_utf8_lossy(&output)));
    /// ```
    pub fn h2(s: impl AsRef<str>) {
        write::h2(&mut GlobalWriter, s);
    }

    /// Output a h3 header to the global writer without state
    ///
    /// ```
    /// use bullet_stream::global::print;
    /// # use pretty_assertions::assert_eq;
    /// #
    /// # let output = bullet_stream::global::with_locked_writer(Vec::<u8>::new(), ||{
    ///
    /// print::h1("I am a top level header");
    /// print::h2("I am an h2 header");
    /// print::h3("I am an h3 header");
    /// let duration = std::time::Instant::now();
    /// // ...
    /// print::all_done(&Some(duration));
    /// # });
    ///
    /// let expected = indoc::formatdoc!{"
    ///
    ///   ## I am a top level header
    ///
    ///   ### I am an h2 header
    ///
    ///   #### I am an h3 header
    ///
    ///   - Done (finished in < 0.1s)
    /// "};
    /// assert_eq!(expected, bullet_stream::strip_ansi(String::from_utf8_lossy(&output)));
    /// ```
    pub fn h3(s: impl AsRef<str>) {
        write::h3(&mut GlobalWriter, s);
    }

    /// Output plain text
    ///
    /// Like `println!` but it writes to the shared global
    /// writer.
    ///
    /// ```
    /// use bullet_stream::global::print;
    /// # use pretty_assertions::assert_eq;
    /// #
    /// # let output = bullet_stream::global::with_locked_writer(Vec::<u8>::new(), ||{
    ///
    /// print::plain("This almost seems silly.");
    /// print::plain("But it auto-flushes IO.");
    /// print::plain("Which is nice.");
    /// # });
    ///
    /// let expected = indoc::formatdoc!{"
    ///   This almost seems silly.
    ///   But it auto-flushes IO.
    ///   Which is nice.
    /// "};
    /// assert_eq!(expected, bullet_stream::strip_ansi(String::from_utf8_lossy(&output)));
    /// ```
    pub fn plain(s: impl AsRef<str>) {
        write::plain(&mut GlobalWriter, s)
    }

    /// Announce the name of a buildpack
    ///
    /// Use together with [all_done]
    /// ```
    /// use bullet_stream::global::print;
    /// # use pretty_assertions::assert_eq;
    /// #
    /// # let output = bullet_stream::global::with_locked_writer(Vec::<u8>::new(), ||{
    ///
    /// let started = print::buildpack("Heroku Awesome Buildpack");
    /// print::bullet("Just add awesome.");
    /// print::all_done(&Some(started));
    /// # });
    ///
    /// let expected = indoc::formatdoc!{"
    ///
    ///   ### Heroku Awesome Buildpack
    ///
    ///   - Just add awesome.
    ///   - Done (finished in < 0.1s)
    /// "};
    /// assert_eq!(expected, bullet_stream::strip_ansi(String::from_utf8_lossy(&output)));
    /// ```
    pub fn buildpack(s: impl AsRef<str>) -> Instant {
        write::h2(&mut GlobalWriter, s);
        Instant::now()
    }

    /// Header to break up subsections in a buildpack's output
    ///
    /// ```
    /// use bullet_stream::global::print;
    /// # use pretty_assertions::assert_eq;
    /// #
    /// # let output = bullet_stream::global::with_locked_writer(Vec::<u8>::new(), ||{
    ///
    /// let started = print::buildpack("FYEO Buildpack");
    ///
    /// print::header("the branches bending low");
    /// print::bullet("Tracks");
    /// print::sub_bullet("all the windows are glowing");
    /// print::sub_bullet("looking in between those long reeds");
    /// print::bullet("Released");
    /// print::sub_bullet("2024");
    ///
    /// print::header("failed book plots");
    /// print::bullet("Tracks");
    /// print::sub_bullet("the stream at new river beach ");
    /// print::sub_bullet("a line that is broad");
    /// print::bullet("Released");
    /// print::sub_bullet("2023");
    ///
    /// print::all_done(&Some(started));
    /// # });
    ///
    /// let expected = indoc::formatdoc!{"
    ///
    ///   ### FYEO Buildpack
    ///
    ///   #### the branches bending low
    ///
    ///   - Tracks
    ///     - all the windows are glowing
    ///     - looking in between those long reeds
    ///   - Released
    ///     - 2024
    ///
    ///   #### failed book plots
    ///
    ///   - Tracks
    ///     - the stream at new river beach
    ///     - a line that is broad
    ///   - Released
    ///     - 2023
    ///   - Done (finished in < 0.1s)
    /// "};
    /// assert_eq!(expected, bullet_stream::strip_ansi(String::from_utf8_lossy(&output)));
    /// ```
    pub fn header(s: impl AsRef<str>) {
        write::h3(&mut GlobalWriter, s);
    }

    /// Output a bullet point to the global writer without state
    ///
    /// ```
    /// use bullet_stream::global::print;
    /// # use pretty_assertions::assert_eq;
    /// #
    /// # let output = bullet_stream::global::with_locked_writer(Vec::<u8>::new(), ||{
    ///
    /// print::bullet("Good point!");
    /// # });
    ///
    /// let expected = indoc::formatdoc!{"
    ///   - Good point!
    /// "};
    /// assert_eq!(expected, bullet_stream::strip_ansi(String::from_utf8_lossy(&output)));
    /// ```
    pub fn bullet(s: impl AsRef<str>) {
        write::bullet(&mut GlobalWriter, s)
    }

    /// Output a sub-bullet point to the global writer without state
    ///
    /// ```
    /// use bullet_stream::global::print;
    /// # use pretty_assertions::assert_eq;
    /// #
    /// # let output = bullet_stream::global::with_locked_writer(Vec::<u8>::new(), ||{
    ///
    /// print::bullet("Good point!");
    /// print::sub_bullet("Another good point!");
    /// # });
    ///
    /// let expected = indoc::formatdoc!{"
    ///   - Good point!
    ///     - Another good point!
    /// "};
    /// assert_eq!(expected, bullet_stream::strip_ansi(String::from_utf8_lossy(&output)));
    /// ```
    pub fn sub_bullet(s: impl AsRef<str>) {
        write::sub_bullet(&mut GlobalWriter, s);
    }

    /// Print a sub-bullet and stream a command to the global writer without state
    ///
    /// ```no_run
    /// use bullet_stream::global::print;
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
    /// use bullet_stream::global::print;
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
    /// use bullet_stream::global::print;
    /// # use pretty_assertions::assert_eq;
    /// #
    /// # let output = bullet_stream::global::with_locked_writer(Vec::<u8>::new(), ||{
    ///
    /// print::bullet("Ruby");
    /// let timer = print::sub_start_timer("Installing");
    /// // ...
    /// timer.done();
    /// # });
    ///
    /// let expected = indoc::formatdoc!{"
    ///   - Ruby
    ///     - Installing ... (< 0.1s)
    /// "};
    /// assert_eq!(expected, bullet_stream::strip_ansi(String::from_utf8_lossy(&output)));
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
    /// use bullet_stream::global::print;
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
    /// use bullet_stream::global::print;
    /// # use pretty_assertions::assert_eq;
    /// #
    /// # let output = bullet_stream::global::with_locked_writer(Vec::<u8>::new(), ||{
    ///
    /// print::h2("I am an h2 header");
    /// let duration = std::time::Instant::now();
    /// // ...
    /// print::all_done(&Some(duration));
    /// # });
    ///
    /// let expected = indoc::formatdoc!{"
    ///
    ///   ### I am an h2 header
    ///
    ///   - Done (finished in < 0.1s)
    /// "};
    /// assert_eq!(expected, bullet_stream::strip_ansi(String::from_utf8_lossy(&output)));
    /// ```
    pub fn all_done(started: &Option<Instant>) {
        write::all_done(&mut GlobalWriter, started);
    }

    /// Print a warning to the global writer without state
    ///
    /// ```
    /// use bullet_stream::global::print;
    /// # use pretty_assertions::assert_eq;
    /// #
    /// # let output = bullet_stream::global::with_locked_writer(Vec::<u8>::new(), ||{
    ///
    /// print::warning("This town ain't\nbig enough\nfor the both of us");
    /// # });
    ///
    /// let expected = indoc::formatdoc!{"
    ///
    ///   ! This town ain't
    ///   ! big enough
    ///   ! for the both of us
    ///
    /// "};
    /// assert_eq!(expected, bullet_stream::strip_ansi(String::from_utf8_lossy(&output)));
    /// ```
    pub fn warning(s: impl AsRef<str>) {
        write::warning(&mut GlobalWriter, s);
    }

    /// Print an error to the global writer without state
    ///
    /// ```
    /// use bullet_stream::global::print;
    /// # use pretty_assertions::assert_eq;
    /// #
    /// # let output = bullet_stream::global::with_locked_writer(Vec::<u8>::new(), ||{
    /// use indoc::formatdoc;
    ///
    /// print::error(formatdoc! {"
    ///     It's at times like this, when I'm trapped in a Vogon
    ///     airlock with a man from Betelgeuse, and about to die of asphyxiation
    ///     in deep space that I really wish I'd listened to what my mother told
    ///     me when I was young
    /// "});
    /// # });
    ///
    /// let expected = indoc::formatdoc!{"
    ///
    ///   ! It's at times like this, when I'm trapped in a Vogon
    ///   ! airlock with a man from Betelgeuse, and about to die of asphyxiation
    ///   ! in deep space that I really wish I'd listened to what my mother told
    ///   ! me when I was young
    ///
    /// "};
    /// assert_eq!(expected, bullet_stream::strip_ansi(String::from_utf8_lossy(&output)));
    /// ```
    pub fn error(s: impl AsRef<str>) {
        write::error(&mut GlobalWriter, s);
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::strip_ansi;
    use indoc::formatdoc;
    use pretty_assertions::assert_eq;
    use std::panic;
    use std::thread;

    #[test]
    fn with_locked_writer_handles_panics_across_threads() {
        let handle1 = thread::spawn(|| {
            panic::catch_unwind(|| {
                with_locked_writer(Vec::new(), || {
                    print::bullet("About to panic");
                    panic!("Intentional panic for testing");
                });
            })
        });

        let result = handle1
            .join()
            .expect("First thread should complete successfully");

        assert!(result.is_err(), "Expected panic to be caught {:?}", result);

        let handle2 = thread::spawn(|| {
            let output = with_locked_writer(Vec::new(), || {
                print::bullet("This should work fine");
                print::sub_bullet("Even after another thread panicked");
            });

            let expected = formatdoc! {"
                - This should work fine
                  - Even after another thread panicked
            "};

            assert_eq!(expected, strip_ansi(String::from_utf8_lossy(&output)));
        });

        handle2
            .join()
            .expect("Second thread should complete successfully");

        let output = with_locked_writer(Vec::new(), || {
            print::bullet("Main thread still works");
        });

        let expected = "- Main thread still works\n";
        assert_eq!(expected, strip_ansi(String::from_utf8_lossy(&output)));
    }
}
