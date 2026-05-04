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
    /// Barriers mark scope boundaries (e.g. after WITH). Resolution only
    /// searches scopes from the topmost barrier onward.
    barriers: Vec<usize>,
}

impl ScopeStack {
    pub fn new() -> Self {
        Self {
            scopes: vec![HashMap::new()],
            barriers: Vec::new(),
        }
    }

    /// Push a new scope (e.g. entering a WITH or RETURN context).
    pub fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    /// Pop the innermost scope.
    ///
    /// Does nothing if only the base scope remains (to prevent stack underflow).
    /// A `debug_assert!` fires in tests if this guard is triggered unexpectedly.
    pub fn pop_scope(&mut self) {
        debug_assert!(self.scopes.len() > 1, "attempted to pop the base scope");
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
        // Remove any barriers that now point beyond the stack.
        while let Some(&barrier) = self.barriers.last() {
            if barrier >= self.scopes.len() {
                self.barriers.pop();
            } else {
                break;
            }
        }
    }

    /// Push a barrier that hides all scopes below the current innermost one.
    ///
    /// Used after a WITH clause so that only variables projected by the WITH
    /// are visible to subsequent clauses.
    pub fn push_barrier(&mut self) {
        self.barriers.push(self.scopes.len().saturating_sub(1));
    }

    /// Pop the most recent barrier.
    pub fn pop_barrier(&mut self) {
        self.barriers.pop();
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

    /// Resolve a variable name by searching from innermost to outermost scope,
    /// stopping at the most recent barrier.
    pub fn resolve(&self, name: &str) -> Option<(&SymbolEntry, usize)> {
        let start = self.barriers.last().copied().unwrap_or(0);
        for (depth, scope) in self.scopes.iter().enumerate().rev() {
            if depth < start {
                break;
            }
            if let Some(entry) = scope.get(name) {
                return Some((entry, depth));
            }
        }
        None
    }

    /// Check whether a variable is bound in any visible scope.
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

#[cfg(test)]
mod tests {
    use super::{ScopeStack, SymbolKind};
    use crate::error::Span;

    #[test]
    fn pop_scope_preserves_base_scope() {
        let mut scopes = ScopeStack::new();
        scopes.push_scope();

        scopes.pop_scope();

        assert!(
            scopes
                .bind("n", SymbolKind::PatternBound, Span::new(0, 1))
                .is_ok()
        );
        assert!(scopes.is_bound("n"));
    }
}
