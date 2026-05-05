//! Optional `miette::Diagnostic` implementation for [`CypherError`].
//!
//! Compiled only when the `miette` feature is enabled. Provides a
//! [`miette::Report`] conversion and the full `Diagnostic` trait
//! implementation, allowing `CypherError` to be used with miette's
//! `GraphicalReportHandler` and `NarratableReportHandler`.

use crate::error::CypherError;
use miette::{Diagnostic, LabeledSpan, NamedSource, SourceSpan};

impl Diagnostic for CypherError {
    fn code<'a>(&'a self) -> Option<Box<dyn std::fmt::Display + 'a>> {
        Some(Box::new(format!("cypher::{:?}", self.kind)))
    }

    fn severity(&self) -> Option<miette::Severity> {
        Some(miette::Severity::Error)
    }

    fn help<'a>(&'a self) -> Option<Box<dyn std::fmt::Display + 'a>> {
        self.notes
            .iter()
            .find(|n| matches!(n.level, crate::error::NoteLevel::Help))
            .map(|n| Box::new(n.message.to_string()) as Box<dyn std::fmt::Display>)
    }

    fn labels(&self) -> Option<Box<dyn Iterator<Item = LabeledSpan> + '_>> {
        let mut labels = Vec::new();
        labels.push(LabeledSpan::new_with_span(
            Some(format!("{}", self.kind)),
            SourceSpan::from(self.span.start..self.span.end),
        ));
        for note in &self.notes {
            labels.push(LabeledSpan::new_with_span(
                Some(note.message.to_string()),
                SourceSpan::from(note.span.start..note.span.end),
            ));
        }
        Some(Box::new(labels.into_iter()))
    }
}

impl CypherError {
    /// Convert this error into a [`miette::Report`].
    ///
    /// If the error carries a source string, the report is attached to a
    /// [`NamedSource`] so that miette's renderers can display the
    /// annotated source snippet.
    pub fn to_report(&self) -> miette::Report {
        if let Some(ref src) = self.source {
            let label = self.source_label.as_deref().unwrap_or("query");
            miette::Report::new(self.clone()).with_source_code(NamedSource::new(label, src.clone()))
        } else {
            miette::Report::new(self.clone())
        }
    }
}
