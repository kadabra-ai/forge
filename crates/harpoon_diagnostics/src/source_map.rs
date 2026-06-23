use crate::span::FileId;

pub struct SourceFile {
    name: String,
    source: String,
    line_starts: Vec<u32>,
}

impl SourceFile {
    fn new(name: String, source: String) -> Self {
        let line_starts = std::iter::once(0)
            .chain(source.match_indices('\n').map(|(i, _)| (i + 1) as u32))
            .collect();
        Self {
            name,
            source,
            line_starts,
        }
    }
}

pub struct SourceMap {
    files: Vec<SourceFile>,
}

impl SourceMap {
    pub fn new() -> Self {
        Self { files: Vec::new() }
    }

    pub fn add_file(&mut self, name: String, source: String) -> FileId {
        let id = FileId(self.files.len() as u32);
        self.files.push(SourceFile::new(name, source));
        id
    }

    pub fn file_name(&self, id: FileId) -> &str {
        &self.files[id.0 as usize].name
    }

    pub fn file_source(&self, id: FileId) -> &str {
        &self.files[id.0 as usize].source
    }

    pub fn line_starts(&self, id: FileId) -> &[u32] {
        &self.files[id.0 as usize].line_starts
    }

    /// Returns (line, column), both 0-indexed.
    pub fn line_col(&self, id: FileId, offset: u32) -> (usize, usize) {
        let starts = &self.files[id.0 as usize].line_starts;
        let line = starts.partition_point(|&s| s <= offset) - 1;
        let col = (offset - starts[line]) as usize;
        (line, col)
    }

    pub fn file_count(&self) -> usize {
        self.files.len()
    }
}

impl Default for SourceMap {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_file_and_retrieve() {
        let mut sm = SourceMap::new();
        let id = sm.add_file("test.kerml".into(), "package Foo {}".into());
        assert_eq!(sm.file_name(id), "test.kerml");
        assert_eq!(sm.file_source(id), "package Foo {}");
    }

    #[test]
    fn line_col_from_offset() {
        let mut sm = SourceMap::new();
        let id = sm.add_file("test.kerml".into(), "line1\nline2\nline3".into());
        // 'l' of "line2" is at byte offset 6
        let (line, col) = sm.line_col(id, 6);
        assert_eq!(line, 1); // 0-indexed lines
        assert_eq!(col, 0);
    }
}
