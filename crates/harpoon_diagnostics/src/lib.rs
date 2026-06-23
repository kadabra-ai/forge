pub mod diagnostic;
pub mod render;
pub mod source_map;
pub mod span;

pub use diagnostic::{Diagnostic, DiagnosticSink, Label, LabelStyle, Severity, Suggestion};
pub use render::render_diagnostics;
pub use source_map::SourceMap;
pub use span::{FileId, Span};
