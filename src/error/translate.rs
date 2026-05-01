//! Pest error translation and common-mistake detection.

use crate::error::*;
use crate::parser::Rule;
use pest::error::{ErrorVariant, InputLocation};
use std::collections::BTreeSet;
use std::sync::Arc;

/// Translate a raw pest error into a structured `CypherError`.
pub fn translate_pest_error(pest_err: pest::error::Error<Rule>, source: Arc<str>) -> CypherError {
    let span = match pest_err.location {
        InputLocation::Pos(pos) => Span::new(pos, pos.saturating_add(1).min(source.len())),
        InputLocation::Span((start, end)) => Span::new(start, end.max(start + 1)),
    };

    let kind = match &pest_err.variant {
        ErrorVariant::ParsingError { positives, .. } => {
            let expected = build_expected(positives);
            if expected.is_empty() || span.start >= source.len() {
                ErrorKind::UnexpectedEof { expected }
            } else {
                let found_char = source[span.start..span.end.min(source.len())].to_string();
                ErrorKind::UnexpectedToken {
                    expected,
                    found: found_char,
                }
            }
        }
        ErrorVariant::CustomError { message } => ErrorKind::Internal {
            message: message.clone(),
        },
    };

    let source_len = source.len();
    let mut err = CypherError {
        kind,
        span,
        source_label: None,
        notes: Vec::new(),
        source: Some(source.clone()),
    };

    detect_common_mistakes(&mut err, &source, source_len);
    err
}

/// Build a deduplicated, user-friendly list of expected tokens from pest positives.
fn build_expected(positives: &[Rule]) -> Vec<Expected> {
    let mut seen = BTreeSet::new();
    let mut result = Vec::new();

    for rule in positives {
        let entry = rule_to_expected(rule);
        let key = match &entry {
            Expected::Keyword(k) => ("K", *k),
            Expected::Symbol(s) => ("S", *s),
            Expected::Category(c) => ("C", *c),
        };
        if seen.insert(key) {
            result.push(entry);
        }
    }

    result
}

