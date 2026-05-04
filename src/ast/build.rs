use pest::iterators::Pair;

use crate::ast::clause::*;
use crate::ast::expr::*;
use crate::ast::names::*;
use crate::ast::pattern::*;
use crate::ast::procedure::*;
use crate::ast::query::*;
use crate::ast::schema::*;
use crate::error::{CypherError, ErrorKind, Result, Span};
use crate::pest_parser::Rule;

fn span(pair: &Pair<'_, Rule>) -> Span {
    let s = pair.as_span();
    Span::new(s.start(), s.end())
}

fn unsupported(rule: Rule, sp: Span) -> CypherError {
    CypherError {
        kind: ErrorKind::Unsupported {
            production: match rule {
                Rule::Cypher => "Cypher",
                Rule::Statement => "Statement",
                Rule::Query => "Query",
                Rule::RegularQuery => "RegularQuery",
                Rule::SingleQuery => "SingleQuery",
                Rule::SinglePartQuery => "SinglePartQuery",
                Rule::MultiPartQuery => "MultiPartQuery",
                Rule::Union => "Union",
                Rule::ReadingClause => "ReadingClause",
                Rule::UpdatingClause => "UpdatingClause",
                Rule::Match => "Match",
                Rule::Unwind => "Unwind",
                Rule::Merge => "Merge",
                Rule::MergeAction => "MergeAction",
                Rule::Create => "Create",
                Rule::Delete => "Delete",
                Rule::Set => "Set",
                Rule::SetItem => "SetItem",
                Rule::Remove => "Remove",
                Rule::RemoveItem => "RemoveItem",
                Rule::InQueryCall => "InQueryCall",
                Rule::StandaloneCall => "StandaloneCall",
                Rule::YieldItems => "YieldItems",
                Rule::YieldItem => "YieldItem",
                Rule::With => "With",
                Rule::Return => "Return",
                Rule::ProjectionBody => "ProjectionBody",
                Rule::ProjectionItems => "ProjectionItems",
                Rule::ProjectionItem => "ProjectionItem",
                Rule::Order => "Order",
                Rule::SortItem => "SortItem",
                Rule::Where => "Where",
                Rule::Pattern => "Pattern",
                Rule::PatternPart => "PatternPart",
                Rule::AnonymousPatternPart => "AnonymousPatternPart",
                Rule::PatternElement => "PatternElement",
                Rule::NodePattern => "NodePattern",
                Rule::PatternElementChain => "PatternElementChain",
                Rule::RelationshipPattern => "RelationshipPattern",
                Rule::RelationshipDetail => "RelationshipDetail",
                Rule::RelationshipTypes => "RelationshipTypes",
                Rule::NodeLabels => "NodeLabels",
                Rule::NodeLabel => "NodeLabel",
                Rule::RangeLiteral => "RangeLiteral",
                Rule::LabelName => "LabelName",
                Rule::RelTypeName => "RelTypeName",
                Rule::Properties => "Properties",
                Rule::RelationshipsPattern => "RelationshipsPattern",
                Rule::Expression => "Expression",
                Rule::OrExpression => "OrExpression",
                Rule::XorExpression => "XorExpression",
                Rule::AndExpression => "AndExpression",
                Rule::NotExpression => "NotExpression",
                Rule::ComparisonExpression => "ComparisonExpression",
                Rule::AddOrSubtractExpression => "AddOrSubtractExpression",
                Rule::MultiplyDivideModuloExpression => "MultiplyDivideModuloExpression",
                Rule::PowerOfExpression => "PowerOfExpression",
                Rule::UnaryAddOrSubtractExpression => "UnaryAddOrSubtractExpression",
                Rule::StringListNullOperatorExpression => "StringListNullOperatorExpression",
                Rule::PropertyOrLabelsExpression => "PropertyOrLabelsExpression",
                Rule::Atom => "Atom",
                Rule::Literal => "Literal",
                Rule::NumberLiteral => "NumberLiteral",
                Rule::IntegerLiteral => "IntegerLiteral",
                Rule::DoubleLiteral => "DoubleLiteral",
                Rule::StringLiteral => "StringLiteral",
                Rule::BooleanLiteral => "BooleanLiteral",
                Rule::ListLiteral => "ListLiteral",
                Rule::MapLiteral => "MapLiteral",
                Rule::Parameter => "Parameter",
                Rule::Variable => "Variable",
                Rule::SymbolicName => "SymbolicName",
                Rule::FunctionInvocation => "FunctionInvocation",
                Rule::FunctionName => "FunctionName",
                Rule::Namespace => "Namespace",
                Rule::ProcedureName => "ProcedureName",
                Rule::ExplicitProcedureInvocation => "ExplicitProcedureInvocation",
                Rule::ImplicitProcedureInvocation => "ImplicitProcedureInvocation",
                Rule::ProcedureResultField => "ProcedureResultField",
                Rule::CaseExpression => "CaseExpression",
                Rule::CaseAlternative => "CaseAlternative",
                Rule::ListComprehension => "ListComprehension",
                Rule::PatternComprehension => "PatternComprehension",
                Rule::FilterExpression => "FilterExpression",
                Rule::IdInColl => "IdInColl",
                Rule::ParenthesizedExpression => "ParenthesizedExpression",
                Rule::ExistentialSubquery => "ExistentialSubquery",
                Rule::PartialComparisonExpression => "PartialComparisonExpression",
                Rule::ListOperatorExpression => "ListOperatorExpression",
                Rule::StringOperatorExpression => "StringOperatorExpression",
                Rule::NullOperatorExpression => "NullOperatorExpression",
                Rule::PropertyLookup => "PropertyLookup",
                Rule::PropertyExpression => "PropertyExpression",
                Rule::PropertyKeyName => "PropertyKeyName",
                Rule::SchemaName => "SchemaName",
                Rule::ReservedWord => "ReservedWord",
                Rule::StringDoubleText => "StringDoubleText",
                Rule::StringSingleText => "StringSingleText",
                Rule::StringDoubleTextChar => "StringDoubleTextChar",
                Rule::StringSingleTextChar => "StringSingleTextChar",
                Rule::EscapedChar => "EscapedChar",
                Rule::ALL => "ALL",
                Rule::ANY_ => "ANY_",
                Rule::NONE => "NONE",
                Rule::SINGLE => "SINGLE",
                Rule::COUNT => "COUNT",
                Rule::FILTER => "FILTER",
                Rule::EXTRACT => "EXTRACT",
                Rule::Foreach => "Foreach",
                Rule::CallSubquery => "CallSubquery",
                Rule::InTransactions => "InTransactions",
                Rule::MapProjection => "MapProjection",
                Rule::MapProjectionItem => "MapProjectionItem",
                Rule::CreateIndex => "CreateIndex",
                Rule::DropIndex => "DropIndex",
                Rule::CreateConstraint => "CreateConstraint",
                Rule::DropConstraint => "DropConstraint",
                Rule::SchemaCommand => "SchemaCommand",
                Rule::IndexKind => "IndexKind",
                Rule::ConstraintKind => "ConstraintKind",
                Rule::Options => "Options",
                Rule::Show => "Show",
                Rule::ShowKind => "ShowKind",
                Rule::Use => "Use",
                Rule::INDEX => "INDEX",
                Rule::KEY => "KEY",
                Rule::OPTIONS => "OPTIONS",
                Rule::RANGE => "RANGE",
                Rule::TEXT => "TEXT",
                Rule::POINT => "POINT",
                Rule::LOOKUP => "LOOKUP",
                Rule::FULLTEXT => "FULLTEXT",
                Rule::PROPERTY => "PROPERTY",
                Rule::TYPE => "TYPE",
                Rule::IF => "IF",
                Rule::SHOW => "SHOW",
                Rule::USE => "USE",
                Rule::INDEXES => "INDEXES",
                Rule::CONSTRAINTS => "CONSTRAINTS",
                Rule::FUNCTIONS => "FUNCTIONS",
                Rule::PROCEDURES => "PROCEDURES",
                Rule::DATABASES => "DATABASES",
                Rule::DATABASE => "DATABASE",
                Rule::NODE => "NODE",
                Rule::FOREACH => "FOREACH",
                Rule::TRANSACTIONS => "TRANSACTIONS",
                Rule::ROWS => "ROWS",
                Rule::ERROR => "ERROR",
                Rule::CONTINUE => "CONTINUE",
                Rule::BREAK => "BREAK",
                Rule::FAIL => "FAIL",
                Rule::FOR => "FOR",
                _ => "unknown",
            },
        },
        span: sp,
        source_label: None,
        notes: Vec::new(),
        source: None,
    }
}

pub fn build_query(pair: Pair<'_, Rule>) -> Result<Query> {
    assert_eq!(pair.as_rule(), Rule::Cypher);
    let sp = span(&pair);
    let mut inner = pair.into_inner();
    let statement = inner.next().ok_or_else(|| CypherError {
        kind: ErrorKind::Internal {
            message: "empty statement".into(),
        },
        span: sp,
        source_label: None,
        notes: Vec::new(),
        source: None,
    })?;
    build_statement(statement)
}

fn build_statement(pair: Pair<'_, Rule>) -> Result<Query> {
    assert_eq!(pair.as_rule(), Rule::Statement);
    let sp = span(&pair);
    let inner = pair.into_inner().next().unwrap();
    match inner.as_rule() {
        Rule::Query => build_query_variant(inner),
        Rule::SchemaCommand => {
            let cmd = build_schema_command(inner)?;
            Ok(Query {
                statements: vec![QueryBody::SchemaCommand(cmd)],
                span: sp,
            })
        }
        Rule::Show => {
            let show = build_show(inner)?;
            Ok(Query {
                statements: vec![QueryBody::Show(show)],
                span: sp,
            })
        }
        Rule::Use => {
            let use_stmt = build_use(inner)?;
            Ok(Query {
                statements: vec![QueryBody::Use(use_stmt)],
                span: sp,
            })
        }
        _ => Err(unsupported(inner.as_rule(), sp)),
    }
}

fn build_query_variant(pair: Pair<'_, Rule>) -> Result<Query> {
    assert_eq!(pair.as_rule(), Rule::Query);
    let sp = span(&pair);
    let inner = pair.into_inner().next().unwrap();
    match inner.as_rule() {
        Rule::RegularQuery => {
            let regular = build_regular_query(inner)?;
            if regular.unions.is_empty() {
                Ok(Query {
                    statements: vec![QueryBody::SingleQuery(regular.single_query.clone())],
                    span: sp,
                })
            } else {
                Ok(Query {
                    statements: vec![QueryBody::Regular(regular)],
                    span: sp,
                })
            }
        }
        Rule::StandaloneCall => {
            let standalone = build_standalone_call(inner)?;
            Ok(Query {
                statements: vec![QueryBody::Standalone(standalone)],
                span: sp,
            })
        }
        _ => Err(unsupported(inner.as_rule(), sp)),
    }
}

fn build_regular_query(pair: Pair<'_, Rule>) -> Result<RegularQuery> {
    assert_eq!(pair.as_rule(), Rule::RegularQuery);
    let mut inner = pair.into_inner();
    let single = inner.next().unwrap();
    let single_query = build_single_query(single)?;
    let unions = inner
        .filter(|p| p.as_rule() == Rule::Union)
        .map(|u| build_union(u))
        .collect::<Result<Vec<_>>>()?;
    Ok(RegularQuery {
        single_query,
        unions,
    })
}

fn build_union(pair: Pair<'_, Rule>) -> Result<Union> {
    assert_eq!(pair.as_rule(), Rule::Union);
    let sp = span(&pair);
    let mut all = false;
    let mut single_query_pair = None;
    for child in pair.into_inner() {
        match child.as_rule() {
            Rule::ALL => all = true,
            Rule::SingleQuery => single_query_pair = Some(child),
            Rule::UNION | Rule::SP => {}
            _ => return Err(unsupported(child.as_rule(), sp)),
        }
    }
    let single_query = build_single_query(single_query_pair.unwrap())?;
    Ok(Union {
        all,
        single_query,
        span: sp,
    })
}

fn build_single_query(pair: Pair<'_, Rule>) -> Result<SingleQuery> {
    assert_eq!(pair.as_rule(), Rule::SingleQuery);
    let sp = span(&pair);
    let inner = pair.into_inner().next().unwrap();
    let kind = match inner.as_rule() {
        Rule::SinglePartQuery => SingleQueryKind::SinglePart(build_single_part_query(inner)?),
        Rule::MultiPartQuery => SingleQueryKind::MultiPart(build_multi_part_query(inner)?),
        _ => return Err(unsupported(inner.as_rule(), sp)),
    };
    Ok(SingleQuery { kind })
}

