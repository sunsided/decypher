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
        SyntaxKind::EQ | SyntaxKind::NE | SyntaxKind::LT
        | SyntaxKind::GT | SyntaxKind::LE | SyntaxKind::GE => Some((Prec::COMPARISON, ())),
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
        SyntaxKind::EQ | SyntaxKind::NE | SyntaxKind::LT
        | SyntaxKind::GT | SyntaxKind::LE | SyntaxKind::GE => SyntaxKind::COMPARISON_EXPR,
        SyntaxKind::PLUS | SyntaxKind::MINUS => SyntaxKind::ADD_SUB_EXPR,
        SyntaxKind::STAR | SyntaxKind::SLASH | SyntaxKind::PERCENT => SyntaxKind::MUL_DIV_MOD_EXPR,
        SyntaxKind::POW => SyntaxKind::POWER_EXPR,
        _ => SyntaxKind::EXPRESSION,
    };

    p.start_node(node_kind);
    p.bump();
    p.skip_trivia();

    let rhs_bp = match op {
        SyntaxKind::POW => bp,       // right-associative
        _ => Prec(bp.0 + 1),          // left-associative
    };
    expr_bp(p, rhs_bp);
    p.builder.finish_node();
}

fn parse_property_lookup(p: &mut Parser) {
    p.start_node(SyntaxKind::PROPERTY_LOOKUP);
    p.bump(); // DOT
    p.skip_trivia();
    if p.at(SyntaxKind::IDENT) || p.at(SyntaxKind::ESCAPED_IDENT) {
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
            p.bump();
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
            let is_func = p.peek_next_non_trivia() == Some(SyntaxKind::L_PAREN);
            if is_func {
                p.start_node(SyntaxKind::FUNCTION_INVOCATION);
                p.start_node(SyntaxKind::FUNCTION_NAME);
                parse_namespace_and_name(p);
                p.builder.finish_node();
                p.skip_trivia();
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
            } else {
                p.start_node(SyntaxKind::VARIABLE);
                p.start_node(SyntaxKind::SYMBOLIC_NAME);
                p.bump();
                p.builder.finish_node();
                p.builder.finish_node();
            }
        }
        SyntaxKind::L_PAREN => {
            p.start_node(SyntaxKind::PARENTHESIZED_EXPR);
            p.bump();
            p.skip_trivia();
            expr_bp(p, Prec::MIN);
            p.skip_trivia();
            p.expect(SyntaxKind::R_PAREN);
            p.builder.finish_node();
        }
        SyntaxKind::L_BRACKET => {
            p.start_node(SyntaxKind::LIST_LITERAL);
            p.bump();
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
            if p.at(SyntaxKind::IDENT) || p.at(SyntaxKind::ESCAPED_IDENT) || p.at(SyntaxKind::INTEGER) {
                p.bump();
            }
            p.builder.finish_node();
        }
        SyntaxKind::KW_CASE => {
            parse_case_expr(p);
        }
        SyntaxKind::KW_ALL | SyntaxKind::KW_ANY | SyntaxKind::KW_NONE | SyntaxKind::KW_SINGLE
        | SyntaxKind::KW_FILTER | SyntaxKind::KW_EXTRACT => {
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
                expr_bp(p, Prec::MIN);
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
                p.bump();
                p.skip_trivia();
                p.bump();
                p.skip_trivia();
                while !p.at(SyntaxKind::R_BRACE) && p.current_len() > 0 {
                    p.bump();
                    p.skip_trivia();
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
            p.start_node(SyntaxKind::ERROR);
            if p.current_len() > 0 {
                p.bump();
            }
            p.builder.finish_node();
        }
    }
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
            if p.at(SyntaxKind::IDENT) {
                p.bump();
            }
            p.builder.finish_node();
            p.builder.finish_node();
        }
    } else {
        p.start_node(SyntaxKind::PROPERTY_KEY_NAME);
        p.start_node(SyntaxKind::SYMBOLIC_NAME);
        if p.at(SyntaxKind::IDENT) {
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
        SyntaxKind::KW_DELETE => parse_delete_clause(p),
        SyntaxKind::KW_SET => parse_set_clause(p),
        SyntaxKind::KW_REMOVE => parse_remove_clause(p),
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
        parse_node_pattern(p);
        p.skip_trivia();
        while is_relationship_chain_start(p) {
            parse_pattern_element_chain(p);
            p.skip_trivia();
        }
    } else {
        p.bump();
        parse_pattern_element(p);
        p.expect(SyntaxKind::R_PAREN);
    }
    p.builder.finish_node();
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
    if p.at(SyntaxKind::IDENT) || p.at(SyntaxKind::ESCAPED_IDENT) {
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
            if p.at(SyntaxKind::IDENT) || p.at(SyntaxKind::ESCAPED_IDENT) {
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
    if p.at(SyntaxKind::IDENT) || p.at(SyntaxKind::ESCAPED_IDENT) {
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
        if p.at(SyntaxKind::IDENT) || p.at(SyntaxKind::ESCAPED_IDENT) {
            p.start_node(SyntaxKind::NODE_LABEL);
            p.start_node(SyntaxKind::LABEL_NAME);
            p.start_node(SyntaxKind::SYMBOLIC_NAME);
            p.bump();
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
    if p.at(SyntaxKind::KW_ASCENDING) || p.at(SyntaxKind::KW_ASC)
        || p.at(SyntaxKind::KW_DESCENDING) || p.at(SyntaxKind::KW_DESC)
    {
        p.bump();
    }
    p.builder.finish_node();
}

fn is_relationship_chain_start(p: &Parser) -> bool {
    matches!(
        p.current_kind(),
        SyntaxKind::LT | SyntaxKind::GT | SyntaxKind::ARROW_LEFT
            | SyntaxKind::ARROW_RIGHT | SyntaxKind::MINUS | SyntaxKind::DASH
    )
}
