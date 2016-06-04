use term;
use std::fmt;

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

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Style {
    HeaderMsg,
    FileNameStyle,
    LineAndColumn,
    LineNumber,
    Quotation,
    UnderlinePrimary,
    UnderlineSecondary,
    LabelPrimary,
    LabelSecondary,
    OldSchoolNoteText,
    OldSchoolNote,
    NoStyle,
    ErrorCode,
    Level(Level),
}

#[derive(Debug)]
pub struct StyledString {
    pub text: String,
    pub style: Style,
}

#[derive(Debug)]
pub struct StyledBuffer {
    text: Vec<Vec<char>>,
    styles: Vec<Vec<Style>>,
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

    pub fn to_string(self) -> String {
        let output = match self {
            Level::Bug => "error: internal compiler error",
            Level::Fatal | Level::PhaseFatal | Level::Error => "error",
            Level::Warning => "warning",
            Level::Note => "note",
            Level::Help => "help",
            Level::Cancelled => panic!("Shouldn't call on cancelled error"),
        };
        String::from(output)
    }
}

impl StyledBuffer {
    pub fn new() -> StyledBuffer {
        StyledBuffer {
            text: vec![],
            styles: vec![],
        }
    }

    pub fn render(&self) -> Vec<Vec<StyledString>> {
        let mut output: Vec<Vec<StyledString>> = vec![];
        let mut styled_vec: Vec<StyledString> = vec![];

        for (row, row_style) in self.text.iter().zip(&self.styles) {
            let mut current_style = Style::NoStyle;
            let mut current_text = String::new();

            for (&c, &s) in row.iter().zip(row_style) {
                if s != current_style {
                    if !current_text.is_empty() {
                        styled_vec.push(StyledString {
                            text: current_text,
                            style: current_style,
                        });
                    }
                    current_style = s;
                    current_text = String::new();
                }
                current_text.push(c);
            }
            if !current_text.is_empty() {
                styled_vec.push(StyledString {
                    text: current_text,
                    style: current_style,
                });
            }

            // We're done with the row, push and keep going
            output.push(styled_vec);

            styled_vec = vec![];
        }

        output
    }

    fn ensure_lines(&mut self, line: usize) {
        while line >= self.text.len() {
            self.text.push(vec![]);
            self.styles.push(vec![]);
        }
    }

    pub fn putc(&mut self, line: usize, col: usize, chr: char, style: Style) {
        self.ensure_lines(line);
        if col < self.text[line].len() {
            self.text[line][col] = chr;
            self.styles[line][col] = style;
        } else {
            let mut i = self.text[line].len();
            while i < col {
                let s = match self.text[0].get(i) {
                    Some(&'\t') => '\t',
                    _ => ' ',
                };
                self.text[line].push(s);
                self.styles[line].push(Style::NoStyle);
                i += 1;
            }
            self.text[line].push(chr);
            self.styles[line].push(style);
        }
    }

    pub fn puts(&mut self, line: usize, col: usize, string: &str, style: Style) {
        let mut n = col;
        for c in string.chars() {
            self.putc(line, n, c, style);
            n += 1;
        }
    }

    pub fn set_style(&mut self, line: usize, col: usize, style: Style) {
        if self.styles.len() > line && self.styles[line].len() > col {
            self.styles[line][col] = style;
        }
    }

    pub fn prepend(&mut self, line: usize, string: &str, style: Style) {
        self.ensure_lines(line);
        let string_len = string.len();

        // Push the old content over to make room for new content
        for i in 0..string_len {
            self.styles[line].insert(0, Style::NoStyle);
            self.text[line].insert(0, ' ');
        }

        self.puts(line, 0, string, style);
    }

    pub fn append(&mut self, line: usize, string: &str, style: Style) {
        if line >= self.text.len() {
            self.puts(line, 0, string, style);
        } else {
            let col = self.text[line].len();
            self.puts(line, col, string, style);
        }
    }

    pub fn num_lines(&self) -> usize {
        self.text.len()
    }
}
