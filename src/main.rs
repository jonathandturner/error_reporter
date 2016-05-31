#![feature(question_mark)]

extern crate term;

use std::io::{self, Write};
use std::rc::Rc;

mod text_buffer_2d;
use text_buffer_2d::*;

mod error_reporter;
use error_reporter::*;

mod destination;
use destination::*;

mod codemap;
use codemap::*;

trait CodeMapExtension {
    fn span_substr(&self,
                   file: &Rc<FileMap>,
                   source_text: &str,
                   substring: &str,
                   n: usize)
                   -> Span;
}

impl CodeMapExtension for CodeMap {
    fn span_substr(&self,
                   file: &Rc<FileMap>,
                   source_text: &str,
                   substring: &str,
                   n: usize)
                   -> Span
    {
        let mut i = 0;
        let mut hi = 0;
        loop {
            let offset = source_text[hi..].find(substring).unwrap_or_else(|| {
                panic!("source_text `{}` does not have {} occurrences of `{}`, only {}",
                       source_text, n, substring, i);
            });
            let lo = hi + offset;
            hi = lo + substring.len();
            if i == n {
                let span = Span {
                    lo: BytePos(lo as u32 + file.start_pos.0),
                    hi: BytePos(hi as u32 + file.start_pos.0),
                    expn_id: NO_EXPANSION,
                };
                assert_eq!(&self.span_to_snippet(span).unwrap()[..],
                           substring);
                return span;
            }
            i += 1;
        }
    }
}

fn render(msg: Vec<Vec<StyledString>>) -> io::Result<()> {
    let mut dst = Destination::from_stderr();

    for line in msg {
        for part in line {
            dst.apply_style(Level::Error, part.style);
            write!(&mut dst, "{}", part.text);
            dst.reset_attrs()?;
        }
        write!(&mut dst, "\n");
    }
    Ok(())
}

fn main() {
    let file_text = r#"
fn foo() {
    vec.push(vec.pop().unwrap());
}
"#;
    let cm = Rc::new(CodeMap::new());
    let foo = cm.new_filemap_and_lines("foo.rs", file_text);
    let span_vec0 = cm.span_substr(&foo, file_text, "vec", 0);
    let span_vec1 = cm.span_substr(&foo, file_text, "vec", 1);

    let mut err = ErrorReporter::new(Error::unresolved_name, span_vec0, cm);

    err.span_label(span_vec1, Label::primary);
    err.span_label(span_vec0, Label::secondary);

    let msg = err.emit();

    render(msg);
}
