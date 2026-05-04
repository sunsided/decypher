use cypher::error::Span;
use cypher::sema::{ScopeStack, SymbolKind};

fn dummy_span() -> Span {
    Span::new(0, 1)
}

#[test]
fn bind_after_push_pop_works() {
    let mut stack = ScopeStack::new();
    stack.bind("outer", SymbolKind::PatternBound, dummy_span());
    stack.push_scope();
    stack.bind("inner", SymbolKind::WithAlias, dummy_span());
    stack.pop_scope();
    // After popping, "outer" should still be resolvable
    assert!(stack.is_bound("outer"));
    // "inner" should not be visible anymore
    assert!(!stack.is_bound("inner"));
}

#[test]
#[should_panic(expected = "attempted to pop the base scope")]
fn pop_base_scope_panics_in_debug() {
    let mut stack = ScopeStack::new();
    stack.pop_scope();
}

#[test]
#[should_panic(expected = "attempted to pop the base scope")]
fn multiple_pops_trigger_debug_assert() {
    let mut stack = ScopeStack::new();
    stack.push_scope();
    stack.push_scope();
    stack.pop_scope();
    stack.pop_scope();
    stack.pop_scope(); // base scope — debug_assert fires
}

#[test]
fn redeclaration_in_current_scope_fails() {
    let mut stack = ScopeStack::new();
    let span1 = Span::new(0, 1);
    let span2 = Span::new(10, 12);
    assert!(stack.bind("x", SymbolKind::PatternBound, span1).is_ok());
    let err = stack
        .bind("x", SymbolKind::PatternBound, span2)
        .unwrap_err();
    assert_eq!(err, span1);
}

#[test]
fn shadowing_across_scopes_allowed() {
    let mut stack = ScopeStack::new();
    let span1 = Span::new(0, 1);
    let span2 = Span::new(10, 12);
    stack.bind("x", SymbolKind::PatternBound, span1);
    stack.push_scope();
    assert!(stack.bind("x", SymbolKind::WithAlias, span2).is_ok());
}
