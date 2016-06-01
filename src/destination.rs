use std::io::prelude::*;
use std::io;
use std::fmt;

use term;
use text_buffer_2d::*;

use error_reporter::*;

pub enum Destination {
    Terminal(Box<term::StderrTerminal>),
    Raw(Box<Write + Send>),
}

impl Destination {
    pub fn from_stderr() -> Destination {
        match term::stderr() {
            Some(t) => Destination::Terminal(t),
            None => Destination::Raw(Box::new(io::stderr())),
        }
    }

    pub fn apply_style(&mut self, lvl: Level, style: Style) -> io::Result<()> {
        match style {
            Style::FileNameStyle | Style::LineAndColumn => {}
            Style::LineNumber => {
                try!(self.start_attr(term::Attr::Bold));
                try!(self.start_attr(term::Attr::ForegroundColor(term::color::BRIGHT_BLUE)));
            }
            Style::Quotation => {}
            Style::OldSkoolNote => {
                try!(self.start_attr(term::Attr::Bold));
                try!(self.start_attr(term::Attr::ForegroundColor(term::color::BRIGHT_GREEN)));
            }
            Style::OldSkoolNoteText | Style::HeaderMsg => {
                try!(self.start_attr(term::Attr::Bold));
            }
            Style::UnderlinePrimary | Style::LabelPrimary => {
                try!(self.start_attr(term::Attr::Bold));
                try!(self.start_attr(term::Attr::ForegroundColor(lvl.color())));
            }
            Style::UnderlineSecondary |
            Style::LabelSecondary => {
                try!(self.start_attr(term::Attr::Bold));
                try!(self.start_attr(term::Attr::ForegroundColor(term::color::BRIGHT_BLUE)));
            }
            Style::NoStyle => {}
            Style::Level(Level::Error) => {
                try!(self.start_attr(term::Attr::Bold));
                try!(self.start_attr(term::Attr::ForegroundColor(term::color::BRIGHT_RED)));
            }
            Style::Level(Level::Warning) => {
                try!(self.start_attr(term::Attr::Bold));
                try!(self.start_attr(term::Attr::ForegroundColor(term::color::YELLOW)));
            }
            Style::Level(_) => {}
        }
        Ok(())
    }

    pub fn start_attr(&mut self, attr: term::Attr) -> io::Result<()> {
        match *self {
            Destination::Terminal(ref mut t) => {
                try!(t.attr(attr));
            }
            Destination::Raw(_) => {}
        }
        Ok(())
    }

    pub fn reset_attrs(&mut self) -> io::Result<()> {
        match *self {
            Destination::Terminal(ref mut t) => {
                try!(t.reset());
            }
            Destination::Raw(_) => {}
        }
        Ok(())
    }

    pub fn print_maybe_styled(&mut self,
                              args: fmt::Arguments,
                              color: term::Attr,
                              print_newline_at_end: bool)
                              -> io::Result<()> {
        match *self {
            Destination::Terminal(ref mut t) => {
                try!(t.attr(color));
                // If `msg` ends in a newline, we need to reset the color before
                // the newline. We're making the assumption that we end up writing
                // to a `LineBufferedWriter`, which means that emitting the reset
                // after the newline ends up buffering the reset until we print
                // another line or exit. Buffering the reset is a problem if we're
                // sharing the terminal with any other programs (e.g. other rustc
                // instances via `make -jN`).
                //
                // Note that if `msg` contains any internal newlines, this will
                // result in the `LineBufferedWriter` flushing twice instead of
                // once, which still leaves the opportunity for interleaved output
                // to be miscolored. We assume this is rare enough that we don't
                // have to worry about it.
                try!(t.write_fmt(args));
                try!(t.reset());
                if print_newline_at_end {
                    t.write_all(b"\n")
                } else {
                    Ok(())
                }
            }
            Destination::Raw(ref mut w) => {
                try!(w.write_fmt(args));
                if print_newline_at_end {
                    w.write_all(b"\n")
                } else {
                    Ok(())
                }
            }
        }
    }
}

impl Write for Destination {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        match *self {
            Destination::Terminal(ref mut t) => t.write(bytes),
            Destination::Raw(ref mut w) => w.write(bytes),
        }
    }
    fn flush(&mut self) -> io::Result<()> {
        match *self {
            Destination::Terminal(ref mut t) => t.flush(),
            Destination::Raw(ref mut w) => w.flush(),
        }
    }
}