fn build_single_part_query(pair: Pair<'_, Rule>) -> Result<SinglePartQuery> {
    assert_eq!(pair.as_rule(), Rule::SinglePartQuery);
    let sp = span(&pair);
    let mut reading_clauses = Vec::new();
    let mut updating_clauses = Vec::new();
    let mut return_clause = None;

    for child in pair.into_inner() {
        match child.as_rule() {
            Rule::ReadingClause => reading_clauses.push(build_reading_clause(child)?),
            Rule::UpdatingClause => updating_clauses.push(build_updating_clause(child)?),
            Rule::Return => return_clause = Some(build_return(child)?),
            Rule::SP => {}
            _ => return Err(unsupported(child.as_rule(), sp)),
        }
    }

    let body = if !updating_clauses.is_empty() {
        SinglePartBody::Updating {
            updating: updating_clauses,
            return_clause,
        }
    } else {
        SinglePartBody::Return(return_clause.unwrap())
    };

    Ok(SinglePartQuery {
        reading_clauses,
        body,
    })
}

fn build_multi_part_query(pair: Pair<'_, Rule>) -> Result<MultiPartQuery> {
    assert_eq!(pair.as_rule(), Rule::MultiPartQuery);
    let sp = span(&pair);
    let mut parts = Vec::new();
    let mut reading = Vec::new();
    let mut updating = Vec::new();
    let mut final_part = None;

    for child in pair.into_inner() {
        match child.as_rule() {
            Rule::ReadingClause => reading.push(build_reading_clause(child)?),
            Rule::UpdatingClause => updating.push(build_updating_clause(child)?),
            Rule::With => {
                let with_clause = build_with(child)?;
                parts.push(MultiPartQueryPart {
                    reading_clauses: std::mem::take(&mut reading),
                    updating_clauses: std::mem::take(&mut updating),
                    with: with_clause,
                });
            }
            Rule::SinglePartQuery => {
                final_part = Some(build_single_part_query(child)?);
            }
            Rule::SP => {}
            _ => return Err(unsupported(child.as_rule(), sp)),
        }
    }

    let final_part = final_part.ok_or_else(|| CypherError {
        kind: ErrorKind::Internal {
            message: "missing final single part query".into(),
        },
        span: sp,
        source_label: None,
        notes: Vec::new(),
        source: None,
    })?;

    Ok(MultiPartQuery { parts, final_part })
}

fn build_reading_clause(pair: Pair<'_, Rule>) -> Result<ReadingClause> {
    assert_eq!(pair.as_rule(), Rule::ReadingClause);
    let sp = span(&pair);
    let inner = pair.into_inner().next().unwrap();
    match inner.as_rule() {
        Rule::Match => Ok(ReadingClause::Match(build_match(inner)?)),
        Rule::Unwind => Ok(ReadingClause::Unwind(build_unwind(inner)?)),
        Rule::InQueryCall => Ok(ReadingClause::InQueryCall(build_in_query_call(inner)?)),
        Rule::CallSubquery => Ok(ReadingClause::CallSubquery(build_call_subquery(inner)?)),
        _ => Err(unsupported(inner.as_rule(), sp)),
    }
}

fn build_updating_clause(pair: Pair<'_, Rule>) -> Result<UpdatingClause> {
    assert_eq!(pair.as_rule(), Rule::UpdatingClause);
    let sp = span(&pair);
    let inner = pair.into_inner().next().unwrap();
    match inner.as_rule() {
        Rule::Create => Ok(UpdatingClause::Create(build_create(inner)?)),
        Rule::Merge => Ok(UpdatingClause::Merge(build_merge(inner)?)),
        Rule::Delete => Ok(UpdatingClause::Delete(build_delete(inner)?)),
        Rule::Set => Ok(UpdatingClause::Set(build_set(inner)?)),
        Rule::Remove => Ok(UpdatingClause::Remove(build_remove(inner)?)),
        Rule::Foreach => Ok(UpdatingClause::Foreach(build_foreach(inner)?)),
        _ => Err(unsupported(inner.as_rule(), sp)),
    }
}

fn build_match(pair: Pair<'_, Rule>) -> Result<Match> {
    assert_eq!(pair.as_rule(), Rule::Match);
    let sp = span(&pair);
    let mut optional = false;
    let mut pattern = None;
    let mut where_clause = None;

    for child in pair.into_inner() {
        match child.as_rule() {
            Rule::OPTIONAL => optional = true,
            Rule::MATCH | Rule::SP => {}
            Rule::Pattern => pattern = Some(build_pattern(child)?),
            Rule::Where => where_clause = Some(build_where(child)?),
            _ => return Err(unsupported(child.as_rule(), sp)),
        }
    }

    Ok(Match {
        optional,
        pattern: pattern.ok_or_else(|| CypherError {
            kind: ErrorKind::MissingClause {
                clause: "pattern",
                after: "MATCH",
            },
            span: sp,
            source_label: None,
            notes: Vec::new(),
            source: None,
        })?,
        where_clause,
        span: sp,
    })
}

fn build_unwind(pair: Pair<'_, Rule>) -> Result<Unwind> {
    assert_eq!(pair.as_rule(), Rule::Unwind);
    let sp = span(&pair);
    let children: Vec<_> = pair
        .into_inner()
        .filter(|p| {
            p.as_rule() != Rule::UNWIND && p.as_rule() != Rule::AS && p.as_rule() != Rule::SP
        })
        .collect();
    let expr = build_expression(children.first().unwrap().clone())?;
    let variable = build_variable(children.last().unwrap().clone())?;
    Ok(Unwind {
        expression: expr,
        variable,
        span: sp,
    })
}

fn build_merge(pair: Pair<'_, Rule>) -> Result<Merge> {
    assert_eq!(pair.as_rule(), Rule::Merge);
    let sp = span(&pair);
    let children: Vec<_> = pair.into_inner().collect();
    let mut actions = Vec::new();
    let mut pattern = None;

    for child in children {
        match child.as_rule() {
            Rule::MERGE | Rule::ON | Rule::SP => {}
            Rule::PatternPart => pattern = Some(build_pattern_part(child)?),
            Rule::MergeAction => actions.push(build_merge_action(child)?),
            _ => return Err(unsupported(child.as_rule(), sp)),
        }
    }

    Ok(Merge {
        pattern: pattern.ok_or_else(|| CypherError {
            kind: ErrorKind::MissingClause {
                clause: "pattern",
                after: "MERGE",
            },
            span: sp,
            source_label: None,
            notes: Vec::new(),
            source: None,
        })?,
        actions,
        span: sp,
    })
}

fn build_merge_action(pair: Pair<'_, Rule>) -> Result<MergeAction> {
    assert_eq!(pair.as_rule(), Rule::MergeAction);
    let sp = span(&pair);
    let mut on_match = false;
    let mut set_items = Vec::new();

    for child in pair.into_inner() {
        match child.as_rule() {
            Rule::ON | Rule::SP => {}
            Rule::MATCH => on_match = true,
            Rule::CREATE => {}
            Rule::Set => {
                for item in child.into_inner() {
                    if item.as_rule() == Rule::SetItem {
                        set_items.push(build_set_item(item)?);
                    }
                }
            }
            _ => return Err(unsupported(child.as_rule(), sp)),
        }
    }

    Ok(MergeAction {
        on_match,
        set_items,
        span: sp,
    })
}

fn build_create(pair: Pair<'_, Rule>) -> Result<Create> {
    assert_eq!(pair.as_rule(), Rule::Create);
    let sp = span(&pair);
    let pattern = pair
        .clone()
        .into_inner()
        .find(|p| p.as_rule() == Rule::Pattern)
        .map(build_pattern)
        .transpose()?
        .ok_or_else(|| CypherError {
            kind: ErrorKind::MissingClause {
                clause: "pattern",
                after: "CREATE",
            },
            span: sp,
            source_label: None,
            notes: Vec::new(),
            source: None,
        })?;
    Ok(Create { pattern, span: sp })
}

fn build_delete(pair: Pair<'_, Rule>) -> Result<Delete> {
    assert_eq!(pair.as_rule(), Rule::Delete);
    let sp = span(&pair);
    let mut detach = false;
    let mut targets = Vec::new();

    for child in pair.into_inner() {
        match child.as_rule() {
            Rule::DETACH => detach = true,
            Rule::DELETE | Rule::SP => {}
            Rule::Expression => targets.push(build_expression(child)?),
            _ => return Err(unsupported(child.as_rule(), sp)),
        }
    }

    Ok(Delete {
        detach,
        targets,
        span: sp,
    })
}

fn build_set(pair: Pair<'_, Rule>) -> Result<Set> {
    assert_eq!(pair.as_rule(), Rule::Set);
    let sp = span(&pair);
    let items = pair
        .into_inner()
        .filter(|p| p.as_rule() == Rule::SetItem)
        .map(|i| build_set_item(i))
        .collect::<Result<Vec<_>>>()?;
    Ok(Set { items, span: sp })
}

fn build_set_item(pair: Pair<'_, Rule>) -> Result<SetItem> {
    assert_eq!(pair.as_rule(), Rule::SetItem);
    let sp = span(&pair);
    let children: Vec<_> = pair
        .into_inner()
        .filter(|p| p.as_rule() != Rule::SP)
        .collect();

    if children.is_empty() {
        return Err(CypherError {
            kind: ErrorKind::InvalidAssignment {
                reason: "empty set item",
            },
            span: sp,
            source_label: None,
            notes: Vec::new(),
            source: None,
        });
    }

    match children[0].as_rule() {
        Rule::PropertyExpression => {
            let prop = build_property_expression(children[0].clone())?;
            let has_add = children.iter().any(|p| p.as_rule() == Rule::PLUSEQ);
            let operator = if has_add {
                SetOperator::Add
            } else {
                SetOperator::Assign
            };
            let value_expr = children
                .iter()
                .find(|p| p.as_rule() == Rule::Expression)
                .unwrap();
            let value = build_expression(value_expr.clone())?;
            Ok(SetItem::Property {
                property: prop,
                value,
                operator,
            })
        }
        Rule::Variable => {
            let var = build_variable(children[0].clone())?;
            if children.len() >= 2 {
                let has_add = children.iter().any(|p| p.as_rule() == Rule::PLUSEQ);
                let expr_child = children.iter().find(|p| p.as_rule() == Rule::Expression);
                let labels_child = children.iter().find(|p| p.as_rule() == Rule::NodeLabels);
                if let Some(expr) = expr_child {
                    let value = build_expression(expr.clone())?;
                    let operator = if has_add {
                        SetOperator::Add
                    } else {
                        SetOperator::Assign
                    };
                    Ok(SetItem::Variable {
                        variable: var,
                        value,
                        operator,
                    })
                } else if let Some(labels) = labels_child {
                    let labels = build_node_labels(labels.clone())?;
                    Ok(SetItem::Labels {
                        variable: var,
                        labels,
                    })
                } else {
                    Err(unsupported(children[1].as_rule(), sp))
                }
            } else {
                Err(CypherError {
                    kind: ErrorKind::InvalidAssignment {
                        reason: "unexpected end of set item",
                    },
                    span: sp,
                    source_label: None,
                    notes: Vec::new(),
                    source: None,
                })
            }
        }
        _ => Err(unsupported(children[0].as_rule(), sp)),
    }
}

fn build_remove(pair: Pair<'_, Rule>) -> Result<Remove> {
    assert_eq!(pair.as_rule(), Rule::Remove);
    let sp = span(&pair);
    let items = pair
        .into_inner()
        .filter(|p| p.as_rule() == Rule::RemoveItem)
        .map(|i| build_remove_item(i))
        .collect::<Result<Vec<_>>>()?;
    Ok(Remove { items, span: sp })
}

fn build_remove_item(pair: Pair<'_, Rule>) -> Result<RemoveItem> {
    assert_eq!(pair.as_rule(), Rule::RemoveItem);
    let sp = span(&pair);
    let children: Vec<_> = pair
        .into_inner()
        .filter(|p| p.as_rule() != Rule::SP)
        .collect();

    if children.is_empty() {
        return Err(CypherError {
            kind: ErrorKind::InvalidAssignment {
                reason: "empty remove item",
            },
            span: sp,
            source_label: None,
            notes: Vec::new(),
            source: None,
        });
    }

    if children[0].as_rule() == Rule::Variable
        && children.len() >= 2
        && children[1].as_rule() == Rule::NodeLabels
    {
        let var = build_variable(children[0].clone())?;
        let labels = build_node_labels(children[1].clone())?;
        return Ok(RemoveItem::Labels {
            variable: var,
            labels,
        });
    }

    let expr = build_property_expression(children[0].clone())?;
    Ok(RemoveItem::Property(expr))
}

