use crate::error::Expected;
use crate::parser::Parser;
use crate::syntax::SyntaxKind;

/// Precedence levels for Pratt parsing (lowest to highest).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Prec(u8);

impl Prec {
    const MIN: Self = Self(0);
    const OR: Self = Self(1);
    const XOR: Self = Self(2);
    const AND: Self = Self(3);
    const NOT: Self = Self(4);
    const COMPARISON: Self = Self(5);
    const ADD_SUB: Self = Self(6);
    const MUL_DIV_MOD: Self = Self(7);
    const POWER: Self = Self(8);
    const UNARY_ADD_SUB: Self = Self(9);
    const POSTFIX: Self = Self(10);
}

pub fn parse_expression(p: &mut Parser) {
    expr_bp(p, Prec::MIN);
}

fn expr_bp(p: &mut Parser, min_bp: Prec) {
    // Handle prefix unary operators: NOT, +, -
    if is_unary_prefix(p) {
        let op = p.current_kind();
        let prec = match op {
            SyntaxKind::KW_NOT => Prec::NOT,
            _ => Prec::UNARY_ADD_SUB,
        };
        let node_kind = match op {
            SyntaxKind::KW_NOT => SyntaxKind::NOT_EXPR,
            _ => SyntaxKind::UNARY_ADD_SUB_EXPR,
        };
        p.start_node(node_kind);
        p.bump();
        p.skip_trivia();
        expr_bp(p, prec);
        p.builder.finish_node();
    } else {
        parse_atom(p);
    }

    // Parse infix/postfix operators
    loop {
        p.skip_trivia();

        if let Some((bp, _)) = infix_op_bp(p) {
            if bp < min_bp {
                break;
            }
            parse_infix_tail(p, bp);
        } else if p.at(SyntaxKind::DOT) {
            if Prec::POSTFIX < min_bp {
                break;
            }
            parse_property_lookup(p);
        } else if p.at(SyntaxKind::L_BRACKET) {
            if Prec::POSTFIX < min_bp {
                break;
            }
            parse_list_index(p);
        } else if p.at(SyntaxKind::KW_IS) {
            if Prec::POSTFIX < min_bp {
                break;
            }
            parse_is_null(p);
        } else if is_string_op(p) {
            if Prec::POSTFIX < min_bp {
                break;
            }
            parse_string_op(p);
        } else if p.at(SyntaxKind::KW_IN) {
            if Prec::POSTFIX < min_bp {
                break;
            }
            p.start_node(SyntaxKind::LIST_OP_EXPR);
            p.bump();
            p.skip_trivia();
            expr_bp(p, Prec::POSTFIX);
            p.builder.finish_node();
        } else if p.at(SyntaxKind::COLON) && is_label_check_follow(p) {
            if Prec::POSTFIX < min_bp {
                break;
            }
            parse_postfix_node_labels(p);
        } else {
            break;
        }
    }
}

fn is_unary_prefix(p: &Parser) -> bool {
    matches!(
        p.current_kind(),
        SyntaxKind::KW_NOT | SyntaxKind::PLUS | SyntaxKind::MINUS
    )
}

fn infix_op_bp(p: &Parser) -> Option<(Prec, ())> {
    let kind = p.current_kind();
    match kind {
        SyntaxKind::KW_OR => Some((Prec::OR, ())),
        SyntaxKind::KW_XOR => Some((Prec::XOR, ())),
        SyntaxKind::KW_AND => Some((Prec::AND, ())),
        SyntaxKind::EQ
        | SyntaxKind::NE
        | SyntaxKind::LT
        | SyntaxKind::GT
        | SyntaxKind::LE
        | SyntaxKind::GE => Some((Prec::COMPARISON, ())),
        SyntaxKind::PLUS | SyntaxKind::MINUS => Some((Prec::ADD_SUB, ())),
        SyntaxKind::STAR | SyntaxKind::SLASH | SyntaxKind::PERCENT => Some((Prec::MUL_DIV_MOD, ())),
        SyntaxKind::POW => Some((Prec::POWER, ())),
        _ => None,
    }
}

fn parse_infix_tail(p: &mut Parser, bp: Prec) {
    let op = p.current_kind();
    let node_kind = match op {
        SyntaxKind::KW_OR => SyntaxKind::OR_EXPR,
        SyntaxKind::KW_XOR => SyntaxKind::XOR_EXPR,
        SyntaxKind::KW_AND => SyntaxKind::AND_EXPR,
        SyntaxKind::EQ
        | SyntaxKind::NE
        | SyntaxKind::LT
        | SyntaxKind::GT
        | SyntaxKind::LE
        | SyntaxKind::GE => SyntaxKind::COMPARISON_EXPR,
        SyntaxKind::PLUS | SyntaxKind::MINUS => SyntaxKind::ADD_SUB_EXPR,
        SyntaxKind::STAR | SyntaxKind::SLASH | SyntaxKind::PERCENT => SyntaxKind::MUL_DIV_MOD_EXPR,
        SyntaxKind::POW => SyntaxKind::POWER_EXPR,
        _ => SyntaxKind::EXPRESSION,
    };

    p.start_node(node_kind);
    p.bump();
    p.skip_trivia();

    let rhs_bp = match op {
        SyntaxKind::POW => bp, // right-associative
        _ => Prec(bp.0 + 1),   // left-associative
    };
    expr_bp(p, rhs_bp);
    p.builder.finish_node();
}

fn parse_property_lookup(p: &mut Parser) {
    p.start_node(SyntaxKind::PROPERTY_LOOKUP);
    p.bump(); // DOT
    p.skip_trivia();
    if p.at(SyntaxKind::IDENT) || p.at(SyntaxKind::ESCAPED_IDENT) || is_keyword_as_name(p) {
        p.start_node(SyntaxKind::PROPERTY_KEY_NAME);
        p.start_node(SyntaxKind::SYMBOLIC_NAME);
        p.bump();
        p.builder.finish_node();
        p.builder.finish_node();
    }
    p.builder.finish_node();
}

fn parse_list_index(p: &mut Parser) {
    p.start_node(SyntaxKind::LIST_OP_EXPR);
    p.bump(); // L_BRACKET
    p.skip_trivia();
    if !p.at(SyntaxKind::DOT_DOT) && !p.at(SyntaxKind::R_BRACKET) {
        expr_bp(p, Prec::MIN);
        p.skip_trivia();
    }
    if p.at(SyntaxKind::DOT_DOT) {
        p.bump();
        p.skip_trivia();
        if !p.at(SyntaxKind::R_BRACKET) {
            expr_bp(p, Prec::MIN);
            p.skip_trivia();
        }
    }
    p.expect(SyntaxKind::R_BRACKET);
    p.builder.finish_node();
}

fn parse_is_null(p: &mut Parser) {
    p.start_node(SyntaxKind::NULL_OP_EXPR);
    p.bump(); // IS
    p.skip_trivia();
    p.eat(SyntaxKind::KW_NOT);
    p.skip_trivia();
    p.expect(SyntaxKind::NULL_KW);
    p.builder.finish_node();
}

fn is_string_op(p: &Parser) -> bool {
    matches!(
        p.current_kind(),
        SyntaxKind::KW_STARTS | SyntaxKind::KW_ENDS | SyntaxKind::KW_CONTAINS
    )
}

fn parse_string_op(p: &mut Parser) {
    p.start_node(SyntaxKind::STRING_OP_EXPR);
    let op = p.current_kind();
    p.bump();
    if op == SyntaxKind::KW_STARTS || op == SyntaxKind::KW_ENDS {
        p.skip_trivia();
        p.expect(SyntaxKind::KW_WITH);
    }
    p.skip_trivia();
    expr_bp(p, Prec::POSTFIX);
    p.builder.finish_node();
}

fn is_label_check_follow(p: &Parser) -> bool {
    // COLON followed by IDENT/ESCAPED_IDENT or another COLON (multiple labels)
    // but NOT when it's in a property chain context (like :Label:Label)
    let mut lx = p.lexer.clone();
    // skip past COLON
    let _ = lx.advance();
    loop {
        match lx.advance() {
            Some(tok) if tok.kind == SyntaxKind::WHITESPACE => continue,
            Some(tok) => {
                return tok.kind == SyntaxKind::IDENT
                    || tok.kind == SyntaxKind::ESCAPED_IDENT
                    || tok.kind == SyntaxKind::COLON;
            }
            None => return false,
        }
    }
}

fn parse_postfix_node_labels(p: &mut Parser) {
    p.start_node(SyntaxKind::PROPERTY_OR_LABELS_EXPR);
    while p.at(SyntaxKind::COLON) {
        p.bump();
        p.skip_trivia();
        if p.at(SyntaxKind::IDENT) || p.at(SyntaxKind::ESCAPED_IDENT) || is_keyword_as_name(p) {
            p.start_node(SyntaxKind::NODE_LABELS);
            p.start_node(SyntaxKind::NODE_LABEL);
            p.start_node(SyntaxKind::LABEL_NAME);
            p.start_node(SyntaxKind::SYMBOLIC_NAME);
            p.bump();
            p.builder.finish_node();
            p.builder.finish_node();
            p.builder.finish_node();
            p.builder.finish_node();
        }
        p.skip_trivia();
    }
    p.builder.finish_node();
}

