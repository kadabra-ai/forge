use crate::diagnostic::{Diagnostic, LabelStyle, Severity};
use crate::source_map::SourceMap;
use codespan_reporting::diagnostic as cs;
use codespan_reporting::files::SimpleFiles;
use codespan_reporting::term;
use codespan_reporting::term::termcolor::{ColorChoice, StandardStream};

pub fn render_diagnostics(source_map: &SourceMap, diagnostics: &[Diagnostic]) {
    let mut files = SimpleFiles::new();
    for i in 0..source_map.file_count() {
        let file_id = crate::span::FileId(i as u32);
        files.add(
            source_map.file_name(file_id),
            source_map.file_source(file_id),
        );
    }

    let writer = StandardStream::stderr(ColorChoice::Auto);
    let config = term::Config::default();

    for diag in diagnostics {
        let severity = match diag.severity {
            Severity::Error => cs::Severity::Error,
            Severity::Warning => cs::Severity::Warning,
            Severity::Info => cs::Severity::Note,
            Severity::Hint => cs::Severity::Help,
        };

        let mut cs_diag = cs::Diagnostic::new(severity).with_message(&diag.message);

        if let Some(code) = &diag.code {
            cs_diag = cs_diag.with_code(code);
        }

        let labels: Vec<_> = diag
            .labels
            .iter()
            .map(|l| {
                let style = match l.style {
                    LabelStyle::Primary => cs::LabelStyle::Primary,
                    LabelStyle::Secondary => cs::LabelStyle::Secondary,
                };
                cs::Label::new(
                    style,
                    l.span.file.0 as usize,
                    (l.span.start as usize)..(l.span.end as usize),
                )
                .with_message(&l.message)
            })
            .collect();
        cs_diag = cs_diag.with_labels(labels);

        let notes: Vec<String> = diag.notes.iter().cloned().collect();
        cs_diag = cs_diag.with_notes(notes);

        let _ = term::emit_to_write_style(&mut writer.lock(), &config, &files, &cs_diag);
    }
}