fn build_standalone_call(pair: Pair<'_, Rule>) -> Result<StandaloneCall> {
    assert_eq!(pair.as_rule(), Rule::StandaloneCall);
    let sp = span(&pair);
    let mut call = None;
    let mut yield_items = None;

    for child in pair.into_inner() {
        match child.as_rule() {
            Rule::CALL | Rule::SP => {}
            Rule::ExplicitProcedureInvocation | Rule::ImplicitProcedureInvocation => {
                call = Some(build_procedure_invocation(child)?);
            }
            Rule::YIELD => {}
            Rule::STAR => yield_items = Some(YieldSpec::Star { span: span(&child) }),
            Rule::YieldItems => yield_items = Some(YieldSpec::Items(build_yield_items(child)?)),
            _ => return Err(unsupported(child.as_rule(), sp)),
        }
    }

    Ok(StandaloneCall {
        call: call.ok_or_else(|| CypherError {
            kind: ErrorKind::MissingClause {
                clause: "procedure call",
                after: "CALL",
            },
            span: sp,
            source_label: None,
            notes: Vec::new(),
            source: None,
        })?,
        yield_items,
        span: sp,
    })
}

fn build_in_query_call(pair: Pair<'_, Rule>) -> Result<InQueryCall> {
    assert_eq!(pair.as_rule(), Rule::InQueryCall);
    let sp = span(&pair);
    let mut inner = pair.into_inner();
    let call = build_procedure_invocation(inner.next().unwrap())?;
    let yield_items = inner
        .find(|p| p.as_rule() == Rule::YieldItems)
        .map(build_yield_items)
        .transpose()?;
    Ok(InQueryCall {
        call,
        yield_items,
        span: sp,
    })
}

fn build_procedure_invocation(pair: Pair<'_, Rule>) -> Result<ProcedureInvocation> {
    let sp = span(&pair);
    let mut name_parts: Vec<SymbolicName> = Vec::new();
    let mut args = Vec::new();
    let mut distinct = false;
    let mut in_args = false;

    for child in pair.into_inner() {
        match child.as_rule() {
            Rule::Namespace if !in_args => {
                name_parts.extend(extract_symbolic_names(child));
            }
            Rule::SymbolicName if !in_args => {
                name_parts.push(build_symbolic_name(child)?);
            }
            Rule::Expression => {
                in_args = true;
                args.push(build_expression(child)?);
            }
            Rule::DISTINCT => distinct = true,
            Rule::ExplicitProcedureInvocation
            | Rule::ImplicitProcedureInvocation
            | Rule::ProcedureName
            | Rule::FunctionName
            | Rule::SP => {}
            _ => {}
        }
    }

    let func_name = name_parts.pop().unwrap_or_else(|| SymbolicName {
        name: String::new(),
        span: sp,
    });

    Ok(ProcedureInvocation {
        name: FunctionInvocation {
            name: [name_parts, vec![func_name]].concat(),
            distinct,
            arguments: args,
            span: sp,
        },
        span: sp,
    })
}

fn extract_symbolic_names(pair: Pair<'_, Rule>) -> Vec<SymbolicName> {
    match pair.as_rule() {
        Rule::SymbolicName => vec![build_symbolic_name(pair).unwrap()],
        Rule::Namespace => pair
            .into_inner()
            .flat_map(|p| extract_symbolic_names(p))
            .collect(),
        _ => vec![],
    }
}

fn build_yield_items(pair: Pair<'_, Rule>) -> Result<YieldItems> {
    assert_eq!(pair.as_rule(), Rule::YieldItems);
    let sp = span(&pair);
    let mut items = Vec::new();
    let mut where_clause = None;

    for child in pair.into_inner() {
        match child.as_rule() {
            Rule::YieldItem => items.push(build_yield_item(child)?),
            Rule::Where => where_clause = Some(build_where(child)?),
            Rule::SP | Rule::YIELD => {}
            _ => return Err(unsupported(child.as_rule(), sp)),
        }
    }

    Ok(YieldItems {
        items,
        where_clause,
    })
}

fn build_yield_item(pair: Pair<'_, Rule>) -> Result<YieldItem> {
    assert_eq!(pair.as_rule(), Rule::YieldItem);
    let sp = span(&pair);
    let mut procedure_field = None;
    let mut alias = None;

    for child in pair.clone().into_inner() {
        match child.as_rule() {
            Rule::ProcedureResultField => {
                procedure_field = Some(build_symbolic_name(child.into_inner().next().unwrap())?);
            }
            Rule::Variable => alias = Some(build_variable(child)?),
            Rule::AS | Rule::SP => {}
            _ => {}
        }
    }

    let pf = procedure_field.unwrap_or_else(|| SymbolicName {
        name: String::new(),
        span: sp,
    });

    Ok(YieldItem {
        procedure_field: pf,
        alias,
    })
}

fn build_with(pair: Pair<'_, Rule>) -> Result<With> {
    assert_eq!(pair.as_rule(), Rule::With);
    let sp = span(&pair);
    let projection = pair
        .clone()
        .into_inner()
        .find(|p| p.as_rule() == Rule::ProjectionBody)
        .map(|p| build_projection_body(p))
        .transpose()?
        .unwrap();
    let where_clause = pair
        .into_inner()
        .find(|p| p.as_rule() == Rule::Where)
        .map(build_where)
        .transpose()?;
    Ok(With {
        distinct: projection.0,
        items: projection.1,
        order: projection.2,
        skip: projection.3,
        limit: projection.4,
        where_clause,
        span: sp,
    })
}

fn build_return(pair: Pair<'_, Rule>) -> Result<Return> {
    assert_eq!(pair.as_rule(), Rule::Return);
    let sp = span(&pair);
    let projection = pair
        .into_inner()
        .find(|p| p.as_rule() == Rule::ProjectionBody)
        .map(build_projection_body)
        .transpose()?
        .unwrap();
    Ok(Return {
        distinct: projection.0,
        items: projection.1,
        order: projection.2,
        skip: projection.3,
        limit: projection.4,
        span: sp,
    })
}

type ProjectionParts = (
    bool,
    Vec<ProjectionItem>,
    Option<Order>,
    Option<Expression>,
    Option<Expression>,
);

fn build_projection_body(pair: Pair<'_, Rule>) -> Result<ProjectionParts> {
    assert_eq!(pair.as_rule(), Rule::ProjectionBody);
    let sp = span(&pair);
    let mut distinct = false;
    let mut items = Vec::new();
    let mut order = None;
    let mut skip = None;
    let mut limit = None;

    for child in pair.into_inner() {
        match child.as_rule() {
            Rule::DISTINCT => distinct = true,
            Rule::ProjectionItems => items = build_projection_items(child)?,
            Rule::Order => order = Some(build_order(child)?),
            Rule::Skip => {
                skip = Some(build_expression(
                    child
                        .into_inner()
                        .find(|p| p.as_rule() == Rule::Expression)
                        .unwrap(),
                )?)
            }
            Rule::Limit => {
                limit = Some(build_expression(
                    child
                        .into_inner()
                        .find(|p| p.as_rule() == Rule::Expression)
                        .unwrap(),
                )?)
            }
            Rule::SP => {}
            _ => return Err(unsupported(child.as_rule(), sp)),
        }
    }

    Ok((distinct, items, order, skip, limit))
}

fn build_projection_items(pair: Pair<'_, Rule>) -> Result<Vec<ProjectionItem>> {
    assert_eq!(pair.as_rule(), Rule::ProjectionItems);
    pair.into_inner()
        .filter(|p| p.as_rule() == Rule::ProjectionItem)
        .map(|p| build_projection_item(p))
        .collect()
}

fn build_projection_item(pair: Pair<'_, Rule>) -> Result<ProjectionItem> {
    assert_eq!(pair.as_rule(), Rule::ProjectionItem);
    let children: Vec<_> = pair
        .into_inner()
        .filter(|p| p.as_rule() != Rule::SP)
        .collect();
    let expr = build_expression(children[0].clone())?;
    let alias = children
        .iter()
        .find(|p| p.as_rule() == Rule::Variable)
        .map(|p| build_variable(p.clone()))
        .transpose()?;
    Ok(ProjectionItem {
        expression: expr,
        alias,
    })
}

fn build_order(pair: Pair<'_, Rule>) -> Result<Order> {
    assert_eq!(pair.as_rule(), Rule::Order);
    let items = pair
        .into_inner()
        .filter(|p| p.as_rule() == Rule::SortItem)
        .map(|p| build_sort_item(p))
        .collect::<Result<Vec<_>>>()?;
    Ok(Order { items })
}

fn build_sort_item(pair: Pair<'_, Rule>) -> Result<SortItem> {
    assert_eq!(pair.as_rule(), Rule::SortItem);
    let children: Vec<_> = pair
        .into_inner()
        .filter(|p| p.as_rule() != Rule::SP)
        .collect();
    let expr = build_expression(children[0].clone())?;
    let direction = children.get(1).and_then(|p| match p.as_rule() {
        Rule::ASC | Rule::ASCENDING => Some(SortDirection::Ascending),
        Rule::DESC | Rule::DESCENDING => Some(SortDirection::Descending),
        _ => None,
    });
    Ok(SortItem {
        expression: expr,
        direction,
    })
}

fn build_where(pair: Pair<'_, Rule>) -> Result<Expression> {
    assert_eq!(pair.as_rule(), Rule::Where);
    let inner = pair
        .into_inner()
        .find(|p| p.as_rule() == Rule::Expression)
        .unwrap();
    build_expression(inner)
}

fn build_pattern(pair: Pair<'_, Rule>) -> Result<Pattern> {
    assert_eq!(pair.as_rule(), Rule::Pattern);
    let sp = span(&pair);
    let parts = pair
        .into_inner()
        .filter(|p| p.as_rule() == Rule::PatternPart)
        .map(|p| build_pattern_part(p))
        .collect::<Result<Vec<_>>>()?;
    Ok(Pattern { parts, span: sp })
}

fn build_pattern_part(pair: Pair<'_, Rule>) -> Result<PatternPart> {
    assert_eq!(pair.as_rule(), Rule::PatternPart);
    let sp = span(&pair);
    let mut variable = None;
    let mut anonymous = None;

    for child in pair.clone().into_inner() {
        match child.as_rule() {
            Rule::Variable => variable = Some(build_variable(child)?),
            Rule::AnonymousPatternPart => anonymous = Some(build_anonymous_pattern_part(child)?),
            Rule::SP => {}
            _ => return Err(unsupported(child.as_rule(), sp)),
        }
    }

    Ok(PatternPart {
        variable,
        anonymous: anonymous.ok_or_else(|| CypherError {
            kind: ErrorKind::Internal {
                message: "missing anonymous pattern part".into(),
            },
            span: sp,
            source_label: None,
            notes: Vec::new(),
            source: None,
        })?,
        span: sp,
    })
}

fn build_anonymous_pattern_part(pair: Pair<'_, Rule>) -> Result<AnonymousPatternPart> {
    assert_eq!(pair.as_rule(), Rule::AnonymousPatternPart);
    let inner = pair.into_inner().next().unwrap();
    let element = build_pattern_element(inner)?;
    Ok(AnonymousPatternPart { element })
}

fn build_pattern_element(pair: Pair<'_, Rule>) -> Result<PatternElement> {
    assert_eq!(pair.as_rule(), Rule::PatternElement);
    let sp = span(&pair);
    let children: Vec<_> = pair
        .into_inner()
        .filter(|p| p.as_rule() != Rule::SP)
        .collect();

    if children.is_empty() {
        return Err(CypherError {
            kind: ErrorKind::Internal {
                message: "empty pattern element".into(),
            },
            span: sp,
            source_label: None,
            notes: Vec::new(),
            source: None,
        });
    }

    if children.len() == 1 && children[0].as_rule() == Rule::PatternElement {
        let elem = build_pattern_element(children[0].clone())?;
        return Ok(PatternElement::Parenthesized(Box::new(elem)));
    }

    let start = build_node_pattern(children[0].clone())?;
    let chains = children
        .into_iter()
        .skip(1)
        .map(|p| build_pattern_element_chain(p))
        .collect::<Result<Vec<_>>>()?;
    Ok(PatternElement::Path { start, chains })
}

fn build_node_pattern(pair: Pair<'_, Rule>) -> Result<NodePattern> {
    assert_eq!(pair.as_rule(), Rule::NodePattern);
    let sp = span(&pair);
    let mut variable = None;
    let mut labels = Vec::new();
    let mut properties = None;

    for child in pair.into_inner() {
        match child.as_rule() {
            Rule::Variable => variable = Some(build_variable(child)?),
            Rule::NodeLabels => labels = build_node_labels(child)?,
            Rule::Properties => properties = Some(build_properties(child)?),
            Rule::SP => {}
            _ => return Err(unsupported(child.as_rule(), sp)),
        }
    }

    Ok(NodePattern {
        variable,
        labels,
        properties,
        span: sp,
    })
}

fn build_pattern_element_chain(pair: Pair<'_, Rule>) -> Result<PatternElementChain> {
    assert_eq!(pair.as_rule(), Rule::PatternElementChain);
    let children: Vec<_> = pair
        .into_inner()
        .filter(|p| p.as_rule() != Rule::SP)
        .collect();
    let relationship = build_relationship_pattern(children[0].clone())?;
    let node = build_node_pattern(children[1].clone())?;
    Ok(PatternElementChain { relationship, node })
}

