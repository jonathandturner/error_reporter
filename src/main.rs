extern crate term;

mod text_buffer_2d;
use text_buffer_2d::*;

mod error_reporter;
use error_reporter::*;

mod destination;

fn main() {
    let mut err = ErrorReporter::new(Error::unresolved_name);
    err.span_label(1, Label::primary);
    err.span_label(3, Label::primary);
    println!("{:?}", err.emit());
}
