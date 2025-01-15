// use crate::state;
use crate::util::ParagraphInspectWrite;
use crate::util::TrailingParagraph;
use crate::util::TrailingParagraphSend;
// use crate::write;
// use crate::Print;
use std::io::Write;
use std::sync::LazyLock;
use std::sync::Mutex;
// use std::time::Instant;

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