fn parse_atom(p: &mut Parser) {
    match p.current_kind() {
        SyntaxKind::INTEGER | SyntaxKind::FLOAT => {
            p.start_node(SyntaxKind::NUMBER_LITERAL);
            p.bump();
            p.builder.finish_node();
        }
        SyntaxKind::STRING => {
            p.start_node(SyntaxKind::STRING_LITERAL);
            p.bump();
            p.builder.finish_node();
        }
        SyntaxKind::TRUE_KW | SyntaxKind::FALSE_KW => {
            p.start_node(SyntaxKind::BOOLEAN_LITERAL);
            p.bump();
            p.builder.finish_node();
        }
        SyntaxKind::NULL_KW => {
            p.start_node(SyntaxKind::NULL_KW);
            p.bump();
            p.builder.finish_node();
        }
        SyntaxKind::KW_COUNT if p.peek_next_non_trivia() == Some(SyntaxKind::L_PAREN) => {
            p.start_node(SyntaxKind::FUNCTION_INVOCATION);
            p.start_node(SyntaxKind::FUNCTION_NAME);
            p.start_node(SyntaxKind::SYMBOLIC_NAME);
            p.bump();
            p.builder.finish_node();
            p.builder.finish_node();
            p.skip_trivia();
            p.bump(); // L_PAREN
            p.skip_trivia();
            p.bump(); // STAR
            p.skip_trivia();
            p.expect(SyntaxKind::R_PAREN);
            p.builder.finish_node();
        }
        SyntaxKind::IDENT | SyntaxKind::ESCAPED_IDENT | SyntaxKind::KW_COUNT => {
            // Detect qualified function calls: `ns1.ns2.name(args)`. We can't
            // rewrite from a VARIABLE + PROPERTY_LOOKUP chain after the fact,
            // so look ahead before committing and build a FUNCTION_INVOCATION
            // with a FUNCTION_NAME containing all SYMBOLIC_NAME parts.
            let is_qcall = p.looks_like_qualified_call();
            if is_qcall {
                p.start_node(SyntaxKind::FUNCTION_INVOCATION);
                p.start_node(SyntaxKind::FUNCTION_NAME);
                // First part
                p.start_node(SyntaxKind::SYMBOLIC_NAME);
                p.bump();
                p.builder.finish_node();
                // Subsequent `.IDENT` parts
                loop {
                    p.skip_trivia();
                    if !p.at(SyntaxKind::DOT) {
                        break;
                    }
                    p.bump(); // DOT
                    p.skip_trivia();
                    if p.at(SyntaxKind::IDENT)
                        || p.at(SyntaxKind::ESCAPED_IDENT)
                        || is_keyword_as_name(p)
                    {
                        p.start_node(SyntaxKind::SYMBOLIC_NAME);
                        p.bump();
                        p.builder.finish_node();
                    } else {
                        break;
                    }
                }
                p.builder.finish_node(); // FUNCTION_NAME
                p.skip_trivia();
                p.expect(SyntaxKind::L_PAREN);
                p.skip_trivia();
                p.eat(SyntaxKind::KW_DISTINCT);
                p.skip_trivia();
                if !p.at(SyntaxKind::R_PAREN) {
                    expr_bp(p, Prec::MIN);
                    p.skip_trivia();
                    while p.eat(SyntaxKind::COMMA) {
                        p.skip_trivia();
                        expr_bp(p, Prec::MIN);
                        p.skip_trivia();
                    }
                }
                p.expect(SyntaxKind::R_PAREN);
                p.builder.finish_node(); // FUNCTION_INVOCATION
                return;
            }
            // Consume identifier first
            p.start_node(SyntaxKind::VARIABLE);
            p.start_node(SyntaxKind::SYMBOLIC_NAME);
            p.bump();
            p.builder.finish_node();
            p.builder.finish_node();
            p.skip_trivia();
            // After identifier, check for MapProjection: var { ... }
            if p.at(SyntaxKind::L_BRACE) {
                let checkpoint = p.checkpoint();
                p.start_node_at(checkpoint, SyntaxKind::MAP_PROJECTION);
                p.bump(); // {
                p.skip_trivia();
                if !p.at(SyntaxKind::R_BRACE) {
                    parse_map_projection_item(p);
                    p.skip_trivia();
                    while p.eat(SyntaxKind::COMMA) {
                        p.skip_trivia();
                        parse_map_projection_item(p);
                        p.skip_trivia();
                    }
                }
                p.expect(SyntaxKind::R_BRACE);
                p.builder.finish_node();
            } else if p.at(SyntaxKind::L_PAREN) {
                // Was actually a function invocation — re-wrap as FUNCTION_INVOCATION
                let checkpoint = p.checkpoint();
                p.start_node_at(checkpoint, SyntaxKind::FUNCTION_INVOCATION);
                p.bump(); // L_PAREN
                p.skip_trivia();
                p.eat(SyntaxKind::KW_DISTINCT);
                p.skip_trivia();
                if !p.at(SyntaxKind::R_PAREN) {
                    expr_bp(p, Prec::MIN);
                    p.skip_trivia();
                    while p.eat(SyntaxKind::COMMA) {
                        p.skip_trivia();
                        expr_bp(p, Prec::MIN);
                        p.skip_trivia();
                    }
                }
                p.expect(SyntaxKind::R_PAREN);
                p.builder.finish_node();
            }
        }
        SyntaxKind::L_PAREN => {
            // Check if this is a RelationshipsPattern (pattern-as-atom)
            // Peeking: after ( should be optional var, optional :Label, optional {props}, then )
            // followed by - or < for chain start
            if looks_like_relationships_pattern(p) {
                p.start_node(SyntaxKind::RELATIONSHIPS_PATTERN);
                parse_node_pattern_for_atom(p);
                p.skip_trivia();
                while is_relationship_chain_start(p) {
                    p.start_node(SyntaxKind::PATTERN_ELEMENT_CHAIN);
                    parse_relationship_pattern(p);
                    p.skip_trivia();
                    parse_node_pattern_for_atom(p);
                    p.builder.finish_node();
                    p.skip_trivia();
                }
                p.builder.finish_node();
            } else {
                p.start_node(SyntaxKind::PARENTHESIZED_EXPR);
                p.bump();
                p.skip_trivia();
                expr_bp(p, Prec::MIN);
                p.skip_trivia();
                p.expect(SyntaxKind::R_PAREN);
                p.builder.finish_node();
            }
        }
        SyntaxKind::L_BRACKET => {
            // Disambiguate: ListComprehension vs PatternComprehension vs ListLiteral
            parse_bracket_expr(p);
        }
        SyntaxKind::L_BRACE => {
            p.start_node(SyntaxKind::MAP_LITERAL);
            p.bump();
            p.skip_trivia();
            if !p.at(SyntaxKind::R_BRACE) {
                parse_map_entry(p);
                p.skip_trivia();
                while p.eat(SyntaxKind::COMMA) {
                    p.skip_trivia();
                    parse_map_entry(p);
                    p.skip_trivia();
                }
            }
            p.expect(SyntaxKind::R_BRACE);
            p.builder.finish_node();
        }
        SyntaxKind::DOLLAR => {
            p.start_node(SyntaxKind::PARAMETER);
            p.bump();
            p.skip_trivia();
            if p.at(SyntaxKind::IDENT)
                || p.at(SyntaxKind::ESCAPED_IDENT)
                || p.at(SyntaxKind::INTEGER)
            {
                p.bump();
            }
            p.builder.finish_node();
        }
        SyntaxKind::KW_CASE => {
            parse_case_expr(p);
        }
        SyntaxKind::KW_ALL
        | SyntaxKind::KW_ANY
        | SyntaxKind::KW_NONE
        | SyntaxKind::KW_SINGLE
        | SyntaxKind::KW_FILTER
        | SyntaxKind::KW_EXTRACT => {
            if p.peek_next_non_trivia() == Some(SyntaxKind::L_PAREN) {
                p.start_node(SyntaxKind::FUNCTION_INVOCATION);
                p.start_node(SyntaxKind::FUNCTION_NAME);
                p.start_node(SyntaxKind::SYMBOLIC_NAME);
                p.bump();
                p.builder.finish_node();
                p.builder.finish_node();
                p.skip_trivia();
                p.bump();
                p.skip_trivia();
                parse_filter_like_expr(p);
                p.skip_trivia();
                p.expect(SyntaxKind::R_PAREN);
                p.builder.finish_node();
            } else {
                p.start_node(SyntaxKind::VARIABLE);
                p.start_node(SyntaxKind::SYMBOLIC_NAME);
                p.bump();
                p.builder.finish_node();
                p.builder.finish_node();
            }
        }
        SyntaxKind::KW_EXISTS => {
            if p.peek_next_non_trivia() == Some(SyntaxKind::L_BRACE) {
                p.start_node(SyntaxKind::EXISTS_SUBQUERY);
                p.bump(); // EXISTS
                p.skip_trivia();
                p.bump(); // {
                p.skip_trivia();
                // Try RegularQuery first (clauses like MATCH, RETURN, etc.)
                if is_clause_start_for_exists(p) {
                    // It's a RegularQuery body
                    while !p.at(SyntaxKind::R_BRACE) && p.current_len() > 0 {
                        if is_clause_start_for_subquery(p) {
                            parse_clause(p);
                        } else {
                            p.start_node(SyntaxKind::ERROR);
                            p.bump();
                            p.builder.finish_node();
                        }
                        p.skip_trivia();
                    }
                } else {
                    // Pattern (WHERE)?
                    parse_node_pattern_for_atom(p);
                    p.skip_trivia();
                    while is_relationship_chain_start(p) {
                        p.start_node(SyntaxKind::PATTERN_ELEMENT_CHAIN);
                        parse_relationship_pattern(p);
                        p.skip_trivia();
                        parse_node_pattern_for_atom(p);
                        p.builder.finish_node();
                        p.skip_trivia();
                    }
                    p.skip_trivia();
                    if p.at(SyntaxKind::KW_WHERE) {
                        p.start_node(SyntaxKind::WHERE_CLAUSE);
                        p.bump();
                        p.skip_trivia();
                        expr_bp(p, Prec::MIN);
                        p.builder.finish_node();
                        p.skip_trivia();
                    }
                }
                p.expect(SyntaxKind::R_BRACE);
                p.builder.finish_node();
            } else {
                p.start_node(SyntaxKind::VARIABLE);
                p.start_node(SyntaxKind::SYMBOLIC_NAME);
                p.bump();
                p.builder.finish_node();
                p.builder.finish_node();
            }
        }
        _ => {
            if p.current_len() > 0 {
                let start = p.byte_pos;
                let end = start + p.current_len;
                if let Some(text) = p.input.get(start..end) {
                    if text.starts_with('\'') || text.starts_with('"') {
                        p.error_here(&[Expected::Category(
                            "closing quote for unterminated string",
                        )]);
                    } else {
                        p.error_here(&[Expected::Category("expression")]);
                    }
                } else {
                    p.error_here(&[Expected::Category("expression")]);
                }
            } else {
                p.error_here(&[Expected::Category("expression")]);
            }
            p.start_node(SyntaxKind::ERROR);
            if p.current_len() > 0 {
                p.bump();
            }
            p.builder.finish_node();
        }
    }
}

fn parse_map_projection_item(p: &mut Parser) {
    if p.at(SyntaxKind::DOT) {
        p.start_node(SyntaxKind::MAP_PROJECTION_ITEM);
        p.bump();
        if p.at(SyntaxKind::STAR) {
            p.bump();
        } else {
            p.start_node(SyntaxKind::PROPERTY_KEY_NAME);
            p.start_node(SyntaxKind::SYMBOLIC_NAME);
            if p.at(SyntaxKind::IDENT) || p.at(SyntaxKind::ESCAPED_IDENT) || is_keyword_as_name(p) {
                p.bump();
            }
            p.builder.finish_node();
            p.builder.finish_node();
        }
        p.builder.finish_node();
    } else if p.at(SyntaxKind::IDENT) || p.at(SyntaxKind::ESCAPED_IDENT) || is_keyword_as_name(p) {
        // alias: expr or PropertyKeyName
        let checkpoint = p.checkpoint();
        let name_kind = p.current_kind();
        p.start_node(SyntaxKind::SYMBOLIC_NAME);
        p.bump();
        p.builder.finish_node();
        p.skip_trivia();
        if p.at(SyntaxKind::COLON) {
            // alias: expr — wrap as MAP_PROJECTION_ITEM
            p.start_node_at(checkpoint, SyntaxKind::MAP_PROJECTION_ITEM);
            p.start_node(SyntaxKind::PROPERTY_KEY_NAME);
            // SYMBOLIC_NAME already emitted as child
            p.builder.finish_node();
            p.bump(); // COLON
            p.skip_trivia();
            expr_bp(p, Prec::MIN);
            p.builder.finish_node();
        } else {
            // bare PropertyKeyName — wrap as MAP_PROJECTION_ITEM
            p.start_node_at(checkpoint, SyntaxKind::MAP_PROJECTION_ITEM);
            p.start_node(SyntaxKind::PROPERTY_KEY_NAME);
            // SYMBOLIC_NAME already emitted
            p.builder.finish_node();
            p.builder.finish_node();
        }
    } else {
        // bare property lookup expression as map projection item
        p.start_node(SyntaxKind::MAP_PROJECTION_ITEM);
        expr_bp(p, Prec::MIN);
        p.builder.finish_node();
    }
}

/// Determine if we're looking at a RelationshipsPattern starting from `(`.
/// Heuristic: after (, we expect optional var, optional :Label, optional {props}, )
/// and then - or < for a chain. If there's no chain after ), it's just a parenthesized expr.
fn looks_like_relationships_pattern(p: &Parser) -> bool {
    // Clone lexer to peek ahead
    let mut lx = p.lexer.clone();
    // Skip past L_PAREN
    loop {
        match lx.advance() {
            Some(tok) if tok.kind == SyntaxKind::WHITESPACE => continue,
            _ => break,
        }
    }
    // Now we should be at optional IDENT, or : or )
    // Skip optional identifier
    if let Some(tok) = lx.advance() {
        if tok.kind == SyntaxKind::WHITESPACE {
            loop {
                match lx.advance() {
                    Some(t) if t.kind == SyntaxKind::WHITESPACE => continue,
                    Some(t) => {
                        if t.kind == SyntaxKind::IDENT || t.kind == SyntaxKind::ESCAPED_IDENT {
                            // Skip whitespace after ident
                            loop {
                                match lx.advance() {
                                    Some(t2) if t2.kind == SyntaxKind::WHITESPACE => continue,
                                    _ => break,
                                }
                            }
                        }
                        break;
                    }
                    None => break,
                }
            }
        }
    }
    // Skip optional :Label sequences
    loop {
        // peek current
        // We can't easily peek without advancing, so use a different approach:
        // Just look for the ) then check if - or < follows
        break;
    }
    // Simplified heuristic: scan for ) followed by - or <
    let mut found_close_paren = false;
    loop {
        match lx.advance() {
            Some(tok) if tok.kind == SyntaxKind::WHITESPACE => continue,
            Some(tok) if tok.kind == SyntaxKind::R_PAREN => {
                found_close_paren = true;
                break;
            }
            Some(_) => continue,
            None => return false,
        }
    }
    if !found_close_paren {
        return false;
    }
    // After ), check if - or < follows
    loop {
        match lx.advance() {
            Some(tok) if tok.kind == SyntaxKind::WHITESPACE => continue,
            Some(tok)
                if tok.kind == SyntaxKind::MINUS
                    || tok.kind == SyntaxKind::DASH
                    || tok.kind == SyntaxKind::LT
                    || tok.kind == SyntaxKind::ARROW_LEFT =>
            {
                return true;
            }
            _ => return false,
        }
    }
}

