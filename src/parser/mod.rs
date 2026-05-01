//! Hand-written error-resilient parser for openCypher.
//!
//! This module contains the lexer and grammar rules that produce a lossless CST
//! backed by `rowan`. The pest-based parser lives in `pest_parser` as a
//! conformance oracle (`#[cfg(test)]` in Phase 1).

pub mod lexer;