/// Map a pest `Rule` to a user-facing `Expected` description.
fn rule_to_expected(rule: &Rule) -> Expected {
    match rule {
        Rule::MATCH => Expected::Keyword("MATCH"),
        Rule::OPTIONAL => Expected::Keyword("OPTIONAL"),
        Rule::CREATE => Expected::Keyword("CREATE"),
        Rule::MERGE => Expected::Keyword("MERGE"),
        Rule::DELETE => Expected::Keyword("DELETE"),
        Rule::DETACH => Expected::Keyword("DETACH"),
        Rule::REMOVE => Expected::Keyword("REMOVE"),
        Rule::SET => Expected::Keyword("SET"),
        Rule::RETURN => Expected::Keyword("RETURN"),
        Rule::WITH => Expected::Keyword("WITH"),
        Rule::WHERE => Expected::Keyword("WHERE"),
        Rule::ORDER => Expected::Keyword("ORDER"),
        Rule::BY => Expected::Keyword("BY"),
        Rule::SKIP => Expected::Keyword("SKIP"),
        Rule::LIMIT => Expected::Keyword("LIMIT"),
        Rule::UNWIND => Expected::Keyword("UNWIND"),
        Rule::AS => Expected::Keyword("AS"),
        Rule::UNION => Expected::Keyword("UNION"),
        Rule::ALL => Expected::Keyword("ALL"),
        Rule::AND => Expected::Keyword("AND"),
        Rule::OR => Expected::Keyword("OR"),
        Rule::XOR => Expected::Keyword("XOR"),
        Rule::NOT => Expected::Keyword("NOT"),
        Rule::IS => Expected::Keyword("IS"),
        Rule::NULL => Expected::Keyword("NULL"),
        Rule::TRUE => Expected::Keyword("TRUE"),
        Rule::FALSE => Expected::Keyword("FALSE"),
        Rule::CASE => Expected::Keyword("CASE"),
        Rule::WHEN => Expected::Keyword("WHEN"),
        Rule::THEN => Expected::Keyword("THEN"),
        Rule::ELSE => Expected::Keyword("ELSE"),
        Rule::END => Expected::Keyword("END"),
        Rule::DISTINCT => Expected::Keyword("DISTINCT"),
        Rule::EXISTS => Expected::Keyword("EXISTS"),
        Rule::IN => Expected::Keyword("IN"),
        Rule::CONTAINS => Expected::Keyword("CONTAINS"),
        Rule::STARTS => Expected::Keyword("STARTS"),
        Rule::ENDS => Expected::Keyword("ENDS"),
        Rule::CALL => Expected::Keyword("CALL"),
        Rule::YIELD => Expected::Keyword("YIELD"),
        Rule::ON => Expected::Keyword("ON"),
        Rule::ADD => Expected::Keyword("ADD"),
        Rule::FILTER => Expected::Keyword("FILTER"),
        Rule::EXTRACT => Expected::Keyword("EXTRACT"),
        Rule::ASC => Expected::Keyword("ASC"),
        Rule::ASCENDING => Expected::Keyword("ASCENDING"),
        Rule::DESC => Expected::Keyword("DESC"),
        Rule::DESCENDING => Expected::Keyword("DESCENDING"),
        Rule::COUNT => Expected::Keyword("COUNT"),
        Rule::ANY_ => Expected::Keyword("ANY"),
        Rule::NONE => Expected::Keyword("NONE"),
        Rule::SINGLE => Expected::Keyword("SINGLE"),
        Rule::DROP_ => Expected::Keyword("DROP"),

        Rule::STAR | Rule::MULTIPLY => Expected::Symbol("*"),
        Rule::PLUS => Expected::Symbol("+"),
        Rule::SUBTRACT => Expected::Symbol("-"),
        Rule::EQ => Expected::Symbol("="),
        Rule::NE => Expected::Symbol("<>"),
        Rule::LT => Expected::Symbol("<"),
        Rule::GT => Expected::Symbol(">"),
        Rule::LE => Expected::Symbol("<="),
        Rule::GE => Expected::Symbol(">="),
        Rule::POW => Expected::Symbol("^"),
        Rule::DIVIDE => Expected::Symbol("/"),
        Rule::MODULO => Expected::Symbol("%"),
        Rule::DOT_DOT => Expected::Symbol(".."),
        Rule::LeftArrowHead => Expected::Symbol("<"),
        Rule::RightArrowHead => Expected::Symbol(">"),
        Rule::Dash => Expected::Symbol("-"),

        Rule::Cypher => Expected::Category("query"),
        Rule::Statement => Expected::Category("statement"),
        Rule::Query => Expected::Category("query"),
        Rule::RegularQuery => Expected::Category("query"),
        Rule::SingleQuery => Expected::Category("query"),
        Rule::SinglePartQuery => Expected::Category("clause"),
        Rule::MultiPartQuery => Expected::Category("query"),
        Rule::ReadingClause => Expected::Category("clause"),
        Rule::UpdatingClause => Expected::Category("clause"),
        Rule::Pattern => Expected::Category("pattern"),
        Rule::PatternPart => Expected::Category("pattern"),
        Rule::AnonymousPatternPart => Expected::Category("pattern"),
        Rule::PatternElement => Expected::Category("pattern"),
        Rule::NodePattern => Expected::Category("node pattern"),
        Rule::RelationshipPattern => Expected::Category("relationship pattern"),
        Rule::RelationshipDetail => Expected::Category("relationship detail"),
        Rule::RelationshipTypes => Expected::Category("relationship type"),
        Rule::NodeLabels => Expected::Category("node labels"),
        Rule::NodeLabel => Expected::Category("node label"),
        Rule::LabelName => Expected::Category("label name"),
        Rule::Expression => Expected::Category("expression"),
        Rule::OrExpression => Expected::Category("expression"),
        Rule::XorExpression => Expected::Category("expression"),
        Rule::AndExpression => Expected::Category("expression"),
        Rule::NotExpression => Expected::Category("expression"),
        Rule::ComparisonExpression => Expected::Category("expression"),
        Rule::AddOrSubtractExpression => Expected::Category("expression"),
        Rule::MultiplyDivideModuloExpression => Expected::Category("expression"),
        Rule::PowerOfExpression => Expected::Category("expression"),
        Rule::UnaryAddOrSubtractExpression => Expected::Category("expression"),
        Rule::StringListNullOperatorExpression => Expected::Category("expression"),
        Rule::PropertyOrLabelsExpression => Expected::Category("expression"),
        Rule::Atom => Expected::Category("expression"),
        Rule::Literal => Expected::Category("literal"),
        Rule::NumberLiteral => Expected::Category("number"),
        Rule::IntegerLiteral => Expected::Category("integer"),
        Rule::DoubleLiteral => Expected::Category("float"),
        Rule::StringLiteral => Expected::Category("string"),
        Rule::BooleanLiteral => Expected::Category("boolean"),
        Rule::ListLiteral => Expected::Category("list"),
        Rule::MapLiteral => Expected::Category("map"),
        Rule::Parameter => Expected::Category("parameter"),
        Rule::Variable => Expected::Category("variable"),
        Rule::SymbolicName => Expected::Category("identifier"),
        Rule::UnescapedSymbolicName => Expected::Category("identifier"),
        Rule::EscapedSymbolicName => Expected::Category("identifier"),
        Rule::FunctionInvocation => Expected::Category("function call"),
        Rule::FunctionName => Expected::Category("function name"),
        Rule::ProcedureName => Expected::Category("procedure name"),
        Rule::Namespace => Expected::Category("namespace"),
        Rule::CaseExpression => Expected::Category("CASE expression"),
        Rule::ListComprehension => Expected::Category("list comprehension"),
        Rule::PatternComprehension => Expected::Category("pattern comprehension"),
        Rule::FilterExpression => Expected::Category("filter expression"),
        Rule::RelationshipsPattern => Expected::Category("pattern"),
        Rule::ParenthesizedExpression => Expected::Category("expression"),
        Rule::ExistentialSubquery => Expected::Category("EXISTS subquery"),
        Rule::ProjectionItems => Expected::Category("projection"),
        Rule::ProjectionItem => Expected::Category("projection"),
        Rule::ProjectionBody => Expected::Category("projection"),
        Rule::SortItem => Expected::Category("sort item"),
        Rule::Order => Expected::Category("ORDER BY clause"),
        Rule::Skip => Expected::Category("SKIP clause"),
        Rule::Limit => Expected::Category("LIMIT clause"),
        Rule::Where => Expected::Category("WHERE clause"),
        Rule::With => Expected::Keyword("WITH"),
        Rule::Return => Expected::Keyword("RETURN"),
        Rule::Match => Expected::Keyword("MATCH"),
        Rule::Unwind => Expected::Keyword("UNWIND"),
        Rule::Merge => Expected::Keyword("MERGE"),
        Rule::Create => Expected::Keyword("CREATE"),
        Rule::Delete => Expected::Keyword("DELETE"),
        Rule::Remove => Expected::Keyword("REMOVE"),
        Rule::Set => Expected::Keyword("SET"),
        Rule::StandaloneCall => Expected::Keyword("CALL"),
        Rule::InQueryCall => Expected::Keyword("CALL"),
        Rule::YieldItems => Expected::Category("YIELD clause"),
        Rule::YieldItem => Expected::Category("YIELD item"),
        Rule::ProcedureResultField => Expected::Category("procedure field"),
        Rule::ExplicitProcedureInvocation => Expected::Category("procedure call"),
        Rule::ImplicitProcedureInvocation => Expected::Category("procedure call"),
        Rule::MergeAction => Expected::Category("MERGE action"),
        Rule::SetItem => Expected::Category("SET item"),
        Rule::RemoveItem => Expected::Category("REMOVE item"),
        Rule::PropertyExpression => Expected::Category("property expression"),
        Rule::PropertyLookup => Expected::Category("property lookup"),
        Rule::PropertyKeyName => Expected::Category("property name"),
        Rule::SchemaName => Expected::Category("name"),
        Rule::ReservedWord => Expected::Category("keyword"),
        Rule::RangeLiteral => Expected::Category("range"),
        Rule::RelTypeName => Expected::Category("relationship type name"),
        Rule::Properties => Expected::Category("properties"),
        Rule::ListOperatorExpression => Expected::Category("list operator"),
        Rule::StringOperatorExpression => Expected::Category("string operator"),
        Rule::NullOperatorExpression => Expected::Category("null operator"),
        Rule::PartialComparisonExpression => Expected::Category("comparison"),
        Rule::CaseAlternative => Expected::Category("CASE alternative"),
        Rule::IdInColl => Expected::Category("variable IN collection"),
        Rule::HexInteger => Expected::Category("hex integer"),
        Rule::DecimalInteger => Expected::Category("integer"),
        Rule::OctalInteger => Expected::Category("octal integer"),
        Rule::ExponentDecimalReal => Expected::Category("float"),
        Rule::RegularDecimalReal => Expected::Category("float"),
        Rule::StringDoubleText => Expected::Category("string content"),
        Rule::StringSingleText => Expected::Category("string content"),
        Rule::StringDoubleTextChar => Expected::Category("string character"),
        Rule::StringSingleTextChar => Expected::Category("string character"),
        Rule::EscapedChar => Expected::Category("escape sequence"),
        Rule::IdentifierStart => Expected::Category("identifier start"),
        Rule::IdentifierPart => Expected::Category("identifier character"),
        Rule::HexDigit => Expected::Category("hex digit"),
        Rule::Digit => Expected::Category("digit"),
        Rule::ZeroDigit => Expected::Category("digit"),
        Rule::NonZeroDigit => Expected::Category("non-zero digit"),
        Rule::HexLetter => Expected::Category("hex letter"),

        Rule::SP | Rule::whitespace | Rule::EOI | Rule::Comment => Expected::Category("whitespace"),

        Rule::Foreach => Expected::Keyword("FOREACH"),
        Rule::CallSubquery => Expected::Keyword("CALL"),
        Rule::InTransactions => Expected::Keyword("TRANSACTIONS"),
        Rule::MapProjection => Expected::Category("map projection"),
        Rule::MapProjectionItem => Expected::Category("map projection item"),
        Rule::CreateIndex => Expected::Keyword("CREATE INDEX"),
        Rule::DropIndex => Expected::Keyword("DROP INDEX"),
        Rule::CreateConstraint => Expected::Keyword("CREATE CONSTRAINT"),
        Rule::DropConstraint => Expected::Keyword("DROP CONSTRAINT"),
        Rule::SchemaCommand => Expected::Category("schema command"),
        Rule::IndexKind => Expected::Category("index kind"),
        Rule::ConstraintKind => Expected::Category("constraint kind"),
        Rule::Options => Expected::Keyword("OPTIONS"),
        Rule::Show => Expected::Keyword("SHOW"),
        Rule::ShowKind => Expected::Category("show kind"),
        Rule::Use => Expected::Keyword("USE"),
        Rule::FOR => Expected::Keyword("FOR"),
        Rule::INDEX => Expected::Keyword("INDEX"),
        Rule::KEY => Expected::Keyword("KEY"),
        Rule::RANGE => Expected::Keyword("RANGE"),
        Rule::TEXT => Expected::Keyword("TEXT"),
        Rule::POINT => Expected::Keyword("POINT"),
        Rule::LOOKUP => Expected::Keyword("LOOKUP"),
        Rule::FULLTEXT => Expected::Keyword("FULLTEXT"),
        Rule::PROPERTY => Expected::Keyword("PROPERTY"),
        Rule::TYPE => Expected::Keyword("TYPE"),
        Rule::IF => Expected::Keyword("IF"),
        Rule::SHOW => Expected::Keyword("SHOW"),
        Rule::USE => Expected::Keyword("USE"),
        Rule::INDEXES => Expected::Keyword("INDEXES"),
        Rule::CONSTRAINTS => Expected::Keyword("CONSTRAINTS"),
        Rule::FUNCTIONS => Expected::Keyword("FUNCTIONS"),
        Rule::PROCEDURES => Expected::Keyword("PROCEDURES"),
        Rule::DATABASES => Expected::Keyword("DATABASES"),
        Rule::DATABASE => Expected::Keyword("DATABASE"),
        Rule::NODE => Expected::Keyword("NODE"),
        Rule::FOREACH => Expected::Keyword("FOREACH"),
        Rule::TRANSACTIONS => Expected::Keyword("TRANSACTIONS"),
        Rule::ROWS => Expected::Keyword("ROWS"),
        Rule::ERROR => Expected::Keyword("ERROR"),
        Rule::CONTINUE => Expected::Keyword("CONTINUE"),
        Rule::BREAK => Expected::Keyword("BREAK"),
        Rule::FAIL => Expected::Keyword("FAIL"),

        _ => Expected::Category("syntax"),
    }
}