fn build_relationship_pattern(pair: Pair<'_, Rule>) -> Result<RelationshipPattern> {
    assert_eq!(pair.as_rule(), Rule::RelationshipPattern);
    let sp = span(&pair);
    let mut direction = RelationshipDirection::Undirected;
    let mut detail = None;
    let mut has_left = false;
    let mut has_right = false;

    for child in pair.into_inner() {
        match child.as_rule() {
            Rule::LeftArrowHead => has_left = true,
            Rule::RightArrowHead => has_right = true,
            Rule::Dash | Rule::SP => {}
            Rule::RelationshipDetail => detail = Some(build_relationship_detail(child)?),
            _ => return Err(unsupported(child.as_rule(), sp)),
        }
    }

    if has_left {
        direction = RelationshipDirection::Left;
    } else if has_right {
        direction = RelationshipDirection::Right;
    }

    Ok(RelationshipPattern {
        direction,
        detail,
        span: sp,
    })
}

fn build_relationship_detail(pair: Pair<'_, Rule>) -> Result<RelationshipDetail> {
    assert_eq!(pair.as_rule(), Rule::RelationshipDetail);
    let sp = span(&pair);
    let mut variable = None;
    let mut types = Vec::new();
    let mut range = None;
    let mut properties = None;

    for child in pair.into_inner() {
        match child.as_rule() {
            Rule::Variable => variable = Some(build_variable(child)?),
            Rule::RelationshipTypes => types = build_relationship_types(child)?,
            Rule::RangeLiteral => range = Some(build_range_literal(child)?),
            Rule::Properties => properties = Some(build_properties(child)?),
            Rule::SP => {}
            _ => return Err(unsupported(child.as_rule(), sp)),
        }
    }

    Ok(RelationshipDetail {
        variable,
        types,
        range,
        properties,
        span: sp,
    })
}

fn build_relationship_types(pair: Pair<'_, Rule>) -> Result<Vec<RelTypeName>> {
    assert_eq!(pair.as_rule(), Rule::RelationshipTypes);
    pair.into_inner()
        .filter(|p| p.as_rule() == Rule::RelTypeName)
        .map(build_rel_type_name)
        .collect()
}

fn build_range_literal(pair: Pair<'_, Rule>) -> Result<RangeLiteral> {
    assert_eq!(pair.as_rule(), Rule::RangeLiteral);
    let sp = span(&pair);
    let children: Vec<_> = pair
        .into_inner()
        .filter(|p| {
            p.as_rule() != Rule::MULTIPLY && p.as_rule() != Rule::DOT_DOT && p.as_rule() != Rule::SP
        })
        .collect();

    let start = children.first().and_then(|p| parse_integer(p.as_str()));
    let end = children.get(1).and_then(|p| parse_integer(p.as_str()));

    Ok(RangeLiteral {
        start,
        end,
        span: sp,
    })
}

fn parse_integer(s: &str) -> Option<i64> {
    s.trim().parse().ok()
}

fn build_node_labels(pair: Pair<'_, Rule>) -> Result<Vec<SymbolicName>> {
    assert_eq!(pair.as_rule(), Rule::NodeLabels);
    pair.into_inner()
        .filter(|p| p.as_rule() != Rule::SP)
        .map(|p| {
            assert_eq!(p.as_rule(), Rule::NodeLabel);
            let inner = p
                .into_inner()
                .find(|c| c.as_rule() == Rule::LabelName)
                .unwrap();
            build_label_name(inner)
        })
        .collect()
}

fn build_properties(pair: Pair<'_, Rule>) -> Result<Properties> {
    assert_eq!(pair.as_rule(), Rule::Properties);
    let sp = span(&pair);
    let inner = pair.into_inner().next().unwrap();
    match inner.as_rule() {
        Rule::MapLiteral => Ok(Properties::Map(build_map_literal(inner)?)),
        Rule::Parameter => Ok(Properties::Parameter(build_parameter(inner)?)),
        _ => Err(unsupported(inner.as_rule(), sp)),
    }
}

fn build_relationships_pattern(pair: Pair<'_, Rule>) -> Result<RelationshipsPattern> {
    assert_eq!(pair.as_rule(), Rule::RelationshipsPattern);
    let sp = span(&pair);
    let children: Vec<_> = pair
        .into_inner()
        .filter(|p| p.as_rule() != Rule::SP)
        .collect();
    let start = build_node_pattern(children[0].clone())?;
    let chains = children
        .into_iter()
        .skip(1)
        .map(|p| build_pattern_element_chain(p))
        .collect::<Result<Vec<_>>>()?;
    Ok(RelationshipsPattern {
        start,
        chains,
        span: sp,
    })
}

// Expression builders

fn build_expression(pair: Pair<'_, Rule>) -> Result<Expression> {
    match pair.as_rule() {
        Rule::NotExpression => {
            let sp = span(&pair);
            let inner = pair.into_inner();
            let mut not_count = 0;
            let mut inner_expr = None;
            for child in inner {
                match child.as_rule() {
                    Rule::NOT => not_count += 1,
                    Rule::SP => {}
                    _ => {
                        inner_expr = Some(build_expression(child)?);
                    }
                }
            }
            let mut result = inner_expr.ok_or_else(|| CypherError {
                kind: ErrorKind::Internal {
                    message: "empty NotExpression".into(),
                },
                span: sp,
                source_label: None,
                notes: Vec::new(),
                source: None,
            })?;
            for _ in 0..not_count {
                result = Expression::UnaryOp {
                    op: UnaryOperator::Not,
                    operand: Box::new(result),
                    span: sp,
                };
            }
            Ok(result)
        }
        Rule::Expression
        | Rule::OrExpression
        | Rule::XorExpression
        | Rule::AndExpression
        | Rule::ComparisonExpression
        | Rule::AddOrSubtractExpression
        | Rule::MultiplyDivideModuloExpression
        | Rule::PowerOfExpression
        | Rule::UnaryAddOrSubtractExpression
        | Rule::StringListNullOperatorExpression
        | Rule::PropertyOrLabelsExpression => {
            let rule = pair.as_rule();
            let sp = span(&pair);
            let mut inner = pair.into_inner();
            let first = inner.next().ok_or_else(|| CypherError {
                kind: ErrorKind::Internal {
                    message: format!("empty expression rule: {:?}", rule),
                },
                span: sp,
                source_label: None,
                notes: Vec::new(),
                source: None,
            })?;

            let mut expr = build_expression(first)?;

            match rule {
                Rule::OrExpression => {
                    while let Some(child) = inner.next() {
                        if child.as_rule() == Rule::OR {
                            if let Some(next) = inner.next() {
                                let rhs = build_expression(next)?;
                                expr = Expression::BinaryOp {
                                    op: BinaryOperator::Or,
                                    lhs: Box::new(expr),
                                    rhs: Box::new(rhs),
                                    span: sp,
                                };
                            }
                        } else if child.as_rule() != Rule::SP {
                            let rhs = build_expression(child)?;
                            expr = Expression::BinaryOp {
                                op: BinaryOperator::Or,
                                lhs: Box::new(expr),
                                rhs: Box::new(rhs),
                                span: sp,
                            };
                        }
                    }
                }
                Rule::XorExpression => {
                    for child in inner.by_ref() {
                        if child.as_rule() != Rule::SP && child.as_rule() != Rule::XOR {
                            let rhs = build_expression(child)?;
                            expr = Expression::BinaryOp {
                                op: BinaryOperator::Xor,
                                lhs: Box::new(expr),
                                rhs: Box::new(rhs),
                                span: sp,
                            };
                        }
                    }
                }
                Rule::AndExpression => {
                    for child in inner.by_ref() {
                        if child.as_rule() != Rule::SP && child.as_rule() != Rule::AND {
                            let rhs = build_expression(child)?;
                            expr = Expression::BinaryOp {
                                op: BinaryOperator::And,
                                lhs: Box::new(expr),
                                rhs: Box::new(rhs),
                                span: sp,
                            };
                        }
                    }
                }
                Rule::ComparisonExpression => {
                    let mut operators = Vec::new();
                    for child in inner.by_ref() {
                        if child.as_rule() == Rule::PartialComparisonExpression {
                            let (op, rhs) = build_partial_comparison(child)?;
                            operators.push((op, rhs));
                        }
                    }
                    if !operators.is_empty() {
                        expr = Expression::Comparison {
                            lhs: Box::new(expr),
                            operators,
                            span: sp,
                        };
                    }
                }
                Rule::AddOrSubtractExpression => {
                    while let Some(child) = inner.next() {
                        if child.as_rule() == Rule::SP {
                            continue;
                        }
                        let op = match child.as_rule() {
                            Rule::ADD | Rule::PLUS => Some(BinaryOperator::Add),
                            Rule::SUBTRACT => Some(BinaryOperator::Subtract),
                            Rule::Expression => Some(BinaryOperator::Add),
                            _ => None,
                        };
                        if let Some(op) = op {
                            let rhs = if child.as_rule() == Rule::Expression {
                                build_expression(child)?
                            } else {
                                while let Some(next) = inner.peek() {
                                    if next.as_rule() == Rule::SP {
                                        inner.next();
                                    } else {
                                        break;
                                    }
                                }
                                if let Some(next) = inner.next() {
                                    build_expression(next)?
                                } else {
                                    continue;
                                }
                            };
                            expr = Expression::BinaryOp {
                                op,
                                lhs: Box::new(expr),
                                rhs: Box::new(rhs),
                                span: sp,
                            };
                        }
                    }
                }
                Rule::MultiplyDivideModuloExpression => {
                    while let Some(child) = inner.next() {
                        if child.as_rule() == Rule::SP {
                            continue;
                        }
                        let op = match child.as_rule() {
                            Rule::MULTIPLY => Some(BinaryOperator::Multiply),
                            Rule::DIVIDE => Some(BinaryOperator::Divide),
                            Rule::MODULO => Some(BinaryOperator::Modulo),
                            Rule::Expression => Some(BinaryOperator::Multiply),
                            _ => None,
                        };
                        if let Some(op) = op {
                            let rhs = if child.as_rule() == Rule::Expression {
                                build_expression(child)?
                            } else {
                                while let Some(next) = inner.peek() {
                                    if next.as_rule() == Rule::SP {
                                        inner.next();
                                    } else {
                                        break;
                                    }
                                }
                                if let Some(next) = inner.next() {
                                    build_expression(next)?
                                } else {
                                    continue;
                                }
                            };
                            expr = Expression::BinaryOp {
                                op,
                                lhs: Box::new(expr),
                                rhs: Box::new(rhs),
                                span: sp,
                            };
                        }
                    }
                }
                Rule::PowerOfExpression => {
                    while let Some(child) = inner.next() {
                        if child.as_rule() == Rule::SP || child.as_rule() == Rule::POW {
                            if child.as_rule() == Rule::POW {
                                while let Some(next) = inner.peek() {
                                    if next.as_rule() == Rule::SP {
                                        inner.next();
                                    } else {
                                        break;
                                    }
                                }
                                if let Some(next) = inner.next() {
                                    let rhs = build_expression(next)?;
                                    expr = Expression::BinaryOp {
                                        op: BinaryOperator::Power,
                                        lhs: Box::new(expr),
                                        rhs: Box::new(rhs),
                                        span: sp,
                                    };
                                }
                            }
                            continue;
                        }
                        let rhs = build_expression(child)?;
                        expr = Expression::BinaryOp {
                            op: BinaryOperator::Power,
                            lhs: Box::new(expr),
                            rhs: Box::new(rhs),
                            span: sp,
                        };
                    }
                }
                Rule::UnaryAddOrSubtractExpression => {
                    let mut ops = Vec::new();
                    while let Some(child) = inner.peek() {
                        match child.as_rule() {
                            Rule::ADD | Rule::PLUS => {
                                ops.push(UnaryOperator::Plus);
                                inner.next();
                            }
                            Rule::SUBTRACT => {
                                ops.push(UnaryOperator::Negate);
                                inner.next();
                            }
                            Rule::SP => {
                                inner.next();
                            }
                            _ => break,
                        }
                    }
                    if let Some(base) = inner.next() {
                        let mut result = build_expression(base)?;
                        for op in ops.into_iter().rev() {
                            result = Expression::UnaryOp {
                                op,
                                operand: Box::new(result),
                                span: sp,
                            };
                        }
                        expr = result;
                    }
                }
                Rule::StringListNullOperatorExpression => {
                    for child in inner.by_ref() {
                        match child.as_rule() {
                            Rule::StringOperatorExpression => {
                                expr = build_string_op(expr, child)?;
                            }
                            Rule::ListOperatorExpression => {
                                expr = build_list_op(expr, child)?;
                            }
                            Rule::NullOperatorExpression => {
                                expr = build_null_op(expr, child)?;
                            }
                            Rule::SP => {}
                            _ => return Err(unsupported(child.as_rule(), sp)),
                        }
                    }
                }
                Rule::PropertyOrLabelsExpression => {
                    for child in inner {
                        match child.as_rule() {
                            Rule::PropertyLookup => {
                                let key = build_property_key_name_from_lookup(child)?;
                                expr = Expression::PropertyLookup {
                                    base: Box::new(expr),
                                    property: key,
                                    span: sp,
                                };
                            }
                            Rule::NodeLabels => {
                                let labels = build_node_labels(child)?;
                                expr = Expression::NodeLabels {
                                    base: Box::new(expr),
                                    labels,
                                    span: sp,
                                };
                            }
                            Rule::SP => {}
                            _ => return Err(unsupported(child.as_rule(), sp)),
                        }
                    }
                }
                _ => {}
            }

            Ok(expr)
        }
        Rule::PartialComparisonExpression => {
            let sp = span(&pair);
            let (op, rhs) = build_partial_comparison(pair)?;
            Ok(Expression::Comparison {
                lhs: Box::new(Expression::Literal(Literal::Null)),
                operators: vec![(op, rhs)],
                span: sp,
            })
        }
        Rule::ListOperatorExpression
        | Rule::StringOperatorExpression
        | Rule::NullOperatorExpression => Err(CypherError {
            kind: ErrorKind::Internal {
                message: "operator expression must be preceded by a base expression".into(),
            },
            span: span(&pair),
            source_label: None,
            notes: Vec::new(),
            source: None,
        }),
        Rule::PropertyLookup => {
            let sp = span(&pair);
            let key = build_property_key_name_from_lookup(pair)?;
            Ok(Expression::PropertyLookup {
                base: Box::new(Expression::Literal(Literal::Null)),
                property: key,
                span: sp,
            })
        }
        Rule::Atom => {
            let inner = pair.into_inner().next().unwrap();
            build_atom(inner)
        }
        Rule::Literal => {
            let inner = pair.into_inner().next().unwrap();
            build_literal(inner)
        }
        Rule::NumberLiteral => {
            let lit = build_number_literal(pair)?;
            Ok(Expression::Literal(Literal::Number(lit)))
        }
        Rule::StringLiteral => {
            let lit = build_string_literal(pair)?;
            Ok(Expression::Literal(Literal::String(lit)))
        }
        Rule::BooleanLiteral => {
            let lit = pair.as_str().trim().to_uppercase() == "TRUE";
            Ok(Expression::Literal(Literal::Boolean(lit)))
        }
        Rule::NULL => Ok(Expression::Literal(Literal::Null)),
        Rule::ListLiteral => {
            let lit = build_list_literal(pair)?;
            Ok(Expression::Literal(Literal::List(lit)))
        }
        Rule::MapLiteral => {
            let lit = build_map_literal(pair)?;
            Ok(Expression::Literal(Literal::Map(lit)))
        }
        Rule::Variable => {
            let var = build_variable(pair)?;
            Ok(Expression::Variable(var))
        }
        Rule::Parameter => {
            let param = build_parameter(pair)?;
            Ok(Expression::Parameter(param))
        }
        Rule::FunctionInvocation => {
            let func = build_function_invocation(pair)?;
            Ok(Expression::FunctionCall(func))
        }
        Rule::CaseExpression => {
            let case = build_case_expression(pair)?;
            Ok(Expression::Case(case))
        }
        Rule::ListComprehension => {
            let lc = build_list_comprehension(pair)?;
            Ok(Expression::ListComprehension(Box::new(lc)))
        }
        Rule::PatternComprehension => {
            let pc = build_pattern_comprehension(pair)?;
            Ok(Expression::PatternComprehension(Box::new(pc)))
        }
        Rule::FilterExpression => {
            let fe = build_filter_expression(pair)?;
            Ok(Expression::Any(Box::new(fe)))
        }
        Rule::RelationshipsPattern => {
            let rp = build_relationships_pattern(pair)?;
            Ok(Expression::Pattern(rp))
        }
        Rule::ParenthesizedExpression => {
            let inner = pair
                .into_inner()
                .find(|p| p.as_rule() == Rule::Expression)
                .unwrap();
            let expr = build_expression(inner)?;
            Ok(Expression::Parenthesized(Box::new(expr)))
        }
        Rule::ExistentialSubquery => {
            let exists = build_exists_expression(pair)?;
            Ok(Expression::Exists(Box::new(exists)))
        }
        Rule::ALL | Rule::ANY_ | Rule::NONE | Rule::SINGLE => {
            let kind = pair.as_rule();
            build_quantified_expression(pair, kind)
        }
        _ => Err(unsupported(pair.as_rule(), span(&pair))),
    }
}

