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
    FileNameStyle,
    LineAndColumn,
    LineNumber,
    Quotation,
    UnderlinePrimary,
    UnderlineSecondary,
    LabelPrimary,
    LabelSecondary,
    OldSkoolNoteText,
    OldSkoolNote,
    NoStyle,
    Level(Level)
}

#[derive(Debug)]
pub struct StyledString {
    pub text: String,
    pub style: Style,
}

#[derive(Debug)]
pub struct TextBuffer2D {
    text: Vec<Vec<char>>,
    styles: Vec<Vec<Style>>
}

impl TextBuffer2D {
    pub fn new() -> TextBuffer2D {
        TextBuffer2D { text: vec![], styles: vec![] }
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
                        styled_vec.push(StyledString { text: current_text, style: current_style });
                    }
                    current_style = s;
                    current_text = String::new();
                }
                current_text.push(c);
            }
            if !current_text.is_empty() {
                styled_vec.push(StyledString { text: current_text, style: current_style });
            }

            //We're done with the row, push and keep going
            output.push(styled_vec);

            styled_vec = vec![];
        }

        output
    }

    pub fn putc(&mut self, line: usize, col: usize, chr: char, style: Style) {
        while line >= self.text.len() {
            self.text.push(vec![]);
            self.styles.push(vec![]);
        }

        if col < self.text[line].len() {
            self.text[line][col] = chr;
            self.styles[line][col] = style;
        } else {
            let mut i = self.text[line].len();
            while i < col {
                let s = match self.text[0].get(i) {
                    Some(&'\t') => '\t',
                    _ => ' '
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

    pub fn append(&mut self, line: usize, string: &str, style: Style) {
        if line >= self.text.len() {
            self.puts(line, 0, string, style);
        } else {
            let col = self.text[line].len();
            self.puts(line, col, string, style);
        }
    }
}

