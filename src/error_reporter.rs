use std::fmt;
use std::rc::Rc;

use text_buffer_2d::*;
use term;

use codemap::{self, Span, CharPos};

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

#[derive(Clone, Debug)]
struct Line {
    //Use a span here as a way to acquire this line later
    span: Span,
    annotations: Vec<Annotation>,
}

#[derive(Clone, Debug, PartialOrd, Ord, PartialEq, Eq)]
struct Annotation {
    /// Start column, 0-based indexing -- counting *characters*, not
    /// utf-8 bytes. Note that it is important that this field goes
    /// first, so that when we sort, we sort orderings by start
    /// column.
    start_col: usize,

    /// End column within the line (exclusive)
    end_col: usize,

    /// Is this annotation derived from primary span
    is_primary: bool,

    /// Is this a large span minimized down to a smaller span
    is_minimized: bool,

    /// Optional label to display adjacent to the annotation.
    label: Option<String>,
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

    fn render_source_lines(&mut self, buffer: &mut TextBuffer2D) {
        use std::collections::HashMap;

        let mut file_map: HashMap<String, HashMap<usize, Line>> = HashMap::new();

        //Convert our labels+spans into the annotations we'll be displaying to the user.
        //To do this, we'll build up a HashMap for each file we need to display
        //in the hashmap, we'll build up our annotated source lines
        for span_label in &self.span_labels {
            let filename = self.cm.span_to_filename(span_label.span);
            let mut line_map = file_map.entry(filename).or_insert(HashMap::new());

            let lo = self.cm.lookup_char_pos(span_label.span.lo);
            let hi = self.cm.lookup_char_pos(span_label.span.hi);
            // If the span is multi-line, simplify down to the span of one character
            let (start_col, mut end_col, is_minimized) =
                if lo.line != hi.line {
                    (lo.col, CharPos(lo.col.0 + 1), true)
                } else {
                    (lo.col, hi.col, false)
                };

            // Watch out for "empty spans". If we get a span like 6..6, we
            // want to just display a `^` at 6, so convert that to
            // 6..7. This is degenerate input, but it's best to degrade
            // gracefully -- and the parser likes to supply a span like
            // that for EOF, in particular.
            if start_col == end_col {
                end_col.0 += 1;
            }

            let line_entry = (*line_map).entry(lo.line).or_insert(
                Line { span: span_label.span.clone(), annotations: vec![] });

            (*line_entry).annotations.push(Annotation { start_col: lo.col.0, end_col: hi.col.0,
                is_primary: span_label.is_primary, is_minimized: is_minimized,
                label: span_label.label.clone()})
        }

        //Now that we have lines with their annotations, we can sort the lines we know about,
        //walk through them, and begin rendering the source block in the error
        //TODO: we should print the primary file first
        for fname in file_map.keys() {
            let mut all_lines: Vec<&usize> = file_map[fname].keys().collect();
            all_lines.sort();
            for line in all_lines {
                self.render_source_line(buffer, &file_map[fname][line]);
            }
        }
        //println!("{:?}", file_map);
    }

    fn render_source_line(&mut self, buffer: &mut TextBuffer2D, line: &Line) {
        println!("{:?}", line);
    }

    pub fn render(&mut self) -> Vec<Vec<StyledString>> {
        let mut buffer = TextBuffer2D::new();

        self.render_header(&mut buffer);
        self.render_source_lines(&mut buffer);

        /*
        let mut current_line = 2;
        println!("{:?}", self.cm.lookup_char_pos(self.primary_span.lo));
        let result = self.cm.span_to_lines(self.primary_span).unwrap();
        for line in result.lines {
            println!("{:?}", result.file.get_line(line.line_index));
        }
        */

        buffer.render()
    }
}