fn build_string_op(lhs: Expression, pair: Pair<'_, Rule>) -> Result<Expression> {
    assert_eq!(pair.as_rule(), Rule::StringOperatorExpression);
    let sp = span(&pair);
    let mut op = ComparisonOperator::Contains;
    let mut rhs = None;

    for child in pair.clone().into_inner() {
        match child.as_rule() {
            Rule::STARTS => op = ComparisonOperator::StartsWith,
            Rule::ENDS => op = ComparisonOperator::EndsWith,
            Rule::CONTAINS => op = ComparisonOperator::Contains,
            Rule::PropertyOrLabelsExpression => {
                rhs = Some(build_property_or_labels_expression(child)?)
            }
            Rule::SP | Rule::WITH => {}
            _ => {}
        }
    }

    let rhs = rhs.ok_or_else(|| CypherError {
        kind: ErrorKind::Internal {
            message: "missing rhs in string operator".into(),
        },
        span: sp,
        source_label: None,
        notes: Vec::new(),
        source: None,
    })?;

    Ok(Expression::Comparison {
        lhs: Box::new(lhs),
        operators: vec![(op, Box::new(rhs))],
        span: sp,
    })
}

fn build_list_op(lhs: Expression, pair: Pair<'_, Rule>) -> Result<Expression> {
    assert_eq!(pair.as_rule(), Rule::ListOperatorExpression);
    let sp = span(&pair);
    let children: Vec<_> = pair
        .into_inner()
        .filter(|p| p.as_rule() != Rule::SP)
        .collect();

    if children.is_empty() {
        return Ok(lhs);
    }

    match children[0].as_rule() {
        Rule::IN => {
            let rhs = build_expression(children[1].clone())?;
            Ok(Expression::In {
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
                span: sp,
            })
        }
        Rule::DOT_DOT => {
            let end = if children.len() > 1 {
                Some(Box::new(build_expression(children[1].clone())?))
            } else {
                None
            };
            Ok(Expression::ListSlice {
                list: Box::new(lhs),
                start: None,
                end,
                span: sp,
            })
        }
        Rule::Expression => {
            if children.len() == 1 {
                let idx = build_expression(children[0].clone())?;
                Ok(Expression::ListIndex {
                    list: Box::new(lhs),
                    index: Box::new(idx),
                    span: sp,
                })
            } else {
                let end = if children.len() > 2 {
                    Some(Box::new(build_expression(children[2].clone())?))
                } else {
                    None
                };
                Ok(Expression::ListSlice {
                    list: Box::new(lhs),
                    start: Some(Box::new(build_expression(children[0].clone())?)),
                    end,
                    span: sp,
                })
            }
        }
        _ => Ok(lhs),
    }
}

fn build_null_op(lhs: Expression, pair: Pair<'_, Rule>) -> Result<Expression> {
    assert_eq!(pair.as_rule(), Rule::NullOperatorExpression);
    let sp = span(&pair);
    let negated = pair.into_inner().any(|c| c.as_rule() == Rule::NOT);
    Ok(Expression::IsNull {
        operand: Box::new(lhs),
        negated,
        span: sp,
    })
}

fn build_property_or_labels_expression(pair: Pair<'_, Rule>) -> Result<Expression> {
    assert_eq!(pair.as_rule(), Rule::PropertyOrLabelsExpression);
    let sp = span(&pair);
    let mut inner = pair.into_inner();
    let mut expr = build_expression(inner.next().unwrap())?;

    for child in inner {
        match child.as_rule() {
            Rule::PropertyLookup => {
                let key = build_property_key_name_from_lookup(child)?;
                expr = Expression::PropertyLookup {
                    base: Box::new(expr),
                    property: key,
                    span: sp,
                };
            }
            Rule::NodeLabels => {
                let labels = build_node_labels(child)?;
                expr = Expression::NodeLabels {
                    base: Box::new(expr),
                    labels,
                    span: sp,
                };
            }
            Rule::SP => {}
            _ => return Err(unsupported(child.as_rule(), sp)),
        }
    }

    Ok(expr)
}

fn build_property_expression(pair: Pair<'_, Rule>) -> Result<Expression> {
    assert_eq!(pair.as_rule(), Rule::PropertyExpression);
    let sp = span(&pair);
    let mut inner = pair.into_inner();
    let mut expr = build_expression(inner.next().unwrap())?;

    for child in inner {
        if child.as_rule() == Rule::PropertyLookup {
            let key = build_property_key_name_from_lookup(child)?;
            expr = Expression::PropertyLookup {
                base: Box::new(expr),
                property: key,
                span: sp,
            };
        }
    }

    Ok(expr)
}

fn build_atom(pair: Pair<'_, Rule>) -> Result<Expression> {
    match pair.as_rule() {
        Rule::Literal => {
            let inner = pair.into_inner().next().unwrap();
            build_literal(inner)
        }
        Rule::Parameter => {
            let param = build_parameter(pair)?;
            Ok(Expression::Parameter(param))
        }
        Rule::CaseExpression => {
            let case = build_case_expression(pair)?;
            Ok(Expression::Case(case))
        }
        Rule::ListComprehension => {
            let lc = build_list_comprehension(pair)?;
            Ok(Expression::ListComprehension(Box::new(lc)))
        }
        Rule::PatternComprehension => {
            let pc = build_pattern_comprehension(pair)?;
            Ok(Expression::PatternComprehension(Box::new(pc)))
        }
        Rule::RelationshipsPattern => {
            let rp = build_relationships_pattern(pair)?;
            Ok(Expression::Pattern(rp))
        }
        Rule::FunctionInvocation => {
            let func = build_function_invocation(pair)?;
            Ok(Expression::FunctionCall(func))
        }
        Rule::ExistentialSubquery => {
            let exists = build_exists_expression(pair)?;
            Ok(Expression::Exists(Box::new(exists)))
        }
        Rule::Variable => {
            let var = build_variable(pair)?;
            Ok(Expression::Variable(var))
        }
        Rule::COUNT => {
            let sp = span(&pair);
            Ok(Expression::CountStar { span: sp })
        }
        Rule::ALL | Rule::ANY_ | Rule::NONE | Rule::SINGLE => {
            let kind = pair.as_rule();
            build_quantified_expression(pair, kind)
        }
        Rule::ParenthesizedExpression => {
            let inner = pair
                .into_inner()
                .find(|p| p.as_rule() == Rule::Expression)
                .unwrap();
            let expr = build_expression(inner)?;
            Ok(Expression::Parenthesized(Box::new(expr)))
        }
        Rule::MapProjection => {
            let mp = build_map_projection(pair)?;
            Ok(Expression::MapProjection(Box::new(mp)))
        }
        _ => Err(unsupported(pair.as_rule(), span(&pair))),
    }
}

fn build_quantified_expression(pair: Pair<'_, Rule>, kind: Rule) -> Result<Expression> {
    let sp = span(&pair);
    let filter = build_filter_expression(pair)?;
    match kind {
        Rule::ALL => Ok(Expression::All(Box::new(filter))),
        Rule::ANY_ => Ok(Expression::Any(Box::new(filter))),
        Rule::NONE => Ok(Expression::None(Box::new(filter))),
        Rule::SINGLE => Ok(Expression::Single(Box::new(filter))),
        _ => Err(unsupported(kind, sp)),
    }
}

fn build_literal(pair: Pair<'_, Rule>) -> Result<Expression> {
    match pair.as_rule() {
        Rule::NumberLiteral => {
            let lit = build_number_literal(pair)?;
            Ok(Expression::Literal(Literal::Number(lit)))
        }
        Rule::StringLiteral => {
            let lit = build_string_literal(pair)?;
            Ok(Expression::Literal(Literal::String(lit)))
        }
        Rule::BooleanLiteral => {
            let b = pair.as_str().trim().to_uppercase() == "TRUE";
            Ok(Expression::Literal(Literal::Boolean(b)))
        }
        Rule::NULL => Ok(Expression::Literal(Literal::Null)),
        Rule::MapLiteral => {
            let lit = build_map_literal(pair)?;
            Ok(Expression::Literal(Literal::Map(lit)))
        }
        Rule::ListLiteral => {
            let lit = build_list_literal(pair)?;
            Ok(Expression::Literal(Literal::List(lit)))
        }
        _ => Err(unsupported(pair.as_rule(), span(&pair))),
    }
}