fn parse_node_pattern_for_atom(p: &mut Parser) {
    // Parse ( optional var :Label* {props}? )
    p.start_node(SyntaxKind::NODE_PATTERN);
    p.bump(); // (
    p.skip_trivia();
    if p.at(SyntaxKind::IDENT) || p.at(SyntaxKind::ESCAPED_IDENT) || is_keyword_as_name(p) {
        p.start_node(SyntaxKind::VARIABLE);
        p.start_node(SyntaxKind::SYMBOLIC_NAME);
        p.bump();
        p.builder.finish_node();
        p.builder.finish_node();
        p.skip_trivia();
    }
    while p.at(SyntaxKind::COLON) {
        p.start_node(SyntaxKind::NODE_LABELS);
        p.bump();
        p.skip_trivia();
        if p.at(SyntaxKind::IDENT) || p.at(SyntaxKind::ESCAPED_IDENT) || is_keyword_as_name(p) {
            p.start_node(SyntaxKind::NODE_LABEL);
            p.start_node(SyntaxKind::LABEL_NAME);
            p.start_node(SyntaxKind::SYMBOLIC_NAME);
            p.bump();
            p.builder.finish_node();
            p.builder.finish_node();
            p.builder.finish_node();
        }
        p.builder.finish_node();
        p.skip_trivia();
    }
    if p.at(SyntaxKind::L_BRACE) {
        p.start_node(SyntaxKind::PROPERTIES);
        p.start_node(SyntaxKind::MAP_LITERAL);
        p.bump();
        p.skip_trivia();
        if !p.at(SyntaxKind::R_BRACE) {
            parse_map_entry(p);
            p.skip_trivia();
            while p.eat(SyntaxKind::COMMA) {
                p.skip_trivia();
                parse_map_entry(p);
                p.skip_trivia();
            }
        }
        p.expect(SyntaxKind::R_BRACE);
        p.builder.finish_node();
        p.builder.finish_node();
        p.skip_trivia();
    }
    p.expect(SyntaxKind::R_PAREN);
    p.builder.finish_node();
}

/// Disambiguate [ ... ] as ListComprehension, PatternComprehension, or ListLiteral.
fn parse_bracket_expr(p: &mut Parser) {
    let checkpoint = p.checkpoint();
    // Peek inside to disambiguate
    let mut lx = p.lexer.clone();
    // Skip past L_BRACKET
    loop {
        match lx.advance() {
            Some(tok) if tok.kind == SyntaxKind::WHITESPACE => continue,
            _ => break,
        }
    }
    // Check for PatternComprehension: optional var = (pattern
    // or ListComprehension: var IN expr
    let mut is_list_comprehension = false;
    let mut is_pattern_comprehension = false;

    // Peek first non-ws token
    if let Some(tok) = lx.advance() {
        if tok.kind == SyntaxKind::IDENT || tok.kind == SyntaxKind::ESCAPED_IDENT {
            // Could be "var IN ..." (list comp) or "var = ..." (pattern comp)
            loop {
                match lx.advance() {
                    Some(t) if t.kind == SyntaxKind::WHITESPACE => continue,
                    Some(t) => {
                        if t.kind == SyntaxKind::KW_IN {
                            is_list_comprehension = true;
                        } else if t.kind == SyntaxKind::EQ {
                            is_pattern_comprehension = true;
                        }
                        break;
                    }
                    None => break,
                }
            }
        } else if tok.kind == SyntaxKind::L_PAREN
            || tok.kind == SyntaxKind::MINUS
            || tok.kind == SyntaxKind::DASH
        {
            // PatternComprehension with optional variable omitted:
            // [ (node)-[rel]->(node) WHERE ... | expr ]
            // or [ -[rel]->(node) WHERE ... | expr ] (anonymous start)
            is_pattern_comprehension = true;
        }
    }

    if is_pattern_comprehension {
        p.start_node_at(checkpoint, SyntaxKind::PATTERN_COMPREHENSION);
        p.bump(); // [
        p.skip_trivia();
        // Optional variable =
        if p.at(SyntaxKind::IDENT) || p.at(SyntaxKind::ESCAPED_IDENT) {
            let next = p.peek_next_non_trivia();
            if next == Some(SyntaxKind::EQ) {
                p.start_node(SyntaxKind::VARIABLE);
                p.start_node(SyntaxKind::SYMBOLIC_NAME);
                p.bump();
                p.builder.finish_node();
                p.builder.finish_node();
                p.skip_trivia();
                p.bump(); // =
                p.skip_trivia();
            }
        }
        // RelationshipsPattern
        parse_relationships_pattern_body(p);
        p.skip_trivia();
        // Optional WHERE
        if p.at(SyntaxKind::KW_WHERE) {
            p.start_node(SyntaxKind::WHERE_CLAUSE);
            p.bump();
            p.skip_trivia();
            expr_bp(p, Prec::MIN);
            p.builder.finish_node();
            p.skip_trivia();
        }
        p.expect(SyntaxKind::PIPE);
        p.skip_trivia();
        expr_bp(p, Prec::MIN);
        p.skip_trivia();
        p.expect(SyntaxKind::R_BRACKET);
        p.builder.finish_node();
    } else if is_list_comprehension {
        p.start_node_at(checkpoint, SyntaxKind::LIST_COMPREHENSION);
        p.bump(); // [
        p.skip_trivia();
        // FilterExpression: IdInColl (WHERE)?
        parse_filter_expression(p);
        p.skip_trivia();
        // Optional | body
        if p.at(SyntaxKind::PIPE) {
            p.bump();
            p.skip_trivia();
            expr_bp(p, Prec::MIN);
            p.skip_trivia();
        }
        p.expect(SyntaxKind::R_BRACKET);
        p.builder.finish_node();
    } else {
        // Plain ListLiteral
        p.start_node_at(checkpoint, SyntaxKind::LIST_LITERAL);
        p.bump(); // [
        p.skip_trivia();
        if !p.at(SyntaxKind::R_BRACKET) {
            expr_bp(p, Prec::MIN);
            p.skip_trivia();
            while p.eat(SyntaxKind::COMMA) {
                p.skip_trivia();
                expr_bp(p, Prec::MIN);
                p.skip_trivia();
            }
        }
        p.expect(SyntaxKind::R_BRACKET);
        p.builder.finish_node();
    }
}

fn parse_relationships_pattern_body(p: &mut Parser) {
    // Parse: NodePattern (PatternElementChain)+
    parse_node_pattern_for_atom(p);
    p.skip_trivia();
    while is_relationship_chain_start(p) {
        p.start_node(SyntaxKind::PATTERN_ELEMENT_CHAIN);
        parse_relationship_pattern(p);
        p.skip_trivia();
        parse_node_pattern_for_atom(p);
        p.builder.finish_node();
        p.skip_trivia();
    }
}

fn parse_filter_expression(p: &mut Parser) {
    // IdInColl (WHERE)?
    p.start_node(SyntaxKind::FILTER_EXPRESSION);
    // Variable
    p.start_node(SyntaxKind::ID_IN_COLL);
    p.start_node(SyntaxKind::VARIABLE);
    p.start_node(SyntaxKind::SYMBOLIC_NAME);
    if p.at(SyntaxKind::IDENT) || p.at(SyntaxKind::ESCAPED_IDENT) || is_keyword_as_name(p) {
        p.bump();
    }
    p.builder.finish_node();
    p.builder.finish_node();
    p.skip_trivia();
    // IN
    p.expect(SyntaxKind::KW_IN);
    p.skip_trivia();
    // Expression (collection)
    expr_bp(p, Prec::MIN);
    p.builder.finish_node();
    p.builder.finish_node();
    p.skip_trivia();
    // Optional WHERE
    if p.at(SyntaxKind::KW_WHERE) {
        // WHERE is a child of FILTER_EXPRESSION, sibling of ID_IN_COLL
        p.start_node(SyntaxKind::WHERE_CLAUSE);
        p.bump();
        p.skip_trivia();
        expr_bp(p, Prec::MIN);
        p.builder.finish_node();
        p.skip_trivia();
    }
}

fn parse_filter_like_expr(p: &mut Parser) {
    // Parse the inner expression for ALL/ANY/NONE/SINGLE/FILTER/EXTRACT(...)
    // These use: variable IN expression (WHERE predicate)? , expression
    // variable IN expression (WHERE predicate)? | expression
    // variable IN expression
    if p.at(SyntaxKind::IDENT) || p.at(SyntaxKind::ESCAPED_IDENT) {
        p.start_node(SyntaxKind::VARIABLE);
        p.start_node(SyntaxKind::SYMBOLIC_NAME);
        p.bump();
        p.builder.finish_node();
        p.builder.finish_node();
        p.skip_trivia();
    }
    if p.at(SyntaxKind::KW_IN) {
        p.bump();
        p.skip_trivia();
        expr_bp(p, Prec::MIN);
        p.skip_trivia();
    }
    if p.at(SyntaxKind::KW_WHERE) {
        p.bump();
        p.skip_trivia();
        expr_bp(p, Prec::MIN);
        p.skip_trivia();
    }
    // Optional , or | separator followed by another expression
    if p.at(SyntaxKind::COMMA) || p.at(SyntaxKind::PIPE) {
        p.bump();
        p.skip_trivia();
        expr_bp(p, Prec::MIN);
        p.skip_trivia();
    }
}

fn is_clause_start_for_exists(p: &Parser) -> bool {
    matches!(
        p.current_kind(),
        SyntaxKind::KW_MATCH
            | SyntaxKind::KW_RETURN
            | SyntaxKind::KW_WITH
            | SyntaxKind::KW_UNWIND
            | SyntaxKind::KW_CREATE
            | SyntaxKind::KW_MERGE
            | SyntaxKind::KW_DELETE
            | SyntaxKind::KW_SET
            | SyntaxKind::KW_REMOVE
            | SyntaxKind::KW_CALL
            | SyntaxKind::KW_FOREACH
            | SyntaxKind::KW_OPTIONAL
            | SyntaxKind::KW_DETACH
            | SyntaxKind::KW_YIELD
    )
}

fn parse_namespace_and_name(p: &mut Parser) {
    loop {
        p.start_node(SyntaxKind::SYMBOLIC_NAME);
        p.bump();
        p.builder.finish_node();
        p.skip_trivia();
        if p.at(SyntaxKind::DOT) {
            p.bump();
            p.skip_trivia();
            if !p.at(SyntaxKind::IDENT) && !p.at(SyntaxKind::ESCAPED_IDENT) {
                break;
            }
        } else {
            break;
        }
    }
}

fn parse_map_entry(p: &mut Parser) {
    p.start_node(SyntaxKind::MAP_ENTRY);
    if p.at(SyntaxKind::DOT) {
        p.bump();
        if p.at(SyntaxKind::STAR) {
            p.bump();
        } else {
            p.start_node(SyntaxKind::PROPERTY_KEY_NAME);
            p.start_node(SyntaxKind::SYMBOLIC_NAME);
            if p.at(SyntaxKind::IDENT) || p.at(SyntaxKind::ESCAPED_IDENT) || is_keyword_as_name(p) {
                p.bump();
            }
            p.builder.finish_node();
            p.builder.finish_node();
        }
    } else {
        p.start_node(SyntaxKind::PROPERTY_KEY_NAME);
        p.start_node(SyntaxKind::SYMBOLIC_NAME);
        if p.at(SyntaxKind::IDENT) || p.at(SyntaxKind::ESCAPED_IDENT) || is_keyword_as_name(p) {
            p.bump();
        }
        p.builder.finish_node();
        p.builder.finish_node();
        p.skip_trivia();
        if p.eat(SyntaxKind::COLON) {
            p.skip_trivia();
            expr_bp(p, Prec::MIN);
        }
    }
    p.builder.finish_node();
}

