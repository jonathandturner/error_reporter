#![feature(question_mark)]
#![feature(range_contains)]

extern crate term;

use std::io::{self, Write};
use std::rc::Rc;

mod styled_buffer;
use styled_buffer::*;

mod error_reporter;
use error_reporter::*;

mod emitter;
use emitter::*;

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
                   -> Span {
        let mut i = 0;
        let mut hi = 0;
        loop {
            let offset = source_text[hi..].find(substring).unwrap_or_else(|| {
                panic!("source_text `{}` does not have {} occurrences of `{}`, only {}",
                       source_text,
                       n,
                       substring,
                       i);
            });
            let lo = hi + offset;
            hi = lo + substring.len();
            if i == n {
                let span = Span {
                    lo: BytePos(lo as u32 + file.start_pos.0),
                    hi: BytePos(hi as u32 + file.start_pos.0),
                    expn_id: NO_EXPANSION,
                };
                assert_eq!(&self.span_to_snippet(span).unwrap()[..], substring);
                return span;
            }
            i += 1;
        }
    }
}

fn emit(level: Level, msg: Vec<Vec<StyledString>>) -> io::Result<()> {
    let mut dst = Destination::from_stderr();

    for line in msg {
        for part in line {
            dst.apply_style(level, part.style);
            write!(&mut dst, "{}", part.text);
            dst.reset_attrs()?;
        }
        write!(&mut dst, "\n");
    }
    Ok(())
}

fn test1() {
    let file_text = r#"
fn foo() {
    //blah blah
    //blah blah
    vec.pop();
    //blah blah
    //blah blah
    //blah blah
    //blah blah
    //blah blah
    //blah blah
    //blah blah
    //blah blah
    //blah blah
    vec.push(vec.pop().unwrap());
}
"#;
    let cm = Rc::new(CodeMap::new());
    let foo = cm.new_filemap_and_lines("foo.rs", file_text);
    let span_vec1 = cm.span_substr(&foo, file_text, "vec", 0);
    let span_vec0 = cm.span_substr(&foo, file_text, "vec", 1);

    let mut err = ErrorReporter::new(Level::Error, String::from("Unresolved name"), span_vec0, cm);

    err.span_label(span_vec0, Some(String::from("primary message")));
    err.span_label(span_vec1, Some(String::from("secondary message")));

    let msg = err.render();

    emit(Level::Error, msg);
}

fn test2() {
    let file_text = r#"
fn foo() {
    vec.push(1);
    vec.push(2);
    vec.push(3);
    vec.push(4);
    vec.push(5);
    vec.push(6);
}
"#;
    let cm = Rc::new(CodeMap::new());
    let foo = cm.new_filemap_and_lines("foo.rs", file_text);
    let span_vec1 = cm.span_substr(&foo, file_text, "vec", 2);
    let span_vec0 = cm.span_substr(&foo, file_text, "vec", 4);

    let mut err = ErrorReporter::new(Level::Warning,
                                     String::from("Not sure what this is"),
                                     span_vec0,
                                     cm);

    err.span_label(span_vec0, Some(String::from("primary message")));
    err.span_label(span_vec1, Some(String::from("secondary message")));

    let msg = err.render();

    emit(Level::Warning, msg);
}

fn test3() {
    let file_text = r#"
fn foo() {
    vec.push(vec.pop().unwrap());
}
"#;
    let file_text2 = r#"
fn bar() {
    //comment line
    vec2.push(vec2.pop().unwrap());
}
"#;
    let cm = Rc::new(CodeMap::new());
    let bar = cm.new_filemap_and_lines("bar.rs", file_text2);
    let foo = cm.new_filemap_and_lines("foo.rs", file_text);
    let span_vec1 = cm.span_substr(&foo, file_text, "vec", 0);
    let span_vec0 = cm.span_substr(&foo, file_text, "vec", 1);
    let span_vec2 = cm.span_substr(&bar, file_text2, "vec2", 1);

    let mut err = ErrorReporter::new(Level::Warning,
                                     String::from("Not sure what this is"),
                                     span_vec0,
                                     cm);

    err.span_label(span_vec0, Some(String::from("primary message")));
    err.span_label(span_vec1, Some(String::from("secondary message")));
    err.span_label(span_vec2, Some(String::from("tertiary message")));

    let msg = err.render();

    emit(Level::Warning, msg);
}

fn main() {
    test1();
    println!("");
    test2();
    println!("");
    test3();
}