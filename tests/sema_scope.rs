//! Integration tests for [`cypher::sema::ScopeStack`] scope tracking.
//!
//! These tests exercise the push/pop/bind/resolve mechanics of the scope
//! stack independently of query parsing.

use cypher::error::Span;
use cypher::sema::{ScopeStack, SymbolKind};

/// Return a unit-length dummy span (byte offsets 0..1) for tests that do not care about position.
fn dummy_span() -> Span {
    Span::new(0, 1)
}

/// Binding to an outer scope remains visible after an inner scope is popped,
/// while a variable bound to the inner scope is no longer accessible.
///
/// Unit: `ScopeStack::push_scope` / `pop_scope` / `is_bound`
/// Precondition: `"outer"` is bound before pushing a new scope; `"inner"` is bound after.
/// Expectation: After `pop_scope`, `"outer"` is still bound and `"inner"` is not.
#[test]
fn bind_after_push_pop_works() {
    let mut stack = ScopeStack::new();
    let _ = stack.bind("outer", SymbolKind::PatternBound, dummy_span());
    stack.push_scope();
    let _ = stack.bind("inner", SymbolKind::WithAlias, dummy_span());
    stack.pop_scope();
    // After popping, "outer" should still be resolvable
    assert!(stack.is_bound("outer"));
    // "inner" should not be visible anymore
    assert!(!stack.is_bound("inner"));
}

/// Attempting to pop the base scope triggers a `debug_assert!` panic.
///
/// Unit: `ScopeStack::pop_scope`
/// Precondition: Only the base scope exists (no pushed inner scope).
/// Expectation: `pop_scope` panics with "attempted to pop the base scope" in debug builds.
#[test]
#[should_panic(expected = "attempted to pop the base scope")]
fn pop_base_scope_panics_in_debug() {
    let mut stack = ScopeStack::new();
    stack.pop_scope();
}

/// Calling `pop_scope` more times than `push_scope` eventually reaches the
/// base scope and panics.
///
/// Unit: `ScopeStack::pop_scope`
/// Precondition: Two extra scopes are pushed and then all three are popped.
/// Expectation: The third `pop_scope` call panics.
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

/// Binding the same variable name twice in the same scope returns an error
/// carrying the span of the first binding.
///
/// Unit: `ScopeStack::bind`
/// Precondition: `"x"` is bound once and then bound again in the same scope.
/// Expectation: The second `bind` call returns `Err(first_span)`.
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

/// Binding the same variable name in an inner scope (shadowing) is permitted.
///
/// Unit: `ScopeStack::bind`
/// Precondition: `"x"` is bound in the outer scope; `push_scope` is called;
///   `"x"` is bound again in the inner scope.
/// Expectation: The inner `bind` call returns `Ok`.
#[test]
fn shadowing_across_scopes_allowed() {
    let mut stack = ScopeStack::new();
    let span1 = Span::new(0, 1);
    let span2 = Span::new(10, 12);
    let _ = stack.bind("x", SymbolKind::PatternBound, span1);
    stack.push_scope();
    assert!(stack.bind("x", SymbolKind::WithAlias, span2).is_ok());
}