fn parse_case_expr(p: &mut Parser) {
    p.start_node(SyntaxKind::CASE_EXPR);
    p.bump();
    p.skip_trivia();
    if !p.at(SyntaxKind::KW_WHEN) {
        expr_bp(p, Prec::MIN);
        p.skip_trivia();
    }
    while p.at(SyntaxKind::KW_WHEN) {
        p.start_node(SyntaxKind::CASE_ALTERNATIVE);
        p.bump();
        p.skip_trivia();
        expr_bp(p, Prec::MIN);
        p.skip_trivia();
        p.expect(SyntaxKind::KW_THEN);
        p.skip_trivia();
        expr_bp(p, Prec::MIN);
        p.builder.finish_node();
        p.skip_trivia();
    }
    if p.eat(SyntaxKind::KW_ELSE) {
        p.skip_trivia();
        expr_bp(p, Prec::MIN);
        p.skip_trivia();
    }
    p.expect(SyntaxKind::KW_END);
    p.builder.finish_node();
}

pub fn parse_clause(p: &mut Parser) {
    p.skip_trivia();
    match p.current_kind() {
        SyntaxKind::KW_MATCH | SyntaxKind::KW_OPTIONAL => parse_match_clause(p),
        SyntaxKind::KW_RETURN => parse_return_clause(p),
        SyntaxKind::KW_WITH => parse_with_clause(p),
        SyntaxKind::KW_UNWIND => parse_unwind_clause(p),
        SyntaxKind::KW_CREATE => parse_create_clause(p),
        SyntaxKind::KW_MERGE => parse_merge_clause(p),
        SyntaxKind::KW_DELETE | SyntaxKind::KW_DETACH => parse_delete_clause(p),
        SyntaxKind::KW_SET => parse_set_clause(p),
        SyntaxKind::KW_REMOVE => parse_remove_clause(p),
        SyntaxKind::KW_FOREACH => parse_foreach_clause(p),
        SyntaxKind::KW_CALL => parse_call_clause(p),
        SyntaxKind::KW_YIELD => parse_yield_clause(p),
        SyntaxKind::KW_DROP => parse_drop_clause(p),
        SyntaxKind::KW_SHOW => parse_show_clause(p),
        SyntaxKind::KW_USE => parse_use_clause(p),
        _ => {}
    }
}

fn parse_match_clause(p: &mut Parser) {
    p.start_node(SyntaxKind::MATCH_CLAUSE);
    p.eat(SyntaxKind::KW_OPTIONAL);
    p.skip_trivia();
    p.expect(SyntaxKind::KW_MATCH);
    p.skip_trivia();
    parse_pattern(p);
    p.skip_trivia();
    if p.at(SyntaxKind::KW_WHERE) {
        p.start_node(SyntaxKind::WHERE_CLAUSE);
        p.bump();
        p.skip_trivia();
        expr_bp(p, Prec::MIN);
        p.builder.finish_node();
        p.skip_trivia();
    }
    p.builder.finish_node();
}

fn parse_return_clause(p: &mut Parser) {
    p.start_node(SyntaxKind::RETURN_CLAUSE);
    p.bump();
    parse_projection_body(p);
    p.builder.finish_node();
}

fn parse_with_clause(p: &mut Parser) {
    p.start_node(SyntaxKind::WITH_CLAUSE);
    p.bump();
    parse_projection_body(p);
    p.skip_trivia();
    if p.at(SyntaxKind::KW_WHERE) {
        p.start_node(SyntaxKind::WHERE_CLAUSE);
        p.bump();
        p.skip_trivia();
        expr_bp(p, Prec::MIN);
        p.builder.finish_node();
    }
    p.builder.finish_node();
}

fn parse_unwind_clause(p: &mut Parser) {
    p.start_node(SyntaxKind::UNWIND_CLAUSE);
    p.bump();
    p.skip_trivia();
    expr_bp(p, Prec::MIN);
    p.skip_trivia();
    p.expect(SyntaxKind::KW_AS);
    p.skip_trivia();
    if p.at(SyntaxKind::IDENT) || p.at(SyntaxKind::ESCAPED_IDENT) {
        p.start_node(SyntaxKind::VARIABLE);
        p.start_node(SyntaxKind::SYMBOLIC_NAME);
        p.bump();
        p.builder.finish_node();
        p.builder.finish_node();
    }
    p.builder.finish_node();
}

fn parse_create_clause(p: &mut Parser) {
    p.skip_trivia();
    let next = p.peek_next_non_trivia();
    if next == Some(SyntaxKind::KW_INDEX)
        || next == Some(SyntaxKind::KW_TEXT)
        || next == Some(SyntaxKind::KW_LOOKUP)
        || next == Some(SyntaxKind::KW_RANGE)
        || next == Some(SyntaxKind::KW_POINT)
        || next == Some(SyntaxKind::KW_FULLTEXT)
    {
        parse_create_index(p);
    } else if next == Some(SyntaxKind::KW_CONSTRAINT) {
        parse_create_constraint(p);
    } else if next == Some(SyntaxKind::KW_DATABASE) || next == Some(SyntaxKind::KW_DATABASES) {
        parse_create_database(p);
    } else {
        parse_create_pattern(p);
    }
}

fn parse_create_pattern(p: &mut Parser) {
    p.start_node(SyntaxKind::CREATE_CLAUSE);
    p.bump();
    p.skip_trivia();
    parse_pattern(p);
    p.builder.finish_node();
}

fn parse_merge_clause(p: &mut Parser) {
    p.start_node(SyntaxKind::MERGE_CLAUSE);
    p.bump();
    p.skip_trivia();
    parse_pattern_part(p);
    p.skip_trivia();
    while p.at(SyntaxKind::KW_ON) {
        p.start_node(SyntaxKind::MERGE_ACTION);
        p.bump();
        p.skip_trivia();
        p.eat(SyntaxKind::KW_MATCH);
        p.eat(SyntaxKind::KW_CREATE);
        p.skip_trivia();
        if p.at(SyntaxKind::KW_SET) {
            p.bump();
            p.skip_trivia();
            parse_set_item(p);
            while p.eat(SyntaxKind::COMMA) {
                p.skip_trivia();
                parse_set_item(p);
            }
        }
        p.builder.finish_node();
        p.skip_trivia();
    }
    p.builder.finish_node();
}

fn parse_delete_clause(p: &mut Parser) {
    p.start_node(SyntaxKind::DELETE_CLAUSE);
    p.eat(SyntaxKind::KW_DETACH);
    p.skip_trivia();
    p.bump();
    p.skip_trivia();
    expr_bp(p, Prec::MIN);
    p.skip_trivia();
    while p.eat(SyntaxKind::COMMA) {
        p.skip_trivia();
        expr_bp(p, Prec::MIN);
        p.skip_trivia();
    }
    p.builder.finish_node();
}

fn parse_set_clause(p: &mut Parser) {
    p.start_node(SyntaxKind::SET_CLAUSE);
    p.bump();
    p.skip_trivia();
    parse_set_item(p);
    p.skip_trivia();
    while p.eat(SyntaxKind::COMMA) {
        p.skip_trivia();
        parse_set_item(p);
        p.skip_trivia();
    }
    p.builder.finish_node();
}

fn parse_set_item(p: &mut Parser) {
    p.start_node(SyntaxKind::SET_ITEM);
    if p.at(SyntaxKind::IDENT) || p.at(SyntaxKind::ESCAPED_IDENT) {
        p.start_node(SyntaxKind::VARIABLE);
        p.start_node(SyntaxKind::SYMBOLIC_NAME);
        p.bump();
        p.builder.finish_node();
        p.builder.finish_node();
        p.skip_trivia();
        while p.at(SyntaxKind::DOT) {
            p.start_node(SyntaxKind::PROPERTY_LOOKUP);
            p.bump();
            p.skip_trivia();
            if p.at(SyntaxKind::IDENT) || p.at(SyntaxKind::ESCAPED_IDENT) {
                p.start_node(SyntaxKind::PROPERTY_KEY_NAME);
                p.start_node(SyntaxKind::SYMBOLIC_NAME);
                p.bump();
                p.builder.finish_node();
                p.builder.finish_node();
            }
            p.builder.finish_node();
            p.skip_trivia();
        }
    }
    p.skip_trivia();
    if p.at(SyntaxKind::PLUSEQ) || p.at(SyntaxKind::EQ) {
        p.bump();
    } else if p.at(SyntaxKind::COLON) {
        p.start_node(SyntaxKind::NODE_LABELS);
        p.bump();
        p.skip_trivia();
        if p.at(SyntaxKind::IDENT) || p.at(SyntaxKind::ESCAPED_IDENT) {
            p.start_node(SyntaxKind::NODE_LABEL);
            p.start_node(SyntaxKind::LABEL_NAME);
            p.start_node(SyntaxKind::SYMBOLIC_NAME);
            p.bump();
            p.builder.finish_node();
            p.builder.finish_node();
        }
        p.builder.finish_node();
        p.builder.finish_node();
        return;
    }
    p.skip_trivia();
    expr_bp(p, Prec::MIN);
    p.builder.finish_node();
}

fn parse_remove_clause(p: &mut Parser) {
    p.start_node(SyntaxKind::REMOVE_CLAUSE);
    p.bump();
    p.skip_trivia();
    parse_remove_item(p);
    while p.eat(SyntaxKind::COMMA) {
        p.skip_trivia();
        parse_remove_item(p);
    }
    p.builder.finish_node();
}

fn parse_remove_item(p: &mut Parser) {
    p.start_node(SyntaxKind::REMOVE_ITEM);
    if p.at(SyntaxKind::IDENT) || p.at(SyntaxKind::ESCAPED_IDENT) {
        p.start_node(SyntaxKind::VARIABLE);
        p.start_node(SyntaxKind::SYMBOLIC_NAME);
        p.bump();
        p.builder.finish_node();
        p.builder.finish_node();
        p.skip_trivia();
        if p.at(SyntaxKind::COLON) {
            p.start_node(SyntaxKind::NODE_LABELS);
            p.bump();
            p.skip_trivia();
            if p.at(SyntaxKind::IDENT) {
                p.start_node(SyntaxKind::NODE_LABEL);
                p.start_node(SyntaxKind::LABEL_NAME);
                p.start_node(SyntaxKind::SYMBOLIC_NAME);
                p.bump();
                p.builder.finish_node();
                p.builder.finish_node();
            }
            p.builder.finish_node();
        }
        while p.at(SyntaxKind::DOT) {
            p.start_node(SyntaxKind::PROPERTY_LOOKUP);
            p.bump();
            p.skip_trivia();
            if p.at(SyntaxKind::IDENT) {
                p.start_node(SyntaxKind::PROPERTY_KEY_NAME);
                p.start_node(SyntaxKind::SYMBOLIC_NAME);
                p.bump();
                p.builder.finish_node();
                p.builder.finish_node();
            }
            p.builder.finish_node();
            p.skip_trivia();
        }
    }
    p.builder.finish_node();
}

fn parse_pattern(p: &mut Parser) {
    p.start_node(SyntaxKind::PATTERN);
    parse_pattern_part(p);
    p.skip_trivia();
    while p.eat(SyntaxKind::COMMA) {
        p.skip_trivia();
        parse_pattern_part(p);
        p.skip_trivia();
    }
    p.builder.finish_node();
}

fn parse_pattern_part(p: &mut Parser) {
    p.start_node(SyntaxKind::PATTERN_PART);
    if p.at(SyntaxKind::IDENT) || p.at(SyntaxKind::ESCAPED_IDENT) {
        let next = p.peek_next_non_trivia();
        if next == Some(SyntaxKind::EQ) {
            p.start_node(SyntaxKind::VARIABLE);
            p.start_node(SyntaxKind::SYMBOLIC_NAME);
            p.bump();
            p.builder.finish_node();
            p.builder.finish_node();
            p.skip_trivia();
            p.bump();
            p.skip_trivia();
        }
    }
    parse_anonymous_pattern_part(p);
    p.builder.finish_node();
}