fn build_partial_comparison(pair: Pair<'_, Rule>) -> Result<(ComparisonOperator, Box<Expression>)> {
    assert_eq!(pair.as_rule(), Rule::PartialComparisonExpression);
    let sp = span(&pair);
    let children: Vec<_> = pair
        .into_inner()
        .filter(|p| p.as_rule() != Rule::SP)
        .collect();
    let op_pair = &children[0];
    let op = match op_pair.as_rule() {
        Rule::EQ => ComparisonOperator::Eq,
        Rule::NE => ComparisonOperator::Ne,
        Rule::LT => ComparisonOperator::Lt,
        Rule::GT => ComparisonOperator::Gt,
        Rule::LE => ComparisonOperator::Le,
        Rule::GE => ComparisonOperator::Ge,
        _ => return Err(unsupported(op_pair.as_rule(), sp)),
    };
    let rhs = build_expression(children[1].clone())?;
    Ok((op, Box::new(rhs)))
}

fn build_number_literal(pair: Pair<'_, Rule>) -> Result<NumberLiteral> {
    assert_eq!(pair.as_rule(), Rule::NumberLiteral);
    let sp = span(&pair);
    let inner = pair.clone().into_inner().next().unwrap();
    match inner.as_rule() {
        Rule::IntegerLiteral => {
            let s = inner.as_str();
            let raw = s.to_string();
            let val = if s.starts_with("0x") || s.starts_with("0X") {
                i64::from_str_radix(&s[2..], 16).map_err(|_| CypherError {
                    kind: ErrorKind::InvalidNumber {
                        raw,
                        reason: crate::error::NumberReason::Overflow,
                    },
                    span: sp,
                    source_label: None,
                    notes: Vec::new(),
                    source: None,
                })?
            } else if s.starts_with('0') && s.len() > 1 && s.chars().all(|c| c.is_ascii_digit()) {
                i64::from_str_radix(s, 8).map_err(|_| CypherError {
                    kind: ErrorKind::InvalidNumber {
                        raw,
                        reason: crate::error::NumberReason::Overflow,
                    },
                    span: sp,
                    source_label: None,
                    notes: Vec::new(),
                    source: None,
                })?
            } else {
                s.parse::<i64>().map_err(|_| CypherError {
                    kind: ErrorKind::InvalidNumber {
                        raw,
                        reason: crate::error::NumberReason::Overflow,
                    },
                    span: sp,
                    source_label: None,
                    notes: Vec::new(),
                    source: None,
                })?
            };
            Ok(NumberLiteral::Integer(val))
        }
        Rule::DoubleLiteral => {
            let s = inner.as_str();
            let raw = s.to_string();
            let val = s.parse::<f64>().map_err(|_| CypherError {
                kind: ErrorKind::InvalidNumber {
                    raw: raw.clone(),
                    reason: crate::error::NumberReason::Other,
                },
                span: sp,
                source_label: None,
                notes: Vec::new(),
                source: None,
            })?;
            if val.is_nan() || val.is_infinite() {
                return Err(CypherError {
                    kind: ErrorKind::InvalidNumber {
                        raw,
                        reason: crate::error::NumberReason::Other,
                    },
                    span: sp,
                    source_label: None,
                    notes: Vec::new(),
                    source: None,
                });
            }
            Ok(NumberLiteral::Float(val))
        }
        _ => Err(unsupported(inner.as_rule(), sp)),
    }
}

fn build_string_literal(pair: Pair<'_, Rule>) -> Result<StringLiteral> {
    assert_eq!(pair.as_rule(), Rule::StringLiteral);
    let sp = span(&pair);
    let raw = pair.as_str().to_string();
    let content = if (raw.starts_with('"') && raw.ends_with('"'))
        || (raw.starts_with('\'') && raw.ends_with('\''))
    {
        &raw[1..raw.len() - 1]
    } else {
        raw.as_str()
    };
    let (value, decode_err) = decode_string_content(content, sp);
    if let Some(err) = decode_err {
        return Err(err);
    }
    Ok(StringLiteral {
        value,
        span: sp,
        raw: Some(raw),
    })
}

fn decode_string_content(content: &str, span: Span) -> (String, Option<CypherError>) {
    let mut result = String::with_capacity(content.len());
    let mut chars = content.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.next() {
                Some('\\') => result.push('\\'),
                Some('\'') => result.push('\''),
                Some('"') => result.push('"'),
                Some('b') | Some('B') => result.push('\u{0008}'),
                Some('f') | Some('F') => result.push('\u{000C}'),
                Some('n') | Some('N') => result.push('\n'),
                Some('r') | Some('R') => result.push('\r'),
                Some('t') | Some('T') => result.push('\t'),
                Some('u') | Some('U') => {
                    let mut hex = String::new();
                    let mut count = 0;
                    while count < 8 && chars.peek().is_some() {
                        let next = *chars.peek().unwrap();
                        if next.is_ascii_hexdigit() {
                            hex.push(chars.next().unwrap());
                            count += 1;
                        } else {
                            break;
                        }
                    }
                    if count == 4 || count == 8 {
                        if let Ok(codepoint) = u32::from_str_radix(&hex, 16) {
                            if let Some(c) = char::from_u32(codepoint) {
                                result.push(c);
                            } else {
                                let err_sp = Span::new(span.start, span.end);
                                return (
                                    result,
                                    Some(CypherError {
                                        kind: ErrorKind::InvalidEscape {
                                            sequence: format!("\\u{}", hex),
                                        },
                                        span: err_sp,
                                        source_label: None,
                                        notes: Vec::new(),
                                        source: None,
                                    }),
                                );
                            }
                        }
                    } else {
                        let err_sp = Span::new(span.start, span.end);
                        return (
                            result,
                            Some(CypherError {
                                kind: ErrorKind::InvalidEscape {
                                    sequence: format!("\\u{}", hex),
                                },
                                span: err_sp,
                                source_label: None,
                                notes: Vec::new(),
                                source: None,
                            }),
                        );
                    }
                }
                Some(other) => {
                    result.push('\\');
                    result.push(other);
                }
                None => {
                    result.push('\\');
                }
            }
        } else {
            result.push(ch);
        }
    }
    (result, None)
}

fn build_list_literal(pair: Pair<'_, Rule>) -> Result<ListLiteral> {
    assert_eq!(pair.as_rule(), Rule::ListLiteral);
    let sp = span(&pair);
    let elements = pair
        .into_inner()
        .filter(|p| p.as_rule() == Rule::Expression)
        .map(|p| build_expression(p))
        .collect::<Result<Vec<_>>>()?;
    Ok(ListLiteral { elements, span: sp })
}

fn build_map_literal(pair: Pair<'_, Rule>) -> Result<MapLiteral> {
    assert_eq!(pair.as_rule(), Rule::MapLiteral);
    let sp = span(&pair);
    let mut entries = Vec::new();
    let mut children: Vec<_> = pair
        .into_inner()
        .filter(|p| p.as_rule() != Rule::SP)
        .collect();

    while let Some(child) = children.first() {
        if child.as_rule() == Rule::PropertyKeyName {
            let key = build_property_key_name(children.remove(0))?;
            if let Some(expr_child) = children.first() {
                if expr_child.as_rule() == Rule::Expression {
                    let value = build_expression(children.remove(0))?;
                    entries.push((key, value));
                } else {
                    children.remove(0);
                }
            }
        } else {
            children.remove(0);
        }
    }

    Ok(MapLiteral { entries, span: sp })
}

fn build_parameter(pair: Pair<'_, Rule>) -> Result<Parameter> {
    assert_eq!(pair.as_rule(), Rule::Parameter);
    let sp = span(&pair);
    let mut inner = pair.into_inner();
    let name_pair = inner.next().unwrap();
    let name = match name_pair.as_rule() {
        Rule::SymbolicName => build_symbolic_name(name_pair)?,
        _ => SymbolicName {
            name: name_pair.as_str().to_string(),
            span: span(&name_pair),
        },
    };
    Ok(Parameter { name, span: sp })
}

fn build_function_invocation(pair: Pair<'_, Rule>) -> Result<FunctionInvocation> {
    assert_eq!(pair.as_rule(), Rule::FunctionInvocation);
    let sp = span(&pair);
    let mut inner = pair.into_inner();
    let func_name = inner.next().unwrap();
    let name_parts = extract_function_name_parts(func_name);
    let mut distinct = false;
    let mut arguments = Vec::new();

    for child in inner {
        match child.as_rule() {
            Rule::DISTINCT => distinct = true,
            Rule::Expression => arguments.push(build_expression(child)?),
            Rule::FunctionName | Rule::Namespace | Rule::SP => {}
            _ => {}
        }
    }

    Ok(FunctionInvocation {
        name: name_parts,
        distinct,
        arguments,
        span: sp,
    })
}

fn extract_function_name_parts(pair: Pair<'_, Rule>) -> Vec<SymbolicName> {
    match pair.as_rule() {
        Rule::FunctionName => pair
            .into_inner()
            .flat_map(|p| extract_symbolic_names(p))
            .collect(),
        Rule::SymbolicName => vec![build_symbolic_name(pair).unwrap()],
        Rule::Namespace => pair
            .into_inner()
            .flat_map(|p| extract_symbolic_names(p))
            .collect(),
        _ => vec![],
    }
}

fn build_case_expression(pair: Pair<'_, Rule>) -> Result<CaseExpression> {
    assert_eq!(pair.as_rule(), Rule::CaseExpression);
    let sp = span(&pair);
    let inner = pair.into_inner();
    let mut scrutinee = None;
    let mut alternatives = Vec::new();
    let mut default = None;

    for child in inner {
        match child.as_rule() {
            Rule::CASE | Rule::END | Rule::SP => {}
            Rule::Expression => {
                if alternatives.is_empty() {
                    scrutinee = Some(Box::new(build_expression(child)?));
                } else if default.is_none() {
                    default = Some(Box::new(build_expression(child)?));
                }
            }
            Rule::CaseAlternative => alternatives.push(build_case_alternative(child)?),
            Rule::ELSE => {}
            _ => return Err(unsupported(child.as_rule(), sp)),
        }
    }

    Ok(CaseExpression {
        scrutinee,
        alternatives,
        default,
        span: sp,
    })
}

fn build_case_alternative(pair: Pair<'_, Rule>) -> Result<CaseAlternative> {
    assert_eq!(pair.as_rule(), Rule::CaseAlternative);
    let children: Vec<_> = pair
        .into_inner()
        .filter(|p| {
            p.as_rule() != Rule::SP && p.as_rule() != Rule::WHEN && p.as_rule() != Rule::THEN
        })
        .collect();
    let when = build_expression(children[0].clone())?;
    let then = build_expression(children[1].clone())?;
    Ok(CaseAlternative { when, then })
}

fn build_list_comprehension(pair: Pair<'_, Rule>) -> Result<ListComprehension> {
    assert_eq!(pair.as_rule(), Rule::ListComprehension);
    let sp = span(&pair);
    let pair_clone = pair.clone();
    let filter = build_filter_expression(pair)?;
    let mut map = None;

    for child in pair_clone.into_inner() {
        if child.as_rule() == Rule::Expression {
            map = Some(build_expression(child)?);
        }
    }

    Ok(ListComprehension {
        variable: filter.variable.clone(),
        filter: filter.predicate,
        map,
        span: sp,
    })
}

fn build_pattern_comprehension(pair: Pair<'_, Rule>) -> Result<PatternComprehension> {
    assert_eq!(pair.as_rule(), Rule::PatternComprehension);
    let sp = span(&pair);
    let mut variable = None;
    let mut pattern = None;
    let mut where_clause = None;
    let mut map = None;

    for child in pair.into_inner() {
        match child.as_rule() {
            Rule::Variable => variable = Some(build_variable(child)?),
            Rule::RelationshipsPattern => pattern = Some(build_relationships_pattern(child)?),
            Rule::Where => where_clause = Some(build_where(child)?),
            Rule::Expression => map = Some(build_expression(child)?),
            Rule::SP => {}
            _ => return Err(unsupported(child.as_rule(), sp)),
        }
    }

    Ok(PatternComprehension {
        variable,
        pattern: pattern.ok_or_else(|| CypherError {
            kind: ErrorKind::MissingClause {
                clause: "pattern",
                after: "[",
            },
            span: sp,
            source_label: None,
            notes: Vec::new(),
            source: None,
        })?,
        where_clause,
        map: map.ok_or_else(|| CypherError {
            kind: ErrorKind::MissingClause {
                clause: "map expression",
                after: "|",
            },
            span: sp,
            source_label: None,
            notes: Vec::new(),
            source: None,
        })?,
        span: sp,
    })
}

