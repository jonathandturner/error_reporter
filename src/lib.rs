#![feature(question_mark)]
#![feature(range_contains)]

extern crate term;

use std::io::{self, Write};
use std::rc::Rc;

mod styled_buffer;
use styled_buffer::*;

mod compiler_message;
use compiler_message::*;

mod render_succinct;
use render_succinct::*;

mod styled_emit;
use styled_emit::*;

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

fn make_string(lines: Vec<Vec<StyledString>>) -> String {
    lines.iter()
        .flat_map(|rl| {
            rl.iter()
                .map(|s| &s.text[..])
                .chain(Some("\n"))
        })
        .collect()
}

#[test]
fn test_ellipsis() {
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
    let error_code = Some("E123".to_string());

    let mut err = CompilerMessage::new(Level::Error,
                                       String::from("Unresolved name"),
                                       span_vec0,
                                       error_code,
                                       cm);

    err.span_label(span_vec0, Some(String::from("primary message")));
    err.span_label(span_vec1, Some(String::from("secondary message")));

    let msg = render_succinct(&err);
    let text = make_string(msg);

    assert_eq!(&text[..],
               &r#"
error: Unresolved name [E123]
  --> foo.rs:15:4
   |>
5  |>    vec.pop();
   |>    --- secondary message
...
15 |>    vec.push(vec.pop().unwrap());
   |>    ^^^ primary message
"#[1..]);

}

#[test]
fn test_warning() {
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
    let error_code = Some("E123".to_string());

    let mut err = CompilerMessage::new(Level::Warning,
                                       String::from("Not sure what this is"),
                                       span_vec0,
                                       error_code,
                                       cm);

    err.span_label(span_vec0, Some(String::from("primary message")));
    err.span_label(span_vec1, Some(String::from("secondary message")));

    let msg = render_succinct(&err);
    let text = make_string(msg);

    assert_eq!(&text[..],
               &r#"
warning: Not sure what this is [E123]
 --> foo.rs:7:4
  |>
5 |>    vec.push(3);
  |>    --- secondary message
6 |>    vec.push(4);
7 |>    vec.push(5);
  |>    ^^^ primary message
"#[1..]);
}

#[test]
fn test_column_different_line_num_sizes() {
    let file_text = r#"
fn foo() {
    vec.push(vec.pop().unwrap());
}
"#;
    let file_text2 = r#"
fn bar() {
    //comment line
    //comment line
    //comment line
    //comment line
    //comment line
    //comment line
    //comment line
    //comment line
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
    let error_code = Some("E123".to_string());

    let mut err = CompilerMessage::new(Level::Warning,
                                       String::from("Not sure what this is"),
                                       span_vec0,
                                       error_code,
                                       cm);

    err.span_label(span_vec0, Some(String::from("primary message")));
    err.span_label(span_vec1, Some(String::from("secondary message")));
    err.span_label(span_vec2, Some(String::from("tertiary message")));

    let msg = render_succinct(&err);
    let text = make_string(msg);

    assert_eq!(&text[..],
               &r#"
warning: Not sure what this is [E123]
  --> foo.rs:3:13
   |>
3  |>    vec.push(vec.pop().unwrap());
   |>    ---      ^^^ primary message
   |>    |
   |>    secondary message
   |>
  ::: bar.rs
   |>
12 |>    vec2.push(vec2.pop().unwrap());
   |>              ---- tertiary message
"#[1..]);
}

#[test]
fn test_notes() {
    let file_text = r#"
fn foo() {
    vec.push(vec.pop().unwrap());
}
"#;
    let cm = Rc::new(CodeMap::new());
    let foo = cm.new_filemap_and_lines("foo.rs", file_text);
    let span_vec1 = cm.span_substr(&foo, file_text, "vec", 0);
    let span_vec0 = cm.span_substr(&foo, file_text, "vec", 1);
    let error_code = Some("E123".to_string());

    let mut err = CompilerMessage::new(Level::Error,
                                       String::from("Not sure what this is"),
                                       span_vec0,
                                       error_code,
                                       cm);

    err.span_label(span_vec0, Some(String::from("primary message")));
    err.span_label(span_vec1, Some(String::from("secondary message")));
    err.note(String::from("Are you sure you want to call it `vec`?"));

    let msg = render_succinct(&err);
    let text = make_string(msg);

    assert_eq!(&text[..],
               &r#"
error: Not sure what this is [E123]
 --> foo.rs:3:13
  |>
3 |>    vec.push(vec.pop().unwrap());
  |>    ---      ^^^ primary message
  |>    |
  |>    secondary message
  |>
  => note: Are you sure you want to call it `vec`?
"#[1..]);
}