fn parse_anonymous_pattern_part(p: &mut Parser) {
    p.start_node(SyntaxKind::ANONYMOUS_PATTERN_PART);
    parse_pattern_element(p);
    p.builder.finish_node();
}

fn parse_pattern_element(p: &mut Parser) {
    p.start_node(SyntaxKind::PATTERN_ELEMENT);
    if p.at(SyntaxKind::L_PAREN) {
        // Check if this is a parenthesized PatternElement: ((a)-[r]->(b))
        // or just a regular node pattern: (a)-[r]->(b)
        // Use the same heuristic as parse_atom
        let is_nested = looks_like_nested_pattern_element(p);
        if is_nested {
            // ((a)-[r]->(b)) — outer parens wrapping inner PatternElement
            p.bump(); // outer (
            p.skip_trivia();
            // Inner could be another L_PAREN (recursive nested) or NodePattern
            if p.at(SyntaxKind::L_PAREN) {
                parse_pattern_element(p);
            } else {
                parse_node_pattern(p);
                p.skip_trivia();
                while is_relationship_chain_start(p) {
                    parse_pattern_element_chain(p);
                    p.skip_trivia();
                }
            }
            p.expect(SyntaxKind::R_PAREN);
        } else {
            // Regular node pattern with optional chains
            parse_node_pattern(p);
            p.skip_trivia();
            while is_relationship_chain_start(p) {
                parse_pattern_element_chain(p);
                p.skip_trivia();
            }
        }
    } else {
        // Not a valid pattern element start — emit error and recover
        p.error_here(&[Expected::Symbol("(")]);
    }
    p.builder.finish_node();
}

fn looks_like_nested_pattern_element(p: &Parser) -> bool {
    // After (, if next non-ws is another (, it's a nested PatternElement
    let mut lx = p.lexer.clone();
    loop {
        match lx.advance() {
            Some(tok) if tok.kind == SyntaxKind::WHITESPACE => continue,
            Some(tok) => return tok.kind == SyntaxKind::L_PAREN,
            None => return false,
        }
    }
}

fn parse_pattern_element_chain(p: &mut Parser) {
    p.start_node(SyntaxKind::PATTERN_ELEMENT_CHAIN);
    parse_relationship_pattern(p);
    p.skip_trivia();
    parse_node_pattern(p);
    p.builder.finish_node();
}

fn parse_relationship_pattern(p: &mut Parser) {
    if p.at(SyntaxKind::LT) || p.at(SyntaxKind::ARROW_LEFT) {
        p.bump();
        p.skip_trivia();
    }
    if p.at(SyntaxKind::MINUS) || p.at(SyntaxKind::DASH) {
        p.bump();
        p.skip_trivia();
    }
    if p.at(SyntaxKind::L_BRACKET) {
        parse_relationship_detail(p);
        p.skip_trivia();
    }
    if p.at(SyntaxKind::MINUS) || p.at(SyntaxKind::DASH) {
        p.bump();
        p.skip_trivia();
    }
    if p.at(SyntaxKind::GT) || p.at(SyntaxKind::ARROW_RIGHT) {
        p.bump();
        p.skip_trivia();
    }
}

fn parse_relationship_detail(p: &mut Parser) {
    p.start_node(SyntaxKind::RELATIONSHIP_DETAIL);
    p.bump();
    p.skip_trivia();
    if p.at(SyntaxKind::IDENT) || p.at(SyntaxKind::ESCAPED_IDENT) || is_keyword_as_name(p) {
        p.start_node(SyntaxKind::VARIABLE);
        p.start_node(SyntaxKind::SYMBOLIC_NAME);
        p.bump();
        p.builder.finish_node();
        p.builder.finish_node();
        p.skip_trivia();
    }
    if p.at(SyntaxKind::COLON) {
        p.start_node(SyntaxKind::RELATIONSHIP_TYPES);
        while p.at(SyntaxKind::COLON) {
            p.bump();
            p.skip_trivia();
            if p.at(SyntaxKind::IDENT) || p.at(SyntaxKind::ESCAPED_IDENT) || is_keyword_as_name(p) {
                p.start_node(SyntaxKind::REL_TYPE_NAME);
                p.start_node(SyntaxKind::SYMBOLIC_NAME);
                p.bump();
                p.builder.finish_node();
                p.builder.finish_node();
            }
            p.skip_trivia();
            if p.at(SyntaxKind::PIPE) {
                p.bump();
                p.eat(SyntaxKind::COLON);
                p.skip_trivia();
            } else {
                break;
            }
        }
        p.builder.finish_node();
    }
    if p.at(SyntaxKind::STAR) {
        p.start_node(SyntaxKind::RANGE_LITERAL);
        p.bump();
        p.skip_trivia();
        if p.at(SyntaxKind::INTEGER) {
            p.start_node(SyntaxKind::NUMBER_LITERAL);
            p.bump();
            p.builder.finish_node();
            p.skip_trivia();
        }
        if p.at(SyntaxKind::DOT_DOT) {
            p.bump();
            p.skip_trivia();
            if p.at(SyntaxKind::INTEGER) {
                p.start_node(SyntaxKind::NUMBER_LITERAL);
                p.bump();
                p.builder.finish_node();
            }
        }
        p.builder.finish_node();
        p.skip_trivia();
    }
    if p.at(SyntaxKind::L_BRACE) {
        p.start_node(SyntaxKind::PROPERTIES);
        p.start_node(SyntaxKind::MAP_LITERAL);
        p.bump();
        p.skip_trivia();
        if !p.at(SyntaxKind::R_BRACE) {
            parse_map_entry(p);
            p.skip_trivia();
            while p.eat(SyntaxKind::COMMA) {
                p.skip_trivia();
                parse_map_entry(p);
                p.skip_trivia();
            }
        }
        p.expect(SyntaxKind::R_BRACE);
        p.builder.finish_node();
        p.builder.finish_node();
        p.skip_trivia();
    }
    p.expect(SyntaxKind::R_BRACKET);
    p.builder.finish_node();
}

fn parse_node_pattern(p: &mut Parser) {
    p.start_node(SyntaxKind::NODE_PATTERN);
    p.bump();
    p.skip_trivia();
    if p.at(SyntaxKind::IDENT) || p.at(SyntaxKind::ESCAPED_IDENT) || is_keyword_as_name(p) {
        p.start_node(SyntaxKind::VARIABLE);
        p.start_node(SyntaxKind::SYMBOLIC_NAME);
        p.bump();
        p.builder.finish_node();
        p.builder.finish_node();
        p.skip_trivia();
    }
    while p.at(SyntaxKind::COLON) {
        p.start_node(SyntaxKind::NODE_LABELS);
        p.bump();
        p.skip_trivia();
        if p.at(SyntaxKind::IDENT) || p.at(SyntaxKind::ESCAPED_IDENT) || is_keyword_as_name(p) {
            p.start_node(SyntaxKind::NODE_LABEL);
            p.start_node(SyntaxKind::LABEL_NAME);
            p.start_node(SyntaxKind::SYMBOLIC_NAME);
            p.bump();
            p.builder.finish_node();
            p.builder.finish_node();
            p.builder.finish_node();
        }
        p.builder.finish_node();
        p.skip_trivia();
    }
    if p.at(SyntaxKind::L_BRACE) {
        p.start_node(SyntaxKind::PROPERTIES);
        p.start_node(SyntaxKind::MAP_LITERAL);
        p.bump();
        p.skip_trivia();
        if !p.at(SyntaxKind::R_BRACE) {
            parse_map_entry(p);
            p.skip_trivia();
            while p.eat(SyntaxKind::COMMA) {
                p.skip_trivia();
                parse_map_entry(p);
                p.skip_trivia();
            }
        }
        p.expect(SyntaxKind::R_BRACE);
        p.builder.finish_node();
        p.builder.finish_node();
        p.skip_trivia();
    }
    p.expect(SyntaxKind::R_PAREN);
    p.builder.finish_node();
}

fn parse_projection_body(p: &mut Parser) {
    p.start_node(SyntaxKind::PROJECTION_BODY);
    p.skip_trivia();
    p.eat(SyntaxKind::KW_DISTINCT);
    p.skip_trivia();
    parse_projection_items(p);
    p.skip_trivia();
    if p.at(SyntaxKind::KW_ORDER) {
        p.start_node(SyntaxKind::ORDER_BY);
        p.bump();
        p.skip_trivia();
        p.expect(SyntaxKind::KW_BY);
        p.skip_trivia();
        parse_sort_item(p);
        while p.eat(SyntaxKind::COMMA) {
            p.skip_trivia();
            parse_sort_item(p);
        }
        p.builder.finish_node();
        p.skip_trivia();
    }
    if p.at(SyntaxKind::KW_SKIP) {
        p.start_node(SyntaxKind::SKIP_CLAUSE);
        p.bump();
        p.skip_trivia();
        expr_bp(p, Prec::MIN);
        p.builder.finish_node();
        p.skip_trivia();
    }
    if p.at(SyntaxKind::KW_LIMIT) {
        p.start_node(SyntaxKind::LIMIT_CLAUSE);
        p.bump();
        p.skip_trivia();
        expr_bp(p, Prec::MIN);
        p.builder.finish_node();
        p.skip_trivia();
    }
    p.builder.finish_node();
}

fn parse_projection_items(p: &mut Parser) {
    p.start_node(SyntaxKind::PROJECTION_ITEMS);
    if p.at(SyntaxKind::STAR) {
        p.bump();
        p.skip_trivia();
        if p.at(SyntaxKind::COMMA) {
            p.bump();
            p.skip_trivia();
            parse_projection_item(p);
            while p.eat(SyntaxKind::COMMA) {
                p.skip_trivia();
                parse_projection_item(p);
                p.skip_trivia();
            }
        }
    } else if p.at(SyntaxKind::KW_ORDER)
        || p.at(SyntaxKind::KW_SKIP)
        || p.at(SyntaxKind::KW_LIMIT)
        || p.at(SyntaxKind::SEMICOLON)
        || p.at(SyntaxKind::KW_UNION)
        || p.at(SyntaxKind::KW_END)
        || p.current_len() == 0
    {
        // No projection items — skip creating empty item nodes
    } else {
        parse_projection_item(p);
        p.skip_trivia();
        while p.eat(SyntaxKind::COMMA) {
            p.skip_trivia();
            parse_projection_item(p);
            p.skip_trivia();
        }
    }
    p.builder.finish_node();
}

fn parse_projection_item(p: &mut Parser) {
    p.start_node(SyntaxKind::PROJECTION_ITEM);
    expr_bp(p, Prec::MIN);
    p.skip_trivia();
    if p.at(SyntaxKind::KW_AS) {
        p.bump();
        p.skip_trivia();
        if p.at(SyntaxKind::IDENT) || p.at(SyntaxKind::ESCAPED_IDENT) {
            p.start_node(SyntaxKind::VARIABLE);
            p.start_node(SyntaxKind::SYMBOLIC_NAME);
            p.bump();
            p.builder.finish_node();
            p.builder.finish_node();
        }
    }
    p.builder.finish_node();
}

fn parse_sort_item(p: &mut Parser) {
    p.start_node(SyntaxKind::SORT_ITEM);
    expr_bp(p, Prec::MIN);
    p.skip_trivia();
    if p.at(SyntaxKind::KW_ASCENDING)
        || p.at(SyntaxKind::KW_ASC)
        || p.at(SyntaxKind::KW_DESCENDING)
        || p.at(SyntaxKind::KW_DESC)
    {
        p.bump();
    }
    p.builder.finish_node();
}

fn is_relationship_chain_start(p: &Parser) -> bool {
    matches!(
        p.current_kind(),
        SyntaxKind::LT
            | SyntaxKind::GT
            | SyntaxKind::ARROW_LEFT
            | SyntaxKind::ARROW_RIGHT
            | SyntaxKind::MINUS
            | SyntaxKind::DASH
    )
}

