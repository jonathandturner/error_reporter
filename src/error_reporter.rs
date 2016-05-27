use std::fmt;

use text_buffer_2d::*;
use term;

#[derive(Copy, PartialEq, Clone, Debug)]
pub enum Level {
    Bug,
    Fatal,
    // An error which while not immediately fatal, should stop the compiler
    // progressing beyond the current phase.
    PhaseFatal,
    Error,
    Warning,
    Note,
    Help,
    Cancelled,
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

#[derive(Debug)]
pub enum Error { unresolved_name }

#[derive(Debug)]
pub enum Label { primary }

type Span = i32;

#[derive(Debug)]
pub struct ErrorReporter {
    kind: Error,
    labels: Vec<(Span, Label)>
}

impl ErrorReporter {
    pub fn span_label(&mut self, span: Span, label: Label) -> &mut ErrorReporter {
        self.labels.push((span, label));
        self
    }

    pub fn new(error: Error) -> ErrorReporter {
        ErrorReporter { kind: error, labels: vec![] }
    }

    pub fn emit(&mut self) -> Vec<Vec<StyledString>> {
        let mut buffer = TextBuffer2D::new();
        let mut current_line = 0;

        for label in &self.labels {
            let label_text = match label.1 {
                Label::primary => buffer.puts(current_line, 0, "Error", Style::LabelPrimary)
            };
            current_line += 1;
        }

        buffer.render()
    }
}