/// Detect common mistakes by scanning the source text around the failure point.
pub fn detect_common_mistakes(err: &mut CypherError, source: &str, source_len: usize) {
    let pos = err.span.start.min(source_len);
    let after = &source[pos..];

    detect_unterminated_string(err, source, pos);
    if !matches!(err.kind, ErrorKind::UnterminatedString) {
        detect_null_comparison(err, after, pos, source_len);
        detect_empty_return(err, after, pos, source_len);
        detect_match_without_paren(err, after);
        detect_stray_semicolon(err, after, pos, source_len);
    }
    detect_leading_semicolon(err, source);
    if !matches!(err.kind, ErrorKind::UnterminatedString) {
        detect_unterminated_comment(err, source);
    }
}

fn detect_unterminated_string(err: &mut CypherError, source: &str, fail_pos: usize) {
    let end = source.len();
    let scan_end = (fail_pos + 4).min(end);
    let before = &source[..scan_end];
    let mut last_open: Option<(usize, char)> = None;
    let mut i = 0;
    let chars: Vec<(usize, char)> = before.char_indices().collect();
    while i < chars.len() {
        let (offset, ch) = chars[i];
        if ch == '"' || ch == '\'' {
            if let Some((open_pos, open_ch)) = last_open {
                if open_ch == ch {
                    if is_escaped(&chars, i) {
                        last_open = Some((open_pos, open_ch));
                    } else {
                        last_open = None;
                    }
                }
            } else {
                last_open = Some((offset, ch));
            }
        }
        i += 1;
    }
    if let Some((quote_pos, open_ch)) = last_open {
        let end = source[quote_pos..]
            .find(open_ch)
            .map(|i| quote_pos + i + 1)
            .unwrap_or(source.len());
        err.kind = ErrorKind::UnterminatedString;
        err.span = Span::new(quote_pos, end.min(source.len()));
        err.notes.push(Note {
            span: Span::new(quote_pos, (quote_pos + 1).min(source.len())),
            message: "string literal starts here".into(),
            level: NoteLevel::Info,
        });
    }
}