// ── FOREACH clause ──────────────────────────────────────────────

fn parse_foreach_clause(p: &mut Parser) {
    p.start_node(SyntaxKind::FOREACH_CLAUSE);
    p.bump(); // FOREACH
    p.skip_trivia();
    // FOREACH (variable IN list | clauses )
    if p.at(SyntaxKind::L_PAREN) {
        p.bump();
        p.skip_trivia();
        // variable
        if p.at(SyntaxKind::IDENT) || p.at(SyntaxKind::ESCAPED_IDENT) {
            p.start_node(SyntaxKind::VARIABLE);
            p.start_node(SyntaxKind::SYMBOLIC_NAME);
            p.bump();
            p.builder.finish_node();
            p.builder.finish_node();
        }
        p.skip_trivia();
        // IN
        p.expect(SyntaxKind::KW_IN);
        p.skip_trivia();
        // list expression
        expr_bp(p, Prec::MIN);
        p.skip_trivia();
        // PIPE
        p.expect(SyntaxKind::PIPE);
        p.skip_trivia();
        // nested clauses
        while !p.at(SyntaxKind::R_PAREN) && p.current_len() > 0 && is_clause_start_in_foreach(p) {
            parse_clause(p);
            p.skip_trivia();
        }
        p.expect(SyntaxKind::R_PAREN);
    }
    p.builder.finish_node();
}

fn is_clause_start_in_foreach(p: &Parser) -> bool {
    matches!(
        p.current_kind(),
        SyntaxKind::KW_MATCH
            | SyntaxKind::KW_OPTIONAL
            | SyntaxKind::KW_CREATE
            | SyntaxKind::KW_MERGE
            | SyntaxKind::KW_DELETE
            | SyntaxKind::KW_DETACH
            | SyntaxKind::KW_SET
            | SyntaxKind::KW_REMOVE
            | SyntaxKind::KW_FOREACH
    )
}

// ── CALL clause (procedures) ────────────────────────────────────

fn parse_call_clause(p: &mut Parser) {
    p.skip_trivia();
    // Check if this is CALL { ... } (subquery) or CALL proc() (procedure)
    let next = p.peek_next_non_trivia();
    if next == Some(SyntaxKind::L_BRACE) {
        parse_call_subquery(p);
    } else {
        parse_procedure_call(p);
    }
}

fn parse_procedure_call(p: &mut Parser) {
    // Determine if standalone or in-query based on context
    // For now, parse as standalone call
    p.start_node(SyntaxKind::STANDALONE_CALL);
    p.start_node(SyntaxKind::IMPLICIT_PROCEDURE_INVOCATION);
    parse_procedure_name(p);
    p.skip_trivia();
    if p.at(SyntaxKind::L_PAREN) {
        // Explicit: CALL proc(args)
        p.start_node(SyntaxKind::EXPLICIT_PROCEDURE_INVOCATION);
        p.bump(); // L_PAREN
        p.skip_trivia();
        // Arguments
        if !p.at(SyntaxKind::R_PAREN) {
            expr_bp(p, Prec::MIN);
            p.skip_trivia();
            while p.eat(SyntaxKind::COMMA) {
                p.skip_trivia();
                expr_bp(p, Prec::MIN);
                p.skip_trivia();
            }
        }
        p.expect(SyntaxKind::R_PAREN);
        p.builder.finish_node();
    }
    // else: Implicit — no parens, just the procedure name
    p.builder.finish_node();
    p.skip_trivia();
    // Optional YIELD
    if p.at(SyntaxKind::KW_YIELD) {
        p.skip_trivia();
        parse_yield_items(p);
    }
    p.builder.finish_node();
}

fn parse_procedure_invocation(p: &mut Parser) {
    p.start_node(SyntaxKind::EXPLICIT_PROCEDURE_INVOCATION);
    parse_procedure_name(p);
    p.skip_trivia();
    p.expect(SyntaxKind::L_PAREN);
    p.skip_trivia();
    // Arguments
    if !p.at(SyntaxKind::R_PAREN) {
        expr_bp(p, Prec::MIN);
        p.skip_trivia();
        while p.eat(SyntaxKind::COMMA) {
            p.skip_trivia();
            expr_bp(p, Prec::MIN);
            p.skip_trivia();
        }
    }
    p.expect(SyntaxKind::R_PAREN);
    p.builder.finish_node();
}

fn parse_procedure_name(p: &mut Parser) {
    p.start_node(SyntaxKind::PROCEDURE_NAME);
    // Namespace parts: e.g., db.labels
    loop {
        p.start_node(SyntaxKind::SYMBOLIC_NAME);
        if p.at(SyntaxKind::IDENT) || p.at(SyntaxKind::ESCAPED_IDENT) || is_keyword_as_name(p) {
            p.bump();
        } else {
            p.start_node(SyntaxKind::ERROR);
            p.builder.finish_node();
        }
        p.builder.finish_node();
        p.skip_trivia();
        if p.at(SyntaxKind::DOT) {
            p.bump();
            p.skip_trivia();
        } else {
            break;
        }
    }
    p.builder.finish_node();
}

fn parse_yield_items(p: &mut Parser) {
    p.start_node(SyntaxKind::YIELD_ITEMS);
    // YIELD * or YIELD field1, field2
    if p.at(SyntaxKind::STAR) {
        p.bump();
    } else {
        parse_yield_item(p);
        p.skip_trivia();
        while p.eat(SyntaxKind::COMMA) {
            p.skip_trivia();
            parse_yield_item(p);
            p.skip_trivia();
        }
    }
    p.skip_trivia();
    // Optional WHERE
    if p.at(SyntaxKind::KW_WHERE) {
        p.bump();
        p.skip_trivia();
        expr_bp(p, Prec::MIN);
        p.skip_trivia();
    }
    p.builder.finish_node();
}

fn parse_yield_item(p: &mut Parser) {
    p.start_node(SyntaxKind::YIELD_ITEM);
    // procedure field
    p.start_node(SyntaxKind::PROCEDURE_RESULT_FIELD);
    p.start_node(SyntaxKind::SYMBOLIC_NAME);
    if p.at(SyntaxKind::IDENT) || p.at(SyntaxKind::ESCAPED_IDENT) || is_keyword_as_name(p) {
        p.bump();
    }
    p.builder.finish_node();
    p.builder.finish_node();
    p.skip_trivia();
    // Optional AS alias
    if p.at(SyntaxKind::KW_AS) {
        p.bump();
        p.skip_trivia();
        if p.at(SyntaxKind::IDENT) || p.at(SyntaxKind::ESCAPED_IDENT) {
            p.start_node(SyntaxKind::VARIABLE);
            p.start_node(SyntaxKind::SYMBOLIC_NAME);
            p.bump();
            p.builder.finish_node();
            p.builder.finish_node();
        }
    }
    p.builder.finish_node();
}

fn parse_yield_clause(p: &mut Parser) {
    // Standalone YIELD (in-query call variant)
    p.start_node(SyntaxKind::IN_QUERY_CALL);
    parse_yield_items(p);
    p.builder.finish_node();
}

// ── CALL SUBQUERY ───────────────────────────────────────────────

fn parse_call_subquery(p: &mut Parser) {
    p.start_node(SyntaxKind::CALL_SUBQUERY_CLAUSE);
    p.bump(); // CALL
    p.skip_trivia();
    // { subquery }
    if p.at(SyntaxKind::L_BRACE) {
        p.bump();
        p.skip_trivia();
        // Parse inner query as RegularQuery (supports nested UNION)
        parse_regular_query_body(p);
        p.expect(SyntaxKind::R_BRACE);
    }
    p.skip_trivia();
    // Optional IN TRANSACTIONS
    if p.at(SyntaxKind::KW_IN) {
        let next = p.peek_next_non_trivia();
        if next == Some(SyntaxKind::KW_TRANSACTIONS) {
            p.start_node(SyntaxKind::IN_TRANSACTIONS);
            p.bump(); // IN
            p.skip_trivia();
            p.expect(SyntaxKind::KW_TRANSACTIONS);
            p.skip_trivia();
            // Optional OF <n> ROWS
            if p.at_keyword(SyntaxKind::KW_OF) {
                p.bump();
                p.skip_trivia();
                if p.at(SyntaxKind::INTEGER) {
                    p.start_node(SyntaxKind::NUMBER_LITERAL);
                    p.bump();
                    p.builder.finish_node();
                    p.skip_trivia();
                }
                p.expect(SyntaxKind::KW_ROWS);
            }
            p.skip_trivia();
            // Optional ON ERROR {CONTINUE|BREAK|FAIL}
            if p.at_keyword(SyntaxKind::KW_ON) {
                let after_on = p.peek_next_non_trivia();
                if after_on == Some(SyntaxKind::KW_ERROR) {
                    p.bump(); // ON
                    p.skip_trivia();
                    p.expect(SyntaxKind::KW_ERROR);
                    p.skip_trivia();
                    if p.at(SyntaxKind::KW_CONTINUE)
                        || p.at(SyntaxKind::KW_BREAK)
                        || p.at(SyntaxKind::KW_FAIL)
                    {
                        p.bump();
                    }
                }
            }
            p.builder.finish_node();
        }
    }
    p.builder.finish_node();
}

fn parse_regular_query_body(p: &mut Parser) {
    // Parse a RegularQuery: SinglePartQuery ( UNION SinglePartQuery )*
    // A SinglePartQuery is: ReadingClause* UpdatingClause?
    // Simplified: parse clauses until UNION or } or end
    let mut has_clauses = false;
    loop {
        p.skip_trivia();
        if p.at(SyntaxKind::R_BRACE) || p.current_len() == 0 {
            break;
        }
        if p.at(SyntaxKind::KW_UNION) {
            // Handled by outer loop
            break;
        }
        if is_clause_start_for_subquery(p) {
            has_clauses = true;
            parse_clause(p);
        } else if p.at(SyntaxKind::KW_YIELD) {
            // YIELD can appear in subquery context
            has_clauses = true;
            p.start_node(SyntaxKind::IN_QUERY_CALL);
            parse_yield_items(p);
            p.builder.finish_node();
        } else {
            p.start_node(SyntaxKind::ERROR);
            p.bump();
            p.builder.finish_node();
        }
        p.skip_trivia();
    }
    // Handle UNION chains
    while p.at(SyntaxKind::KW_UNION) {
        p.start_node(SyntaxKind::UNION);
        p.bump();
        p.skip_trivia();
        p.eat(SyntaxKind::KW_ALL);
        p.skip_trivia();
        // Parse next query body
        parse_regular_query_body(p);
        p.builder.finish_node();
        p.skip_trivia();
    }
    let _ = has_clauses;
}

fn is_clause_start_for_subquery(p: &Parser) -> bool {
    matches!(
        p.current_kind(),
        SyntaxKind::KW_MATCH
            | SyntaxKind::KW_RETURN
            | SyntaxKind::KW_WITH
            | SyntaxKind::KW_UNWIND
            | SyntaxKind::KW_CREATE
            | SyntaxKind::KW_MERGE
            | SyntaxKind::KW_DELETE
            | SyntaxKind::KW_SET
            | SyntaxKind::KW_REMOVE
            | SyntaxKind::KW_CALL
            | SyntaxKind::KW_FOREACH
            | SyntaxKind::KW_OPTIONAL
            | SyntaxKind::KW_DETACH
            | SyntaxKind::KW_YIELD
    )
}

// ── Schema commands ─────────────────────────────────────────────

