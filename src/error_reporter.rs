use std::fmt;
use std::rc::Rc;

use text_buffer_2d::*;
use term;

use codemap::{self, Span};

#[derive(Clone, Debug)]
struct SpanLabel {
    /// The span we are going to include in the final snippet.
    pub span: Span,

    /// Is this a primary span? This is the "locus" of the message,
    /// and is indicated with a `^^^^` underline, versus `----`.
    pub is_primary: bool,

    /// What label should we attach to this span (if any)?
    pub label: Option<String>,
}

pub struct ErrorReporter {
    level: Level,
    primary_span: Span,
    primary_msg: String,
    span_labels: Vec<SpanLabel>,
    cm: Rc<codemap::CodeMap>,
}

impl ErrorReporter {
    pub fn span_label(&mut self, span: Span, label: Option<String>) -> &mut ErrorReporter {
        self.span_labels.push(SpanLabel {
            span: span,
            is_primary: (span == self.primary_span),
            label: label,
        });
        self
    }

    pub fn new(level: Level,
               msg: String,
               primary_span: Span,
               cm: Rc<codemap::CodeMap>)
               -> ErrorReporter {

        ErrorReporter {
            level: level,
            primary_span: primary_span,
            primary_msg: msg,
            span_labels: vec![],
            cm: cm,
        }
    }

    fn render_header(&mut self, buffer: &mut TextBuffer2D) {
        // Header line 1: error: the error message [ENUM]
        buffer.append(0, &self.level.to_string(), Style::Level(self.level));
        buffer.append(0, ": ", Style::HeaderMsg);
        buffer.append(0, &self.primary_msg.clone(), Style::HeaderMsg);

        // Header line 2: filename:line:col (we'll write the --> later)
        buffer.append(1, &self.cm.span_to_string(self.primary_span), Style::LineAndColumn);
    }

    pub fn render(&mut self) -> Vec<Vec<StyledString>> {
        let mut buffer = TextBuffer2D::new();

        self.render_header(&mut buffer);
        let mut current_line = 2;

        for span_label in &self.span_labels {
            if span_label.is_primary {
                buffer.puts(current_line,
                            0,
                            &span_label.label.clone().unwrap(),
                            Style::LabelPrimary);
                buffer.puts(current_line,
                            20,
                            &self.cm.span_to_string(span_label.span),
                            Style::UnderlinePrimary);
            } else {
                buffer.puts(current_line,
                            0,
                            &span_label.label.clone().unwrap(),
                            Style::LabelSecondary);
                buffer.puts(current_line,
                            20,
                            &self.cm.span_to_string(span_label.span),
                            Style::UnderlineSecondary);
            }
            current_line += 1;
        }

        buffer.prepend(1, "--> ", Style::LineNumber);

        buffer.render()
    }
}
