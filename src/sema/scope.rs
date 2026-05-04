//! Lexical scope stack for Cypher variable tracking.

use crate::error::Span;
use std::collections::HashMap;

/// What kind of symbol a variable represents.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolKind {
    /// Introduced by MATCH pattern binding.
    PatternBound,
    /// Introduced by UNWIND.
    UnwindBound,
    /// Introduced by WITH … AS alias.
    WithAlias,
    /// Introduced by RETURN … AS alias.
    ReturnAlias,
    /// Introduced by CALL … YIELD … AS alias.
    YieldAlias,
    /// Introduced by FOREACH loop variable.
    ForeachVar,
    /// Introduced by comprehension (list/pattern/ANY/ALL/etc.).
    ComprehensionVar,
    /// A raw variable reference (not a binding).
    Reference,
}

/// A single symbol entry.
#[derive(Debug, Clone)]
pub struct SymbolEntry {
    pub kind: SymbolKind,
    pub span: Span,
}

/// A stack of lexical scopes.
///
/// Each scope is a `HashMap<String, (SymbolKind, Span)>`.
/// Push/pop maps to WITH / RETURN boundaries.
#[derive(Debug, Clone)]
pub struct ScopeStack {
    scopes: Vec<HashMap<String, SymbolEntry>>,
}

impl ScopeStack {
    pub fn new() -> Self {
        Self {
            scopes: vec![HashMap::new()],
        }
    }

    /// Push a new scope (e.g. entering a WITH or RETURN context).
    pub fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    /// Pop the innermost scope.
    pub fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    /// Bind a variable in the current (innermost) scope.
    ///
    /// Returns `Err(first_span)` if the name is already bound in the
    /// current scope (not in outer scopes — shadowing is allowed across scopes).
    pub fn bind(&mut self, name: &str, kind: SymbolKind, span: Span) -> Result<(), Span> {
        let current = self.scopes.last_mut().unwrap();
        if let Some(existing) = current.get(name) {
            return Err(existing.span);
        }
        current.insert(name.to_string(), SymbolEntry { kind, span });
        Ok(())
    }

    /// Resolve a variable name by searching from innermost to outermost scope.
    pub fn resolve(&self, name: &str) -> Option<(&SymbolEntry, usize)> {
        for (depth, scope) in self.scopes.iter().enumerate().rev() {
            if let Some(entry) = scope.get(name) {
                return Some((entry, depth));
            }
        }
        None
    }

    /// Check whether a variable is bound in any scope.
    pub fn is_bound(&self, name: &str) -> bool {
        self.resolve(name).is_some()
    }

    /// Collect all currently bound variable names (for grouping key checks).
    pub fn bound_names(&self) -> Vec<String> {
        let mut names = Vec::new();
        for scope in &self.scopes {
            for name in scope.keys() {
                if !names.contains(name) {
                    names.push(name.clone());
                }
            }
        }
        names
    }
}

impl Default for ScopeStack {
    fn default() -> Self {
        Self::new()
    }
}