fn build_filter_expression(pair: Pair<'_, Rule>) -> Result<FilterExpression> {
    assert_eq!(pair.as_rule(), Rule::FilterExpression);
    let sp = span(&pair);
    let pair_clone = pair.clone();
    let id = pair_clone
        .into_inner()
        .find(|p| p.as_rule() == Rule::IdInColl)
        .ok_or_else(|| CypherError {
            kind: ErrorKind::Internal {
                message: "missing IdInColl in FilterExpression".into(),
            },
            span: sp,
            source_label: None,
            notes: Vec::new(),
            source: None,
        })?;
    let children: Vec<_> = id
        .into_inner()
        .filter(|p| p.as_rule() != Rule::SP)
        .collect();
    let variable = build_variable(children[0].clone())?;
    let collection = Box::new(build_expression(children[1].clone())?);
    let predicate = pair
        .into_inner()
        .find(|p| p.as_rule() == Rule::Where)
        .map(|p| build_where(p).map(Box::new))
        .transpose()?;
    Ok(FilterExpression {
        variable,
        collection,
        predicate,
        span: sp,
    })
}

fn build_exists_expression(pair: Pair<'_, Rule>) -> Result<ExistsExpression> {
    assert_eq!(pair.as_rule(), Rule::ExistentialSubquery);
    let sp = span(&pair);
    let children: Vec<_> = pair
        .into_inner()
        .filter(|p| p.as_rule() != Rule::SP && p.as_rule() != Rule::EXISTS)
        .collect();
    let body = children.first().ok_or_else(|| CypherError {
        kind: ErrorKind::Internal {
            message: "missing body in EXISTS".into(),
        },
        span: sp,
        source_label: None,
        notes: Vec::new(),
        source: None,
    })?;

    let inner_val = match body.as_rule() {
        Rule::RegularQuery => {
            let regular = build_regular_query(body.clone())?;
            ExistsInner::RegularQuery(Box::new(regular))
        }
        Rule::Pattern => {
            let pattern = build_pattern(body.clone())?;
            let where_clause = children
                .iter()
                .skip(1)
                .find(|p| p.as_rule() == Rule::Where)
                .map(|p| build_where(p.clone()).map(Box::new))
                .transpose()?;
            ExistsInner::Pattern(pattern, where_clause)
        }
        _ => return Err(unsupported(body.as_rule(), sp)),
    };

    Ok(ExistsExpression {
        inner: Box::new(inner_val),
        span: sp,
    })
}

fn build_symbolic_name(pair: Pair<'_, Rule>) -> Result<SymbolicName> {
    assert_eq!(pair.as_rule(), Rule::SymbolicName);
    let sp = span(&pair);
    let raw = pair.as_str().to_string();
    let inner = pair.into_inner().next();
    let name = match inner {
        Some(p) => match p.as_rule() {
            Rule::UnescapedSymbolicName => p.as_str().to_string(),
            Rule::EscapedSymbolicName => {
                let s = p.as_str();
                s[1..s.len() - 1].to_string()
            }
            _ => p.as_str().to_string(),
        },
        None => raw,
    };
    Ok(SymbolicName { name, span: sp })
}

fn build_variable(pair: Pair<'_, Rule>) -> Result<Variable> {
    assert_eq!(pair.as_rule(), Rule::Variable);
    let inner = pair.into_inner().next().unwrap();
    let name = build_symbolic_name(inner)?;
    Ok(Variable { name })
}

fn build_label_name(pair: Pair<'_, Rule>) -> Result<SymbolicName> {
    assert_eq!(pair.as_rule(), Rule::LabelName);
    let inner = pair.into_inner().next().unwrap();
    build_schema_name(inner)
}

fn build_rel_type_name(pair: Pair<'_, Rule>) -> Result<RelTypeName> {
    assert_eq!(pair.as_rule(), Rule::RelTypeName);
    let inner = pair.into_inner().next().unwrap();
    let sym = build_schema_name(inner)?;
    Ok(RelTypeName { name: sym })
}

fn build_property_key_name(pair: Pair<'_, Rule>) -> Result<PropertyKeyName> {
    assert_eq!(pair.as_rule(), Rule::PropertyKeyName);
    let inner = pair.into_inner().next().unwrap();
    let name = build_schema_name(inner)?;
    Ok(PropertyKeyName { name })
}

fn build_property_key_name_from_lookup(pair: Pair<'_, Rule>) -> Result<PropertyKeyName> {
    assert_eq!(pair.as_rule(), Rule::PropertyLookup);
    let sp = span(&pair);
    let key = pair
        .into_inner()
        .find(|p| p.as_rule() == Rule::PropertyKeyName)
        .ok_or_else(|| CypherError {
            kind: ErrorKind::Internal {
                message: "missing PropertyKeyName in PropertyLookup".into(),
            },
            span: sp,
            source_label: None,
            notes: Vec::new(),
            source: None,
        })?;
    build_property_key_name(key)
}

fn build_schema_name(pair: Pair<'_, Rule>) -> Result<SymbolicName> {
    assert_eq!(pair.as_rule(), Rule::SchemaName);
    let sp = span(&pair);
    let inner = pair.into_inner().next().unwrap();
    match inner.as_rule() {
        Rule::SymbolicName => build_symbolic_name(inner),
        Rule::ReservedWord => Ok(SymbolicName {
            name: inner.as_str().to_string(),
            span: sp,
        }),
        _ => build_symbolic_name(inner),
    }
}

// ── New builder functions for Parsing 1.0 ───────────────────────────

fn build_foreach(pair: Pair<'_, Rule>) -> Result<Foreach> {
    assert_eq!(pair.as_rule(), Rule::Foreach);
    let sp = span(&pair);
    let mut variable = None;
    let mut list = None;
    let mut updates = Vec::new();

    for child in pair.into_inner() {
        match child.as_rule() {
            Rule::Variable => variable = Some(build_variable(child)?),
            Rule::Expression => list = Some(build_expression(child)?),
            Rule::UpdatingClause => updates.push(build_updating_clause(child)?),
            Rule::FOREACH | Rule::IN | Rule::SP => {}
            _ => return Err(unsupported(child.as_rule(), sp)),
        }
    }

    Ok(Foreach {
        variable: variable.ok_or_else(|| CypherError {
            kind: ErrorKind::Internal {
                message: "missing variable in FOREACH".into(),
            },
            span: sp,
            source_label: None,
            notes: Vec::new(),
            source: None,
        })?,
        list: list.ok_or_else(|| CypherError {
            kind: ErrorKind::Internal {
                message: "missing list expression in FOREACH".into(),
            },
            span: sp,
            source_label: None,
            notes: Vec::new(),
            source: None,
        })?,
        updates: updates
            .into_iter()
            .map(ForeachUpdate::from_updating_clause)
            .collect(),
        span: sp,
    })
}

impl ForeachUpdate {
    fn from_updating_clause(uc: UpdatingClause) -> Self {
        match uc {
            UpdatingClause::Create(c) => ForeachUpdate::Create(c),
            UpdatingClause::Merge(m) => ForeachUpdate::Merge(m),
            UpdatingClause::Delete(d) => ForeachUpdate::Delete(d),
            UpdatingClause::Set(s) => ForeachUpdate::Set(s),
            UpdatingClause::Remove(r) => ForeachUpdate::Remove(r),
            UpdatingClause::Foreach(f) => ForeachUpdate::Foreach(f),
        }
    }
}

fn build_call_subquery(pair: Pair<'_, Rule>) -> Result<CallSubquery> {
    assert_eq!(pair.as_rule(), Rule::CallSubquery);
    let sp = span(&pair);
    let mut query = None;
    let mut in_transactions = None;

    for child in pair.into_inner() {
        match child.as_rule() {
            Rule::RegularQuery => query = Some(build_regular_query(child)?),
            Rule::InTransactions => in_transactions = Some(build_in_transactions(child)?),
            Rule::CALL | Rule::SP => {}
            _ => return Err(unsupported(child.as_rule(), sp)),
        }
    }

    Ok(CallSubquery {
        query: query.ok_or_else(|| CypherError {
            kind: ErrorKind::Internal {
                message: "missing subquery body".into(),
            },
            span: sp,
            source_label: None,
            notes: Vec::new(),
            source: None,
        })?,
        in_transactions,
        span: sp,
    })
}

fn build_in_transactions(pair: Pair<'_, Rule>) -> Result<InTransactions> {
    assert_eq!(pair.as_rule(), Rule::InTransactions);
    let sp = span(&pair);
    let mut of_rows = None;
    let mut on_error = None;
    let mut expr_collected = false;

    for child in pair.into_inner() {
        match child.as_rule() {
            Rule::Expression => {
                if !expr_collected {
                    of_rows = Some(build_expression(child)?);
                    expr_collected = true;
                }
            }
            Rule::CONTINUE => on_error = Some(OnErrorBehavior::Continue),
            Rule::BREAK => on_error = Some(OnErrorBehavior::Break),
            Rule::FAIL => on_error = Some(OnErrorBehavior::Fail),
            Rule::IN
            | Rule::TRANSACTIONS
            | Rule::OF
            | Rule::ROWS
            | Rule::ON
            | Rule::ERROR
            | Rule::SP => {}
            _ => return Err(unsupported(child.as_rule(), sp)),
        }
    }

    Ok(InTransactions {
        of_rows,
        on_error,
        span: sp,
    })
}

fn build_map_projection(pair: Pair<'_, Rule>) -> Result<MapProjection> {
    assert_eq!(pair.as_rule(), Rule::MapProjection);
    let sp = span(&pair);
    let mut base = None;
    let mut items = Vec::new();

    for child in pair.into_inner() {
        match child.as_rule() {
            Rule::Variable => base = Some(build_variable(child)?),
            Rule::MapProjectionItem => items.push(build_map_projection_item(child)?),
            Rule::SP => {}
            _ => return Err(unsupported(child.as_rule(), sp)),
        }
    }

    Ok(MapProjection {
        base: base.ok_or_else(|| CypherError {
            kind: ErrorKind::Internal {
                message: "missing base variable in map projection".into(),
            },
            span: sp,
            source_label: None,
            notes: Vec::new(),
            source: None,
        })?,
        items,
        span: sp,
    })
}

fn build_map_projection_item(pair: Pair<'_, Rule>) -> Result<MapProjectionItem> {
    assert_eq!(pair.as_rule(), Rule::MapProjectionItem);
    let sp = span(&pair);
    let children: Vec<_> = pair
        .clone()
        .into_inner()
        .filter(|p| p.as_rule() != Rule::SP)
        .collect();
    if children.is_empty() {
        return Err(unsupported(pair.as_rule(), sp));
    }
    match children[0].as_rule() {
        Rule::STAR => Ok(MapProjectionItem::AllProperties { span: sp }),
        Rule::PropertyKeyName => {
            let key = build_property_key_name(children[0].clone())?;
            let value = children
                .iter()
                .find(|p| p.as_rule() == Rule::Expression)
                .map(|p| build_expression(p.clone()))
                .transpose()?
                .ok_or_else(|| CypherError {
                    kind: ErrorKind::Internal {
                        message: "missing value in map projection item".into(),
                    },
                    span: sp,
                    source_label: None,
                    notes: Vec::new(),
                    source: None,
                })?;
            Ok(MapProjectionItem::Literal { key, value })
        }
        Rule::PropertyLookup => {
            let prop = build_property_key_name_from_lookup(children[0].clone())?;
            Ok(MapProjectionItem::PropertyLookup { property: prop })
        }
        _ => Err(unsupported(children[0].as_rule(), sp)),
    }
}

fn build_schema_command(pair: Pair<'_, Rule>) -> Result<SchemaCommand> {
    assert_eq!(pair.as_rule(), Rule::SchemaCommand);
    let sp = span(&pair);
    let inner = pair.clone().into_inner().next().unwrap();
    match inner.as_rule() {
        Rule::CreateIndex => Ok(SchemaCommand::CreateIndex(build_create_index(inner)?)),
        Rule::DropIndex => Ok(SchemaCommand::DropIndex(build_drop_index(inner)?)),
        Rule::CreateConstraint => Ok(SchemaCommand::CreateConstraint(build_create_constraint(
            inner,
        )?)),
        Rule::DropConstraint => Ok(SchemaCommand::DropConstraint(build_drop_constraint(inner)?)),
        _ => Err(unsupported(inner.as_rule(), sp)),
    }
}