fn is_escaped(chars: &[(usize, char)], idx: usize) -> bool {
    if idx == 0 {
        return false;
    }
    let mut count = 0;
    let mut i = idx - 1;
    loop {
        if chars[i].1 == '\\' {
            count += 1;
        } else {
            break;
        }
        if i == 0 {
            break;
        }
        i -= 1;
    }
    count % 2 != 0
}

fn detect_null_comparison(err: &mut CypherError, after: &str, pos: usize, source_len: usize) {
    let trimmed = after.trim_start();
    let lower = trimmed.to_lowercase();
    let offset = after.len() - trimmed.len();
    if lower.starts_with("= null")
        || lower.starts_with("<> null")
        || lower.starts_with("!= null")
        || lower.starts_with("=null")
        || lower.starts_with("<>null")
        || lower.starts_with("!=null")
    {
        let null_pos = pos + offset + lower.find("null").unwrap_or(0);
        let null_end = (null_pos + 4).min(source_len);
        err.notes.push(Note {
            span: Span::new(null_pos.min(source_len), null_end),
            message: "Cypher uses 'IS NULL' / 'IS NOT NULL' for null comparison".into(),
            level: NoteLevel::Help,
        });
    }
}

fn detect_empty_return(err: &mut CypherError, after: &str, pos: usize, source_len: usize) {
    let trimmed = after.trim_start();
    if (trimmed.starts_with(';') || trimmed.is_empty())
        && matches!(
            &err.kind,
            ErrorKind::UnexpectedToken { .. } | ErrorKind::UnexpectedEof { .. }
        )
    {
        err.kind = ErrorKind::MissingClause {
            clause: "projection",
            after: "RETURN",
        };
        let semi_pos = pos + (after.len() - after.trim_start().len());
        err.span = Span::new(semi_pos, (semi_pos + 1).min(source_len));
    }
}

