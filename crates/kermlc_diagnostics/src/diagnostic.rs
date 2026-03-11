use crate::span::Span;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Info,
    Hint,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LabelStyle {
    Primary,
    Secondary,
}

#[derive(Clone, Debug)]
pub struct Label {
    pub span: Span,
    pub message: String,
    pub style: LabelStyle,
}

impl Label {
    pub fn primary(span: Span, message: impl Into<String>) -> Self {
        Self {
            span,
            message: message.into(),
            style: LabelStyle::Primary,
        }
    }

    pub fn secondary(span: Span, message: impl Into<String>) -> Self {
        Self {
            span,
            message: message.into(),
            style: LabelStyle::Secondary,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Suggestion {
    pub message: String,
    pub span: Span,
    pub replacement: String,
}

#[derive(Clone, Debug)]
pub struct Diagnostic {
    pub severity: Severity,
    pub message: String,
    pub code: Option<String>,
    pub labels: Vec<Label>,
    pub notes: Vec<String>,
    pub suggestions: Vec<Suggestion>,
}

impl Diagnostic {
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Error,
            message: message.into(),
            code: None,
            labels: Vec::new(),
            notes: Vec::new(),
            suggestions: Vec::new(),
        }
    }

    pub fn warning(message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Warning,
            message: message.into(),
            code: None,
            labels: Vec::new(),
            notes: Vec::new(),
            suggestions: Vec::new(),
        }
    }

    pub fn with_code(mut self, code: impl Into<String>) -> Self {
        self.code = Some(code.into());
        self
    }

    pub fn with_label(mut self, label: Label) -> Self {
        self.labels.push(label);
        self
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    pub fn with_suggestion(mut self, suggestion: Suggestion) -> Self {
        self.suggestions.push(suggestion);
        self
    }
}

/// Collects diagnostics across compilation phases.
#[derive(Default)]
pub struct DiagnosticSink {
    diagnostics: Vec<Diagnostic>,
}

impl DiagnosticSink {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn emit(&mut self, diag: Diagnostic) {
        self.diagnostics.push(diag);
    }

    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| d.severity == Severity::Error)
    }

    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    pub fn into_diagnostics(self) -> Vec<Diagnostic> {
        self.diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::span::{FileId, Span};

    #[test]
    fn create_error_diagnostic() {
        let diag = Diagnostic::error("type not found")
            .with_code("E0001")
            .with_label(Label::primary(
                Span::new(FileId(0), 10, 20),
                "not found here",
            ));
        assert_eq!(diag.severity, Severity::Error);
        assert_eq!(diag.message, "type not found");
        assert_eq!(diag.labels.len(), 1);
    }
}
