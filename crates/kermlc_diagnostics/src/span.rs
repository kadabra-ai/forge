#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FileId(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Span {
    pub file: FileId,
    pub start: u32,
    pub end: u32,
}

impl Span {
    pub fn new(file: FileId, start: u32, end: u32) -> Self {
        Self { file, start, end }
    }

    pub fn dummy() -> Self {
        Self {
            file: FileId(u32::MAX),
            start: 0,
            end: 0,
        }
    }

    pub fn len(&self) -> u32 {
        self.end - self.start
    }

    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }

    pub fn merge(self, other: Span) -> Span {
        debug_assert_eq!(self.file, other.file);
        Span {
            file: self.file,
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn span_creation() {
        let span = Span::new(FileId(0), 10, 25);
        assert_eq!(span.file, FileId(0));
        assert_eq!(span.start, 10);
        assert_eq!(span.end, 25);
    }

    #[test]
    fn span_len() {
        let span = Span::new(FileId(0), 10, 25);
        assert_eq!(span.len(), 15);
    }

    #[test]
    fn dummy_span() {
        let span = Span::dummy();
        assert_eq!(span.file, FileId(u32::MAX));
    }
}