fn parse_create_index(p: &mut Parser) {
    p.start_node(SyntaxKind::CREATE_INDEX);
    p.bump(); // CREATE
    p.skip_trivia();
    // Optional index type: LOOKUP, TEXT, RANGE, POINT, FULLTEXT
    if p.at_keyword(SyntaxKind::KW_LOOKUP)
        || p.at_keyword(SyntaxKind::KW_TEXT)
        || p.at_keyword(SyntaxKind::KW_RANGE)
        || p.at_keyword(SyntaxKind::KW_POINT)
        || p.at_keyword(SyntaxKind::KW_FULLTEXT)
    {
        p.start_node(SyntaxKind::INDEX_KIND);
        p.bump();
        p.builder.finish_node();
        p.skip_trivia();
    }
    p.expect(SyntaxKind::KW_INDEX);
    p.skip_trivia();
    // Optional IF NOT EXISTS
    if p.at_keyword(SyntaxKind::KW_IF) {
        p.bump();
        p.skip_trivia();
        p.eat(SyntaxKind::KW_NOT);
        p.skip_trivia();
        p.eat(SyntaxKind::KW_EXISTS);
        p.skip_trivia();
    }
    // Optional index name
    if p.at(SyntaxKind::IDENT) || p.at(SyntaxKind::ESCAPED_IDENT) || is_keyword_as_name(p) {
        p.start_node(SyntaxKind::SCHEMA_NAME);
        p.start_node(SyntaxKind::SYMBOLIC_NAME);
        p.bump();
        p.builder.finish_node();
        p.builder.finish_node();
        p.skip_trivia();
    }
    // FOR pattern
    if p.at_keyword(SyntaxKind::KW_FOR) {
        p.bump();
        p.skip_trivia();
        parse_index_pattern(p);
        p.skip_trivia();
    }
    // ON or ON EACH
    if p.at_keyword(SyntaxKind::KW_ON) {
        p.bump();
        p.skip_trivia();
        p.eat(SyntaxKind::KW_EACH);
        p.skip_trivia();
        // Properties
        if p.at(SyntaxKind::L_PAREN) {
            p.start_node(SyntaxKind::PROPERTIES);
            parse_properties_expression(p);
            p.builder.finish_node();
        } else if p.at(SyntaxKind::L_BRACKET) {
            p.start_node(SyntaxKind::PROPERTIES);
            p.start_node(SyntaxKind::LIST_LITERAL);
            p.bump();
            p.skip_trivia();
            if !p.at(SyntaxKind::R_BRACKET) {
                parse_properties_expression(p);
                p.skip_trivia();
                while p.eat(SyntaxKind::COMMA) {
                    p.skip_trivia();
                    parse_properties_expression(p);
                    p.skip_trivia();
                }
            }
            p.expect(SyntaxKind::R_BRACKET);
            p.builder.finish_node();
            p.builder.finish_node();
        }
        p.skip_trivia();
    }
    // Optional OPTIONS
    if p.at_keyword(SyntaxKind::KW_OPTIONS) {
        p.bump();
        p.skip_trivia();
        parse_options_clause(p);
        p.skip_trivia();
    }
    p.builder.finish_node();
}

fn parse_index_pattern(p: &mut Parser) {
    // (variable:Label) or ()-[variable:REL]-()
    if p.at(SyntaxKind::L_PAREN) {
        p.start_node(SyntaxKind::NODE_PATTERN);
        p.bump();
        p.skip_trivia();
        // Optional variable
        if p.at(SyntaxKind::IDENT) || p.at(SyntaxKind::ESCAPED_IDENT) {
            p.start_node(SyntaxKind::VARIABLE);
            p.start_node(SyntaxKind::SYMBOLIC_NAME);
            p.bump();
            p.builder.finish_node();
            p.builder.finish_node();
            p.skip_trivia();
        }
        // Optional labels
        while p.at(SyntaxKind::COLON) {
            p.start_node(SyntaxKind::NODE_LABELS);
            p.bump();
            p.skip_trivia();
            if p.at(SyntaxKind::IDENT) || p.at(SyntaxKind::ESCAPED_IDENT) {
                p.start_node(SyntaxKind::NODE_LABEL);
                p.start_node(SyntaxKind::LABEL_NAME);
                p.start_node(SyntaxKind::SYMBOLIC_NAME);
                p.bump();
                p.builder.finish_node();
                p.builder.finish_node();
                p.builder.finish_node();
            }
            p.builder.finish_node();
            p.skip_trivia();
        }
        p.expect(SyntaxKind::R_PAREN);
        p.builder.finish_node();
        p.skip_trivia();
        // Check for relationship pattern: ()-[r:REL]-()
        if p.at(SyntaxKind::MINUS) || p.at(SyntaxKind::DASH) {
            p.bump();
            p.skip_trivia();
            if p.at(SyntaxKind::L_BRACKET) {
                parse_relationship_detail(p);
                p.skip_trivia();
            }
            if p.at(SyntaxKind::MINUS) || p.at(SyntaxKind::DASH) {
                p.bump();
                p.skip_trivia();
            }
            if p.at(SyntaxKind::L_PAREN) {
                p.start_node(SyntaxKind::NODE_PATTERN);
                p.bump();
                p.skip_trivia();
                p.expect(SyntaxKind::R_PAREN);
                p.builder.finish_node();
            }
        }
    }
}

fn parse_properties_expression(p: &mut Parser) {
    // e.g., (n.property) or labels(n) or properties(n)
    expr_bp(p, Prec::MIN);
}

fn parse_options_clause(p: &mut Parser) {
    p.start_node(SyntaxKind::OPTIONS_CLAUSE);
    if p.at(SyntaxKind::L_BRACE) {
        p.start_node(SyntaxKind::MAP_LITERAL);
        p.bump();
        p.skip_trivia();
        if !p.at(SyntaxKind::R_BRACE) {
            parse_map_entry(p);
            p.skip_trivia();
            while p.eat(SyntaxKind::COMMA) {
                p.skip_trivia();
                parse_map_entry(p);
                p.skip_trivia();
            }
        }
        p.expect(SyntaxKind::R_BRACE);
        p.builder.finish_node();
    }
    p.builder.finish_node();
}

fn parse_create_constraint(p: &mut Parser) {
    p.start_node(SyntaxKind::CREATE_CONSTRAINT);
    p.bump(); // CREATE
    p.skip_trivia();
    // Optional constraint type: UNIQUE, NODE KEY, EXISTENCE, IS TYPED
    if is_constraint_kind(p) {
        p.start_node(SyntaxKind::CONSTRAINT_KIND);
        if p.at(SyntaxKind::KW_UNIQUE) {
            p.bump();
        } else if p.at(SyntaxKind::KW_NODE) {
            p.bump();
            p.skip_trivia();
            p.expect(SyntaxKind::KW_KEY);
        } else if p.at_keyword(SyntaxKind::KW_IS) {
            p.bump();
            p.skip_trivia();
            if p.at(SyntaxKind::KW_TYPE) {
                p.bump();
            }
        }
        p.builder.finish_node();
        p.skip_trivia();
    }
    p.expect(SyntaxKind::KW_CONSTRAINT);
    p.skip_trivia();
    // Optional constraint name
    if p.at(SyntaxKind::IDENT) || p.at(SyntaxKind::ESCAPED_IDENT) {
        p.start_node(SyntaxKind::SCHEMA_NAME);
        p.start_node(SyntaxKind::SYMBOLIC_NAME);
        p.bump();
        p.builder.finish_node();
        p.builder.finish_node();
        p.skip_trivia();
    }
    // FOR pattern
    if p.at_keyword(SyntaxKind::KW_FOR) {
        p.bump();
        p.skip_trivia();
        parse_index_pattern(p);
        p.skip_trivia();
    }
    // REQUIRE
    if p.at_keyword(SyntaxKind::KW_REQUIRE) {
        p.bump();
        p.skip_trivia();
        // Parse constraint expression: prop IS UNIQUE, prop IS NOT NULL, etc.
        parse_constraint_expression(p);
        p.skip_trivia();
    }
    // Optional OPTIONS
    if p.at_keyword(SyntaxKind::KW_OPTIONS) {
        p.bump();
        p.skip_trivia();
        parse_options_clause(p);
        p.skip_trivia();
    }
    p.builder.finish_node();
}

fn parse_constraint_expression(p: &mut Parser) {
    // Parse: property_expression IS [NOT] (UNIQUE|NULL|TYPED)
    // Parse property chain directly to avoid expr_bp consuming IS as IS NULL
    parse_constraint_property(p);
    p.skip_trivia();
    // IS [NOT] constraint_type
    if p.at(SyntaxKind::KW_IS) {
        p.start_node(SyntaxKind::CONSTRAINT_KIND);
        p.bump(); // IS
        p.skip_trivia();
        p.eat(SyntaxKind::KW_NOT);
        p.skip_trivia();
        if p.at(SyntaxKind::KW_UNIQUE) || p.at(SyntaxKind::NULL_KW) || p.at(SyntaxKind::KW_TYPE) {
            p.bump();
            // PROPERTY TYPE IS (type | type | ...)
            if p.at(SyntaxKind::L_PAREN) {
                p.bump();
                p.skip_trivia();
                if p.at(SyntaxKind::IDENT)
                    || p.at(SyntaxKind::ESCAPED_IDENT)
                    || is_keyword_as_name(p)
                {
                    p.start_node(SyntaxKind::SYMBOLIC_NAME);
                    p.bump();
                    p.builder.finish_node();
                    p.skip_trivia();
                }
                while p.at(SyntaxKind::PIPE) {
                    p.bump();
                    p.skip_trivia();
                    if p.at(SyntaxKind::IDENT)
                        || p.at(SyntaxKind::ESCAPED_IDENT)
                        || is_keyword_as_name(p)
                    {
                        p.start_node(SyntaxKind::SYMBOLIC_NAME);
                        p.bump();
                        p.builder.finish_node();
                        p.skip_trivia();
                    }
                }
                p.expect(SyntaxKind::R_PAREN);
            }
        }
        p.builder.finish_node();
    }
}

fn parse_constraint_property(p: &mut Parser) {
    // Parse variable.property chain
    if p.at(SyntaxKind::IDENT) || p.at(SyntaxKind::ESCAPED_IDENT) {
        p.start_node(SyntaxKind::VARIABLE);
        p.start_node(SyntaxKind::SYMBOLIC_NAME);
        p.bump();
        p.builder.finish_node();
        p.builder.finish_node();
        p.skip_trivia();
        while p.at(SyntaxKind::DOT) {
            p.start_node(SyntaxKind::PROPERTY_LOOKUP);
            p.bump();
            p.skip_trivia();
            if p.at(SyntaxKind::IDENT) || p.at(SyntaxKind::ESCAPED_IDENT) {
                p.start_node(SyntaxKind::PROPERTY_KEY_NAME);
                p.start_node(SyntaxKind::SYMBOLIC_NAME);
                p.bump();
                p.builder.finish_node();
                p.builder.finish_node();
            }
            p.builder.finish_node();
            p.skip_trivia();
        }
    }
}

fn is_constraint_kind(p: &Parser) -> bool {
    matches!(
        p.current_kind(),
        SyntaxKind::KW_UNIQUE | SyntaxKind::KW_NODE | SyntaxKind::KW_IS
    )
}

fn parse_create_database(p: &mut Parser) {
    p.start_node(SyntaxKind::SCHEMA_COMMAND);
    p.bump(); // CREATE
    p.skip_trivia();
    // DATABASE or DATABASES
    if p.at_keyword(SyntaxKind::KW_DATABASE) {
        p.bump();
    } else if p.at_keyword(SyntaxKind::KW_DATABASES) {
        p.bump();
    }
    p.skip_trivia();
    // Database name
    if p.at(SyntaxKind::IDENT) || p.at(SyntaxKind::ESCAPED_IDENT) {
        p.start_node(SyntaxKind::SCHEMA_NAME);
        p.start_node(SyntaxKind::SYMBOLIC_NAME);
        p.bump();
        p.builder.finish_node();
        p.builder.finish_node();
    }
    p.skip_trivia();
    // Optional IF NOT EXISTS
    if p.at_keyword(SyntaxKind::KW_IF) {
        p.bump();
        p.skip_trivia();
        p.eat(SyntaxKind::KW_NOT);
        p.skip_trivia();
        p.eat(SyntaxKind::KW_EXISTS);
    }
    p.builder.finish_node();
}

