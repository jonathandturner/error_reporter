use std::fmt;
use std::rc::Rc;

use term;

use styled_buffer::*;
use codemap::{self, Span, CharPos, FileMap, SpanLabel};

pub struct CompilerMessage {
    pub level: Level,
    pub primary_span: Span,
    pub primary_msg: String,
    pub span_labels: Vec<SpanLabel>,
    pub notes: Vec<String>,
    pub error_code: Option<String>,
    pub cm: Rc<codemap::CodeMap>,
}

impl CompilerMessage {
    pub fn span_label(&mut self, span: Span, label: Option<String>) -> &mut CompilerMessage {
        self.span_labels.push(SpanLabel {
            span: span,
            is_primary: (span == self.primary_span),
            label: label,
        });
        self
    }

    pub fn note(&mut self, note: String) -> &mut CompilerMessage {
        self.notes.push(note);
        self
    }

    pub fn new(level: Level,
               msg: String,
               primary_span: Span,
               error_code: Option<String>,
               cm: Rc<codemap::CodeMap>)
               -> CompilerMessage {

        CompilerMessage {
            level: level,
            primary_span: primary_span,
            primary_msg: msg,
            error_code: error_code,
            span_labels: vec![],
            notes: vec![],
            cm: cm,
        }
    }
}