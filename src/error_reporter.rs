use std::fmt;
use std::rc::Rc;

use text_buffer_2d::*;
use term;

use codemap::{self, Span};

#[derive(Debug)]
pub enum Error { unresolved_name }

#[derive(Debug)]
pub enum Label { primary, secondary }

pub struct ErrorReporter {
    kind: Error,
    primary_span: Span,
    labels: Vec<(Span, Label)>,
    cm: Rc<codemap::CodeMap>
}

impl fmt::Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.to_str().fmt(f)
    }
}

impl Level {
    pub fn color(self) -> term::color::Color {
        match self {
            Level::Bug | Level::Fatal | Level::PhaseFatal | Level::Error => term::color::BRIGHT_RED,
            Level::Warning => term::color::YELLOW,
            Level::Note => term::color::BRIGHT_GREEN,
            Level::Help => term::color::BRIGHT_CYAN,
            Level::Cancelled => unreachable!(),
        }
    }

    fn to_str(self) -> &'static str {
        match self {
            Level::Bug => "error: internal compiler error",
            Level::Fatal | Level::PhaseFatal | Level::Error => "error",
            Level::Warning => "warning",
            Level::Note => "note",
            Level::Help => "help",
            Level::Cancelled => panic!("Shouldn't call on cancelled error"),
        }
    }
}

impl ErrorReporter {
    pub fn span_label(&mut self, span: Span, label: Label) -> &mut ErrorReporter {
        self.labels.push((span, label));
        self
    }

    pub fn new(error: Error, primary_span: Span, cm: Rc<codemap::CodeMap>) -> ErrorReporter {
        ErrorReporter { kind: error, primary_span: primary_span, labels: vec![], cm: cm }
    }

    pub fn emit(&mut self) -> Vec<Vec<StyledString>> {
        let mut buffer = TextBuffer2D::new();
        let mut current_line = 0;

        for label in &self.labels {
            let label_text = match label {
                &(sp, Label::primary) => {
                    buffer.puts(current_line, 0, "Error", Style::LabelPrimary);
                    buffer.puts(current_line, 20, &self.cm.span_to_string(sp),
                        Style::UnderlinePrimary);
                }
                &(sp, Label::secondary) => {
                    buffer.puts(current_line, 0, "Error", Style::LabelSecondary);
                    buffer.puts(current_line, 20, &self.cm.span_to_string(sp),
                        Style::UnderlineSecondary);
                }
            };
            current_line += 1;
        }

        buffer.render()
    }
}

