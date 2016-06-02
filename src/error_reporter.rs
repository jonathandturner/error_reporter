use std::fmt;
use std::rc::Rc;

use styled_buffer::*;
use term;

use codemap::{self, Span, CharPos, FileMap};

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

struct FileWithAnnotatedLines {
    file: Rc<FileMap>,
    lines: Vec<Line>,
}

#[derive(Clone, Debug, PartialOrd, Ord, PartialEq, Eq)]
struct Line {
    // Use a span here as a way to acquire this line later
    line_number: usize,
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

fn check_old_school() -> bool {
    false
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

    pub fn render(&mut self) -> Vec<Vec<StyledString>> {
        // Create our styled buffer that we'll use to render the whole error message
        let mut buffer = StyledBuffer::new();

        // Header line
        // eg) error: type mismatch [E123]
        //TODO: still needs error number
        buffer.append(0, &self.level.to_string(), Style::Level(self.level));
        buffer.append(0, ": ", Style::HeaderMsg);
        buffer.append(0, &self.primary_msg.clone(), Style::HeaderMsg);

        // Preprocess all the annotations so that they are grouped by file and by line number
        // This helps us quickly iterate over the whole message (including secondary file spans)
        let mut annotated_files = self.preprocess_annotations();

        // Make sure our primary file comes first
        let primary_lo = self.cm.lookup_char_pos(self.primary_span.lo);
        if let Ok(pos) =
               annotated_files.binary_search_by(|x| x.file.name.cmp(&primary_lo.file.name)) {
            annotated_files.swap(0, pos);
        }

        // Print out the annotate source lines that correspond with the error
        for annotated_file in annotated_files {
            // figure out the largest line number so we can align the line number column
            let highest_line = annotated_file.lines.last().unwrap().line_number;
            let len_of_largest_line = highest_line.to_string().len();

            // remember where we are in the output buffer for easy reference
            let mut buffer_msg_line_offset = buffer.num_lines();

            // print out the span location and spacer before we print the annotated source
            // to do this, we need to know if this span will be primary
            let is_primary = primary_lo.file.name == annotated_file.file.name;
            if is_primary {
                buffer.prepend(buffer_msg_line_offset, "--> ", Style::LineNumber);
                let loc = self.cm.lookup_char_pos(self.primary_span.lo);
                buffer.append(buffer_msg_line_offset,
                            &format!("{}:{}:{}", loc.file.name, loc.line, loc.col.0),
                            Style::LineAndColumn);
            }
            else {
                buffer.prepend(buffer_msg_line_offset, "::: ", Style::LineNumber);
                buffer.append(buffer_msg_line_offset,
                            &annotated_file.file.name,
                            Style::LineAndColumn);
            }
            for i in 0..len_of_largest_line {
                buffer.prepend(buffer_msg_line_offset, " ", Style::NoStyle);
            }

            // Put in the spacer between the location and annotated source
            buffer.puts(buffer_msg_line_offset + 1,
                        len_of_largest_line + 1,
                        "|>",
                        Style::LineNumber);

            // Next, output the annotate source for this file
            for line in &annotated_file.lines {
                self.render_source_line(&mut buffer,
                                        annotated_file.file.clone(),
                                        &line,
                                        3 + len_of_largest_line);
            }
        }

        //final step: take our styled buffer and render it
        buffer.render()
    }

    fn preprocess_annotations(&mut self) -> Vec<FileWithAnnotatedLines> {
        fn add_annotation_to_file(file_vec: &mut Vec<FileWithAnnotatedLines>,
                                  file: Rc<FileMap>,
                                  line_number: usize,
                                  ann: Annotation) {

            for slot in file_vec.iter_mut() {
                // Look through each of our files for the one we're adding to
                if slot.file.name == file.name {
                    // See if we already have a line for it
                    for line_slot in &mut slot.lines {
                        if line_slot.line_number == line_number {
                            line_slot.annotations.push(ann);
                            return;
                        }
                    }
                    // We don't have a line yet, create one
                    slot.lines.push(Line {
                        line_number: line_number,
                        annotations: vec![ann],
                    });
                    slot.lines.sort();
                    return;
                }
            }
            // This is the first time we're seeing the file
            file_vec.push(FileWithAnnotatedLines {
                file: file,
                lines: vec![Line {
                                line_number: line_number,
                                annotations: vec![ann],
                            }],
            });
        }

        let mut output = vec![];

        for span_label in &self.span_labels {
            let lo = self.cm.lookup_char_pos(span_label.span.lo);
            let hi = self.cm.lookup_char_pos(span_label.span.hi);

            // If the span is multi-line, simplify down to the span of one character
            let (start_col, mut end_col, is_minimized) = if lo.line != hi.line {
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

            add_annotation_to_file(&mut output,
                                   lo.file,
                                   lo.line,
                                   Annotation {
                                       start_col: lo.col.0,
                                       end_col: hi.col.0,
                                       is_primary: span_label.is_primary,
                                       is_minimized: is_minimized,
                                       label: span_label.label.clone(),
                                   });
        }
        output
    }

    fn render_source_line(&mut self,
                          buffer: &mut StyledBuffer,
                          file: Rc<FileMap>,
                          line: &Line,
                          width_offset: usize) {
        let source_string = file.get_line(line.line_number - 1)
            .unwrap_or("");

        let line_offset = buffer.num_lines();

        // First create the source line we will highlight.
        buffer.puts(line_offset, width_offset, &source_string, Style::Quotation);
        buffer.puts(line_offset,
                    0,
                    &(line.line_number.to_string()),
                    Style::LineNumber);

        buffer.puts(line_offset, width_offset - 2, "|>", Style::LineNumber);

        if line.annotations.is_empty() {
            return;
        }

        // We want to display like this:
        //
        //      vec.push(vec.pop().unwrap());
        //      ---      ^^^               _ previous borrow ends here
        //      |        |
        //      |        error occurs here
        //      previous borrow of `vec` occurs here
        //
        // But there are some weird edge cases to be aware of:
        //
        //      vec.push(vec.pop().unwrap());
        //      --------                    - previous borrow ends here
        //      ||
        //      |this makes no sense
        //      previous borrow of `vec` occurs here
        //
        // For this reason, we group the lines into "highlight lines"
        // and "annotations lines", where the highlight lines have the `~`.

        // let mut highlight_line = Self::whitespace(&source_string);
        let old_school = check_old_school();

        // Sort the annotations by (start, end col)
        let mut annotations = line.annotations.clone();
        annotations.sort();

        // Next, create the highlight line.
        for annotation in &annotations {
            if old_school {
                for p in annotation.start_col..annotation.end_col {
                    if p == annotation.start_col {
                        buffer.putc(line_offset + 1,
                                    width_offset + p,
                                    '^',
                                    if annotation.is_primary {
                                        Style::UnderlinePrimary
                                    } else {
                                        Style::OldSchoolNote
                                    });
                    } else {
                        buffer.putc(line_offset + 1,
                                    width_offset + p,
                                    '~',
                                    if annotation.is_primary {
                                        Style::UnderlinePrimary
                                    } else {
                                        Style::OldSchoolNote
                                    });
                    }
                }
            } else {
                for p in annotation.start_col..annotation.end_col {
                    if annotation.is_primary {
                        buffer.putc(line_offset + 1,
                                    width_offset + p,
                                    '^',
                                    Style::UnderlinePrimary);
                        if !annotation.is_minimized {
                            buffer.set_style(line_offset,
                                             width_offset + p,
                                             Style::UnderlinePrimary);
                        }
                    } else {
                        buffer.putc(line_offset + 1,
                                    width_offset + p,
                                    '-',
                                    Style::UnderlineSecondary);
                        if !annotation.is_minimized {
                            buffer.set_style(line_offset,
                                             width_offset + p,
                                             Style::UnderlineSecondary);
                        }
                    }
                }
            }
        }
        buffer.puts(line_offset + 1, width_offset - 2, "|>", Style::LineNumber);

        // Now we are going to write labels in. To start, we'll exclude
        // the annotations with no labels.
        let (labeled_annotations, unlabeled_annotations): (Vec<_>, _) = annotations.into_iter()
            .partition(|a| a.label.is_some());

        // If there are no annotations that need text, we're done.
        if labeled_annotations.is_empty() {
            return;
        }
        if old_school {
            return;
        }

        // Now add the text labels. We try, when possible, to stick the rightmost
        // annotation at the end of the highlight line:
        //
        //      vec.push(vec.pop().unwrap());
        //      ---      ---               - previous borrow ends here
        //
        // But sometimes that's not possible because one of the other
        // annotations overlaps it. For example, from the test
        // `span_overlap_label`, we have the following annotations
        // (written on distinct lines for clarity):
        //
        //      fn foo(x: u32) {
        //      --------------
        //             -
        //
        // In this case, we can't stick the rightmost-most label on
        // the highlight line, or we would get:
        //
        //      fn foo(x: u32) {
        //      -------- x_span
        //      |
        //      fn_span
        //
        // which is totally weird. Instead we want:
        //
        //      fn foo(x: u32) {
        //      --------------
        //      |      |
        //      |      x_span
        //      fn_span
        //
        // which is...less weird, at least. In fact, in general, if
        // the rightmost span overlaps with any other span, we should
        // use the "hang below" version, so we can at least make it
        // clear where the span *starts*.
        let mut labeled_annotations = &labeled_annotations[..];
        match labeled_annotations.split_last().unwrap() {
            (last, previous) => {
                if previous.iter()
                    .chain(&unlabeled_annotations)
                    .all(|a| !overlaps(a, last)) {
                    // append the label afterwards; we keep it in a separate
                    // string
                    let highlight_label: String = format!(" {}", last.label.as_ref().unwrap());
                    if last.is_primary {
                        buffer.append(line_offset + 1, &highlight_label, Style::LabelPrimary);
                    } else {
                        buffer.append(line_offset + 1, &highlight_label, Style::LabelSecondary);
                    }
                    labeled_annotations = previous;
                }
            }
        }

        // If that's the last annotation, we're done
        if labeled_annotations.is_empty() {
            return;
        }

        for (index, annotation) in labeled_annotations.iter().enumerate() {
            // Leave:
            // - 1 extra line
            // - One line for each thing that comes after
            let comes_after = labeled_annotations.len() - index - 1;
            let blank_lines = 3 + comes_after;

            // For each blank line, draw a `|` at our column. The
            // text ought to be long enough for this.
            for index in 2..blank_lines {
                if annotation.is_primary {
                    buffer.putc(line_offset + index,
                                width_offset + annotation.start_col,
                                '|',
                                Style::UnderlinePrimary);
                } else {
                    buffer.putc(line_offset + index,
                                width_offset + annotation.start_col,
                                '|',
                                Style::UnderlineSecondary);
                }
                buffer.puts(line_offset + index,
                            width_offset - 2,
                            "|>",
                            Style::LineNumber);
            }

            if annotation.is_primary {
                buffer.puts(line_offset + blank_lines,
                            width_offset + annotation.start_col,
                            annotation.label.as_ref().unwrap(),
                            Style::LabelPrimary);
            } else {
                buffer.puts(line_offset + blank_lines,
                            width_offset + annotation.start_col,
                            annotation.label.as_ref().unwrap(),
                            Style::LabelSecondary);
            }
            buffer.puts(line_offset + blank_lines,
                        width_offset - 2,
                        "|>",
                        Style::LineNumber);
        }
    }
}

fn overlaps(a1: &Annotation, a2: &Annotation) -> bool {
    (a2.start_col..a2.end_col).contains(a1.start_col) ||
    (a1.start_col..a1.end_col).contains(a2.start_col)
}
