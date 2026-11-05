//! Files, byte spans, and line/column only when a person needs one.

/// A file in a [`SourceMap`]. Small and copyable, so a span costs 12 bytes
/// and a re-parse of one file never touches another file's ids.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FileId(pub u32);

/// A byte range `start..end` in one file.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Span {
    pub file: FileId,
    pub start: u32,
    pub end: u32,
}

impl Span {
    pub fn new(file: FileId, start: u32, end: u32) -> Self {
        debug_assert!(start <= end);
        Span { file, start, end }
    }

    /// The smallest span covering both. Both must be in the same file.
    pub fn to(self, other: Span) -> Span {
        debug_assert_eq!(self.file, other.file);
        Span { file: self.file, start: self.start.min(other.start), end: self.end.max(other.end) }
    }

    pub fn len(self) -> u32 {
        self.end - self.start
    }

    pub fn is_empty(self) -> bool {
        self.start == self.end
    }
}

/// Where each line starts, so a byte offset becomes a line and column
/// only when a diagnostic is shown.
#[derive(Clone, Debug)]
pub struct LineIndex {
    line_starts: Vec<u32>,
}

impl LineIndex {
    pub fn new(text: &str) -> Self {
        let mut line_starts = vec![0];
        for (i, b) in text.bytes().enumerate() {
            if b == b'\n' {
                line_starts.push(i as u32 + 1);
            }
        }
        LineIndex { line_starts }
    }

    /// 1-based line and column; the column counts characters, not bytes.
    pub fn line_col(&self, text: &str, offset: u32) -> (u32, u32) {
        let line = match self.line_starts.binary_search(&offset) {
            Ok(i) => i,
            Err(i) => i - 1,
        };
        let start = self.line_starts[line] as usize;
        let end = (offset as usize).min(text.len());
        let col = text.get(start..end).map_or(end - start, |s| s.chars().count());
        (line as u32 + 1, col as u32 + 1)
    }
}

/// One file's name, text and line index.
#[derive(Clone, Debug)]
pub struct SourceFile {
    pub name: String,
    pub text: String,
    pub lines: LineIndex,
}

/// Every file of a compilation, by [`FileId`].
#[derive(Clone, Debug, Default)]
pub struct SourceMap {
    files: Vec<SourceFile>,
}

impl SourceMap {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, name: impl Into<String>, text: impl Into<String>) -> FileId {
        let text = text.into();
        let lines = LineIndex::new(&text);
        self.files.push(SourceFile { name: name.into(), text, lines });
        FileId(self.files.len() as u32 - 1)
    }

    pub fn get(&self, id: FileId) -> &SourceFile {
        &self.files[id.0 as usize]
    }

    /// 1-based line and column of a byte offset in a file.
    pub fn line_col(&self, id: FileId, offset: u32) -> (u32, u32) {
        let f = self.get(id);
        f.lines.line_col(&f.text, offset)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_and_column_count_characters() {
        let text = "ab\nçd\nx";
        let idx = LineIndex::new(text);
        assert_eq!(idx.line_col(text, 0), (1, 1));
        assert_eq!(idx.line_col(text, 3), (2, 1));
        // 'ç' is two bytes; 'd' is the second character.
        assert_eq!(idx.line_col(text, 5), (2, 2));
        assert_eq!(idx.line_col(text, 7), (3, 1));
    }
}