fn build_create_index(pair: Pair<'_, Rule>) -> Result<CreateIndex> {
    assert_eq!(pair.as_rule(), Rule::CreateIndex);
    let sp = span(&pair);
    let mut kind = None;
    let mut if_not_exists = false;
    let mut name = None;
    let mut target = None;
    let mut options = None;
    let mut saw_if = false;

    for child in pair.into_inner() {
        match child.as_rule() {
            Rule::IndexKind => kind = Some(build_index_kind(child)?),
            Rule::INDEX | Rule::CREATE | Rule::SP => {}
            Rule::IF => saw_if = true,
            Rule::NOT => {}
            Rule::EXISTS if saw_if => {
                if_not_exists = true;
                saw_if = false;
            }
            Rule::SymbolicName if target.is_none() => {
                name = Some(build_symbolic_name(child.clone())?);
            }
            Rule::SchemaName => {
                target = Some(build_schema_name(child)?);
            }
            Rule::Options => options = Some(build_options(child)?),
            _ => {}
        }
    }

    // If name and target are the same (only one SymbolicName), check if it's the name or target
    // The grammar: CREATE ~ IndexKind? ~ INDEX ~ (IF NOT EXISTS)? ~ SymbolicName? ~ "(" ~ SchemaName ~ ")"
    // The SymbolicName before "(" is the name, SchemaName inside "(" is the target
    let (actual_name, actual_target) = if let Some(t) = target {
        (name, t)
    } else if let Some(n) = name {
        // No separate target — the SymbolicName is the target
        (None, n)
    } else {
        return Err(CypherError {
            kind: ErrorKind::Internal {
                message: "missing index target".into(),
            },
            span: sp,
            source_label: None,
            notes: Vec::new(),
            source: None,
        });
    };

    Ok(CreateIndex {
        kind,
        if_not_exists,
        name: actual_name,
        target: actual_target,
        options,
        span: sp,
    })
}

fn build_index_kind(pair: Pair<'_, Rule>) -> Result<IndexKind> {
    assert_eq!(pair.as_rule(), Rule::IndexKind);
    let sp = span(&pair);
    let inner = pair.clone().into_inner().next().unwrap();
    match inner.as_rule() {
        Rule::RANGE => Ok(IndexKind::Range),
        Rule::TEXT => Ok(IndexKind::Text),
        Rule::POINT => Ok(IndexKind::Point),
        Rule::LOOKUP => Ok(IndexKind::Lookup),
        Rule::FULLTEXT => Ok(IndexKind::Fulltext),
        _ => Err(unsupported(inner.as_rule(), sp)),
    }
}

fn build_options(pair: Pair<'_, Rule>) -> Result<MapLiteral> {
    assert_eq!(pair.as_rule(), Rule::Options);
    let sp = span(&pair);
    let inner = pair
        .clone()
        .into_inner()
        .find(|p| p.as_rule() != Rule::SP && p.as_rule() != Rule::OPTIONS);
    match inner {
        Some(map_pair) if map_pair.as_rule() == Rule::MapLiteral => build_map_literal(map_pair),
        _ => Err(CypherError {
            kind: ErrorKind::Internal {
                message: "missing map literal in OPTIONS".into(),
            },
            span: sp,
            source_label: None,
            notes: Vec::new(),
            source: None,
        }),
    }
}

fn build_drop_index(pair: Pair<'_, Rule>) -> Result<DropIndex> {
    assert_eq!(pair.as_rule(), Rule::DropIndex);
    let sp = span(&pair);
    let mut if_exists = false;
    let mut name = None;
    let mut saw_if = false;

    for child in pair.into_inner() {
        match child.as_rule() {
            Rule::IF => saw_if = true,
            Rule::EXISTS => {
                if saw_if {
                    if_exists = true;
                    saw_if = false;
                }
            }
            Rule::SymbolicName => name = Some(build_symbolic_name(child)?),
            Rule::DROP_ | Rule::INDEX | Rule::SP => {}
            _ => return Err(unsupported(child.as_rule(), sp)),
        }
    }

    Ok(DropIndex {
        if_exists,
        name: name.ok_or_else(|| CypherError {
            kind: ErrorKind::Internal {
                message: "missing index name".into(),
            },
            span: sp,
            source_label: None,
            notes: Vec::new(),
            source: None,
        })?,
        span: sp,
    })
}

fn build_create_constraint(pair: Pair<'_, Rule>) -> Result<CreateConstraint> {
    assert_eq!(pair.as_rule(), Rule::CreateConstraint);
    let sp = span(&pair);
    let mut name = None;
    let mut variable = None;
    let mut kind = None;
    let mut name_slot_open = true;

    for child in pair.into_inner() {
        match child.as_rule() {
            Rule::SymbolicName => {
                if name_slot_open && variable.is_none() {
                    name = Some(build_symbolic_name(child.clone())?);
                } else {
                    name_slot_open = false;
                }
            }
            Rule::Variable => variable = Some(build_variable(child)?),
            Rule::ConstraintKind => kind = Some(build_constraint_kind(child)?),
            Rule::CREATE | Rule::CONSTRAINT | Rule::FOR | Rule::SP => {
                if child.as_rule() == Rule::FOR {
                    name_slot_open = false;
                }
            }
            _ => return Err(unsupported(child.as_rule(), sp)),
        }
    }

    Ok(CreateConstraint {
        name,
        variable: variable.ok_or_else(|| CypherError {
            kind: ErrorKind::Internal {
                message: "missing constraint variable".into(),
            },
            span: sp,
            source_label: None,
            notes: Vec::new(),
            source: None,
        })?,
        kind: kind.ok_or_else(|| CypherError {
            kind: ErrorKind::Internal {
                message: "missing constraint kind".into(),
            },
            span: sp,
            source_label: None,
            notes: Vec::new(),
            source: None,
        })?,
        span: sp,
    })
}

fn build_constraint_kind(pair: Pair<'_, Rule>) -> Result<ConstraintKind> {
    assert_eq!(pair.as_rule(), Rule::ConstraintKind);
    let sp = span(&pair);
    let mut has_unique = false;
    let mut has_node = false;
    let mut has_key = false;
    let mut has_not_null = false;
    let mut has_property_type = false;
    let mut properties = Vec::new();
    let mut types = Vec::new();
    let in_parens = false;
    let mut saw_is = false;
    let mut saw_not = false;
    let mut saw_property = false;
    let mut saw_type = false;

    for child in pair.into_inner() {
        match child.as_rule() {
            Rule::IS => {
                saw_is = true;
            }
            Rule::NOT if saw_is => {
                saw_not = true;
            }
            Rule::NULL if saw_not => {
                has_not_null = true;
            }
            Rule::UNIQUE => {
                has_unique = true;
            }
            Rule::NODE => {
                has_node = true;
            }
            Rule::KEY => {
                has_key = true;
            }
            Rule::PROPERTY => {
                saw_property = true;
                has_property_type = true;
            }
            Rule::TYPE => {
                saw_type = true;
            }
            Rule::SymbolicName if in_parens => {
                types.push(build_symbolic_name(child)?);
            }
            Rule::PropertyKeyName => {
                properties.push(build_property_key_name(child)?);
            }
            _ => {}
        }
    }

    if has_property_type {
        Ok(ConstraintKind::PropertyType { types })
    } else if has_node && has_key {
        Ok(ConstraintKind::NodeKey { properties })
    } else if has_unique {
        Ok(ConstraintKind::Unique)
    } else if has_not_null {
        Ok(ConstraintKind::NotNull)
    } else {
        Err(CypherError {
            kind: ErrorKind::Internal {
                message: "unrecognized constraint kind".into(),
            },
            span: sp,
            source_label: None,
            notes: Vec::new(),
            source: None,
        })
    }
}

fn build_drop_constraint(pair: Pair<'_, Rule>) -> Result<DropConstraint> {
    assert_eq!(pair.as_rule(), Rule::DropConstraint);
    let sp = span(&pair);
    let mut if_exists = false;
    let mut name = None;
    let mut saw_if = false;

    for child in pair.into_inner() {
        match child.as_rule() {
            Rule::IF => saw_if = true,
            Rule::EXISTS => {
                if saw_if {
                    if_exists = true;
                    saw_if = false;
                }
            }
            Rule::SymbolicName => name = Some(build_symbolic_name(child)?),
            Rule::DROP_ | Rule::CONSTRAINT | Rule::SP => {}
            _ => return Err(unsupported(child.as_rule(), sp)),
        }
    }

    Ok(DropConstraint {
        if_exists,
        name: name.ok_or_else(|| CypherError {
            kind: ErrorKind::Internal {
                message: "missing constraint name".into(),
            },
            span: sp,
            source_label: None,
            notes: Vec::new(),
            source: None,
        })?,
        span: sp,
    })
}

fn build_show(pair: Pair<'_, Rule>) -> Result<Show> {
    assert_eq!(pair.as_rule(), Rule::Show);
    let sp = span(&pair);
    let mut kind = None;
    let mut yield_items = None;
    let mut where_clause = None;
    let mut return_clause = None;

    for child in pair.into_inner() {
        match child.as_rule() {
            Rule::ShowKind => kind = Some(build_show_kind(child)?),
            Rule::YieldItems => yield_items = Some(build_show_yield_items(child)?),
            Rule::STAR => {
                yield_items = Some(ShowYieldSpec::Star { span: span(&child) });
            }
            Rule::Where => where_clause = Some(build_where(child)?),
            Rule::Return => return_clause = Some(build_return_body(child)?),
            Rule::SHOW | Rule::SP => {}
            Rule::YIELD => {}
            _ => return Err(unsupported(child.as_rule(), sp)),
        }
    }

    Ok(Show {
        kind: kind.ok_or_else(|| CypherError {
            kind: ErrorKind::Internal {
                message: "missing SHOW kind".into(),
            },
            span: sp,
            source_label: None,
            notes: Vec::new(),
            source: None,
        })?,
        yield_items,
        where_clause,
        return_clause,
        span: sp,
    })
}

fn build_show_kind(pair: Pair<'_, Rule>) -> Result<ShowKind> {
    assert_eq!(pair.as_rule(), Rule::ShowKind);
    let sp = span(&pair);
    let mut inner = pair.clone().into_inner();
    let first = inner.next();
    match first {
        Some(child) => match child.as_rule() {
            Rule::INDEXES => Ok(ShowKind::Indexes),
            Rule::CONSTRAINTS => Ok(ShowKind::Constraints),
            Rule::FUNCTIONS => Ok(ShowKind::Functions),
            Rule::PROCEDURES => Ok(ShowKind::Procedures),
            Rule::DATABASES => Ok(ShowKind::Databases),
            Rule::DATABASE => {
                let name = inner
                    .find(|p| p.as_rule() == Rule::SymbolicName)
                    .map(|p| build_symbolic_name(p))
                    .transpose()?;
                Ok(ShowKind::Database(name.ok_or_else(|| CypherError {
                    kind: ErrorKind::Internal {
                        message: "missing database name".into(),
                    },
                    span: sp,
                    source_label: None,
                    notes: Vec::new(),
                    source: None,
                })?))
            }
            _ => Err(unsupported(child.as_rule(), sp)),
        },
        None => Err(unsupported(pair.as_rule(), sp)),
    }
}

fn build_show_yield_items(pair: Pair<'_, Rule>) -> Result<ShowYieldSpec> {
    assert_eq!(pair.as_rule(), Rule::YieldItems);
    let _sp = span(&pair);
    let mut items = Vec::new();

    for child in pair.into_inner() {
        match child.as_rule() {
            Rule::YieldItem => items.push(build_show_yield_item(child)?),
            Rule::Where | Rule::SP | Rule::YIELD => {}
            _ => {}
        }
    }

    Ok(ShowYieldSpec::Items(items))
}

fn build_show_yield_item(pair: Pair<'_, Rule>) -> Result<ShowYieldItem> {
    assert_eq!(pair.as_rule(), Rule::YieldItem);
    let sp = span(&pair);
    let mut procedure_field = None;
    let mut alias = None;

    for child in pair.clone().into_inner() {
        match child.as_rule() {
            Rule::ProcedureResultField => {
                procedure_field = Some(build_symbolic_name(child.into_inner().next().unwrap())?);
            }
            Rule::Variable => alias = Some(build_variable(child)?),
            Rule::AS | Rule::SP => {}
            _ => {}
        }
    }

    Ok(ShowYieldItem {
        procedure_field: procedure_field.unwrap_or_else(|| SymbolicName {
            name: String::new(),
            span: sp,
        }),
        alias,
    })
}

fn build_return_body(pair: Pair<'_, Rule>) -> Result<ReturnBody> {
    assert_eq!(pair.as_rule(), Rule::Return);
    let _sp = span(&pair);
    let projection = pair
        .clone()
        .into_inner()
        .find(|p| p.as_rule() == Rule::ProjectionBody)
        .map(|p| build_projection_body(p))
        .transpose()?
        .unwrap();
    Ok(ReturnBody {
        distinct: projection.0,
        items: projection.1,
        order: projection.2,
        skip: projection.3,
        limit: projection.4,
    })
}

fn build_use(pair: Pair<'_, Rule>) -> Result<Use> {
    assert_eq!(pair.as_rule(), Rule::Use);
    let sp = span(&pair);
    let name = pair
        .into_inner()
        .find(|p| p.as_rule() == Rule::SymbolicName)
        .map(|p| build_symbolic_name(p))
        .transpose()?
        .ok_or_else(|| CypherError {
            kind: ErrorKind::Internal {
                message: "missing graph name in USE".into(),
            },
            span: sp,
            source_label: None,
            notes: Vec::new(),
            source: None,
        })?;
    Ok(Use {
        graph: name,
        span: sp,
    })
}