fn parse_drop_clause(p: &mut Parser) {
    p.skip_trivia();
    let next = p.peek_next_non_trivia();
    match next {
        Some(SyntaxKind::KW_INDEX) => parse_drop_index(p),
        Some(SyntaxKind::KW_CONSTRAINT) => parse_drop_constraint(p),
        Some(SyntaxKind::KW_DATABASE) | Some(SyntaxKind::KW_DATABASES) => parse_drop_database(p),
        _ => {
            // Unknown DROP - eat it
            p.start_node(SyntaxKind::ERROR);
            p.bump();
            p.builder.finish_node();
        }
    }
}

fn parse_drop_index(p: &mut Parser) {
    p.start_node(SyntaxKind::DROP_INDEX);
    p.bump(); // DROP
    p.skip_trivia();
    p.expect(SyntaxKind::KW_INDEX);
    p.skip_trivia();
    // Optional CONCURRENTLY
    if p.at_keyword(SyntaxKind::KW_CONCURRENTLY) {
        p.bump();
        p.skip_trivia();
    }
    // Index name
    if p.at(SyntaxKind::IDENT) || p.at(SyntaxKind::ESCAPED_IDENT) {
        p.start_node(SyntaxKind::SCHEMA_NAME);
        p.start_node(SyntaxKind::SYMBOLIC_NAME);
        p.bump();
        p.builder.finish_node();
        p.builder.finish_node();
    }
    p.skip_trivia();
    // Optional IF EXISTS
    if p.at_keyword(SyntaxKind::KW_IF) {
        p.bump();
        p.skip_trivia();
        p.eat(SyntaxKind::KW_EXISTS);
    }
    p.builder.finish_node();
}

fn parse_drop_constraint(p: &mut Parser) {
    p.start_node(SyntaxKind::DROP_CONSTRAINT);
    p.bump(); // DROP
    p.skip_trivia();
    p.expect(SyntaxKind::KW_CONSTRAINT);
    p.skip_trivia();
    // Optional CONCURRENTLY
    if p.at_keyword(SyntaxKind::KW_CONCURRENTLY) {
        p.bump();
        p.skip_trivia();
    }
    // Constraint name
    if p.at(SyntaxKind::IDENT) || p.at(SyntaxKind::ESCAPED_IDENT) {
        p.start_node(SyntaxKind::SCHEMA_NAME);
        p.start_node(SyntaxKind::SYMBOLIC_NAME);
        p.bump();
        p.builder.finish_node();
        p.builder.finish_node();
    }
    p.skip_trivia();
    // Optional IF EXISTS
    if p.at_keyword(SyntaxKind::KW_IF) {
        p.bump();
        p.skip_trivia();
        p.eat(SyntaxKind::KW_EXISTS);
    }
    p.builder.finish_node();
}

fn parse_drop_database(p: &mut Parser) {
    p.start_node(SyntaxKind::SCHEMA_COMMAND);
    p.bump(); // DROP
    p.skip_trivia();
    // DATABASE or DATABASES
    if p.at_keyword(SyntaxKind::KW_DATABASE) {
        p.bump();
    } else if p.at_keyword(SyntaxKind::KW_DATABASES) {
        p.bump();
    }
    p.skip_trivia();
    // Database name
    if p.at(SyntaxKind::IDENT) || p.at(SyntaxKind::ESCAPED_IDENT) {
        p.start_node(SyntaxKind::SCHEMA_NAME);
        p.start_node(SyntaxKind::SYMBOLIC_NAME);
        p.bump();
        p.builder.finish_node();
        p.builder.finish_node();
    }
    p.skip_trivia();
    // Optional IF EXISTS
    if p.at_keyword(SyntaxKind::KW_IF) {
        p.bump();
        p.skip_trivia();
        p.eat(SyntaxKind::KW_EXISTS);
    }
    p.builder.finish_node();
}

// ── SHOW clause ─────────────────────────────────────────────────

fn parse_show_clause(p: &mut Parser) {
    p.start_node(SyntaxKind::SHOW_CLAUSE);
    p.bump(); // SHOW
    p.skip_trivia();
    // SHOW kind: INDEXES, CONSTRAINTS, DATABASES, PROCEDURES, FUNCTIONS, etc.
    parse_show_kind(p);
    p.skip_trivia();
    // Optional YIELD
    if p.at(SyntaxKind::KW_YIELD) {
        p.bump();
        p.skip_trivia();
        parse_show_yield(p);
    }
    // Optional trailing RETURN <projection>
    if p.at(SyntaxKind::KW_RETURN) {
        parse_return_clause(p);
    }
    p.builder.finish_node();
}

fn parse_show_kind(p: &mut Parser) {
    p.start_node(SyntaxKind::SHOW_KIND);
    if p.at(SyntaxKind::KW_INDEX) || p.at(SyntaxKind::KW_INDEXES) {
        p.bump();
    } else if p.at(SyntaxKind::KW_CONSTRAINT) || p.at(SyntaxKind::KW_CONSTRAINTS) {
        p.bump();
    } else if p.at_keyword(SyntaxKind::KW_DATABASE) || p.at_keyword(SyntaxKind::KW_DATABASES) {
        p.bump();
    } else if p.at_keyword(SyntaxKind::KW_PROCEDURES) {
        p.bump();
    } else if p.at_keyword(SyntaxKind::KW_FUNCTIONS) {
        p.bump();
    } else if p.at_keyword(SyntaxKind::KW_TYPES) {
        p.bump();
    } else if p.at_keyword(SyntaxKind::KW_PROPERTY) {
        p.bump();
        p.skip_trivia();
        if p.at_keyword(SyntaxKind::KW_GRAPH) {
            p.bump();
            p.skip_trivia();
            if p.at(SyntaxKind::L_PAREN) {
                p.bump();
                p.skip_trivia();
                if p.at(SyntaxKind::IDENT) || p.at(SyntaxKind::ESCAPED_IDENT) {
                    p.start_node(SyntaxKind::VARIABLE);
                    p.start_node(SyntaxKind::SYMBOLIC_NAME);
                    p.bump();
                    p.builder.finish_node();
                    p.builder.finish_node();
                }
                p.skip_trivia();
                p.expect(SyntaxKind::R_PAREN);
            }
        }
    } else if p.at_keyword(SyntaxKind::KW_ACCESS) {
        p.bump();
        p.skip_trivia();
        p.eat(SyntaxKind::KW_FOR);
        p.skip_trivia();
        if p.at(SyntaxKind::IDENT) || p.at(SyntaxKind::ESCAPED_IDENT) {
            p.start_node(SyntaxKind::VARIABLE);
            p.start_node(SyntaxKind::SYMBOLIC_NAME);
            p.bump();
            p.builder.finish_node();
            p.builder.finish_node();
        }
    } else {
        // Generic - just bump the token
        if p.current_len() > 0 {
            p.bump();
        }
    }
    p.builder.finish_node();
}

fn parse_show_yield(p: &mut Parser) {
    p.start_node(SyntaxKind::SHOW_RETURN);
    if p.at(SyntaxKind::STAR) {
        p.bump();
    } else {
        parse_yield_item(p);
        p.skip_trivia();
        while p.eat(SyntaxKind::COMMA) {
            p.skip_trivia();
            parse_yield_item(p);
            p.skip_trivia();
        }
    }
    p.skip_trivia();
    // Optional WHERE
    if p.at(SyntaxKind::KW_WHERE) {
        p.bump();
        p.skip_trivia();
        expr_bp(p, Prec::MIN);
    }
    p.builder.finish_node();
}

// ── USE clause ──────────────────────────────────────────────────

fn parse_use_clause(p: &mut Parser) {
    p.start_node(SyntaxKind::USE_CLAUSE);
    p.bump(); // USE
    p.skip_trivia();
    // Database name
    if p.at(SyntaxKind::IDENT) || p.at(SyntaxKind::ESCAPED_IDENT) {
        p.start_node(SyntaxKind::SCHEMA_NAME);
        p.start_node(SyntaxKind::SYMBOLIC_NAME);
        p.bump();
        p.builder.finish_node();
        p.builder.finish_node();
    }
    p.builder.finish_node();
}

// ── Helpers ─────────────────────────────────────────────────────

fn is_keyword_as_name(p: &Parser) -> bool {
    matches!(
        p.current_kind(),
        SyntaxKind::KW_ACCESS
            | SyntaxKind::KW_ADD
            | SyntaxKind::KW_ALL
            | SyntaxKind::KW_AND
            | SyntaxKind::KW_ANY
            | SyntaxKind::KW_AS
            | SyntaxKind::KW_ASC
            | SyntaxKind::KW_ASCENDING
            | SyntaxKind::KW_BREAK
            | SyntaxKind::KW_BY
            | SyntaxKind::KW_CALL
            | SyntaxKind::KW_CASE
            | SyntaxKind::KW_CONTAINS
            | SyntaxKind::KW_CONTINUE
            | SyntaxKind::KW_CONSTRAINT
            | SyntaxKind::KW_CONSTRAINTS
            | SyntaxKind::KW_CREATE
            | SyntaxKind::KW_DATABASE
            | SyntaxKind::KW_DATABASES
            | SyntaxKind::KW_DELETE
            | SyntaxKind::KW_DESC
            | SyntaxKind::KW_DESCENDING
            | SyntaxKind::KW_DETACH
            | SyntaxKind::KW_DISTINCT
            | SyntaxKind::KW_DO
            | SyntaxKind::KW_DROP
            | SyntaxKind::KW_ELSE
            | SyntaxKind::KW_END
            | SyntaxKind::KW_ENDS
            | SyntaxKind::KW_ERROR
            | SyntaxKind::KW_EXISTS
            | SyntaxKind::KW_EXTRACT
            | SyntaxKind::KW_FAIL
            | SyntaxKind::KW_FILTER
            | SyntaxKind::KW_FOR
            | SyntaxKind::KW_FOREACH
            | SyntaxKind::KW_EACH
            | SyntaxKind::KW_FUNCTIONS
            | SyntaxKind::KW_FULLTEXT
            | SyntaxKind::KW_IF
            | SyntaxKind::KW_IN
            | SyntaxKind::KW_INDEX
            | SyntaxKind::KW_INDEXES
            | SyntaxKind::KW_IS
            | SyntaxKind::KW_KEY
            | SyntaxKind::KW_LIMIT
            | SyntaxKind::KW_LOOKUP
            | SyntaxKind::KW_MANDATORY
            | SyntaxKind::KW_MATCH
            | SyntaxKind::KW_MERGE
            | SyntaxKind::KW_NODE
            | SyntaxKind::KW_NONE
            | SyntaxKind::KW_NOT
            | SyntaxKind::KW_OF
            | SyntaxKind::KW_ON
            | SyntaxKind::KW_OPTIONAL
            | SyntaxKind::KW_OPTIONS
            | SyntaxKind::KW_OR
            | SyntaxKind::KW_ORDER
            | SyntaxKind::KW_POINT
            | SyntaxKind::KW_PROCEDURES
            | SyntaxKind::KW_PROPERTY
            | SyntaxKind::KW_RANGE
            | SyntaxKind::KW_REMOVE
            | SyntaxKind::KW_REQUIRE
            | SyntaxKind::KW_RETURN
            | SyntaxKind::KW_ROWS
            | SyntaxKind::KW_SCALAR
            | SyntaxKind::KW_SET
            | SyntaxKind::KW_SHOW
            | SyntaxKind::KW_SINGLE
            | SyntaxKind::KW_SKIP
            | SyntaxKind::KW_STARTS
            | SyntaxKind::KW_TEXT
            | SyntaxKind::KW_THEN
            | SyntaxKind::KW_TRANSACTIONS
            | SyntaxKind::KW_TYPE
            | SyntaxKind::KW_TYPES
            | SyntaxKind::KW_UNION
            | SyntaxKind::KW_UNIQUE
            | SyntaxKind::KW_UNWIND
            | SyntaxKind::KW_USE
            | SyntaxKind::KW_WHEN
            | SyntaxKind::KW_WHERE
            | SyntaxKind::KW_WITH
            | SyntaxKind::KW_XOR
            | SyntaxKind::KW_YIELD
            | SyntaxKind::KW_COUNT
            | SyntaxKind::KW_CALL_SUBQUERY
            | SyntaxKind::KW_IN_TRANSACTIONS
            | SyntaxKind::KW_CONCURRENTLY
    )
}