fn detect_match_without_paren(err: &mut CypherError, after: &str) {
    let trimmed = after.trim_start();
    if !trimmed.starts_with('(') && !trimmed.is_empty() && !trimmed.starts_with(';') {
        if let ErrorKind::UnexpectedToken { expected, .. } = &mut err.kind {
            if expected.iter().any(|e| {
                matches!(
                    e,
                    Expected::Category("pattern") | Expected::Keyword("MATCH")
                )
            }) {
                err.notes.push(Note {
                    span: err.span,
                    message: "patterns must start with '('".into(),
                    level: NoteLevel::Help,
                });
            }
        }
    }
}

fn detect_stray_semicolon(err: &mut CypherError, after: &str, pos: usize, source_len: usize) {
    let trimmed = after.trim_start();
    if trimmed.starts_with(";;") {
        let semi_pos = pos + (after.len() - after.trim_start().len());
        err.notes.push(Note {
            span: Span::new(semi_pos, (semi_pos + 2).min(source_len)),
            message: "stray extra semicolon".into(),
            level: NoteLevel::Warning,
        });
    }
}

fn detect_leading_semicolon(err: &mut CypherError, source: &str) {
    let trimmed = source.trim_start();
    if trimmed.starts_with(';') {
        let semi_pos = source.len() - trimmed.len();
        err.notes.push(Note {
            span: Span::new(semi_pos, semi_pos + 1),
            message: "leading semicolon is not expected".into(),
            level: NoteLevel::Warning,
        });
    }
}

fn detect_unterminated_comment(err: &mut CypherError, source: &str) {
    let mut in_block = false;
    let mut last_star = false;
    let mut block_start = 0;
    for (i, ch) in source.char_indices() {
        if !in_block && ch == '/' {
            let next = source[i + 1..].chars().next();
            if next == Some('*') {
                in_block = true;
                block_start = i;
                continue;
            } else if next == Some('/') {
                continue;
            }
        }
        if in_block {
            if ch == '*' {
                last_star = true;
            } else if ch == '/' && last_star {
                in_block = false;
                last_star = false;
            } else {
                last_star = false;
            }
        }
    }
    if in_block {
        err.notes.push(Note {
            span: Span::new(block_start, source.len()),
            message: "unterminated comment".into(),
            level: NoteLevel::Warning,
        });
    }
}
