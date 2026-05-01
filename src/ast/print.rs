/// AST-to-text emission for openCypher queries.
///
/// The [`ToCypher`] trait converts any AST node back into valid openCypher text.
/// This enables round-trips: `parse(src) → ast → to_cypher() → parse`.
///
/// # Formatting guarantees
///
/// - **Single-line output** — no line breaks or indentation are inserted.
/// - **Keyword casing** — all keywords are emitted in uppercase, matching openCypher convention.
/// - **Strings** — emitted in single quotes. Escape sequences are preserved as stored
///   by the parser (the parser strips surrounding quotes but does not unescape).
/// - **Numbers** — integers use `Display`; floats use [`ryu`] for shortest round-trip-safe
///   representation. NaN and Inf are emitted as `NaN` / `Infinity` / `-Infinity`
///   (note: openCypher has no literals for these values).
/// - **Parentheses** — emitted only for `Expression::Parenthesized` nodes.
///   Precedence-based parentheses are NOT added; the output relies on the parser
///   having already inserted `Parenthesized` nodes where needed.
/// - **Identifiers** — reserved words or identifiers containing non-alphanumeric/underscore
///   characters are backtick-quoted.
///
/// # Non-guarantees
///
/// - Whitespace between tokens may differ from the original input.
/// - Comments are not preserved (they are not represented in the AST).
/// - `Parenthesized` nodes that were added by the parser for precedence disambiguation
///   will be re-emitted, possibly producing more parens than the original input.
use core::fmt;

use crate::ast::clause::*;
use crate::ast::expr::*;
use crate::ast::names::*;
use crate::ast::pattern::*;
use crate::ast::procedure::*;
use crate::ast::query::*;
use crate::ast::schema::*;

pub trait ToCypher {
    fn to_cypher(&self) -> String {
        let mut s = String::new();
        let _ = self.write_cypher(&mut s);
        s
    }

    fn write_cypher(&self, w: &mut dyn fmt::Write) -> fmt::Result;

    /// Returns a wrapper that implements [`fmt::Display`], allowing the AST node
    /// to be used with `{}` format specifiers.
    ///
    /// # Example
    ///
    /// ```
    /// use open_cypher::ast::ToCypher;
    /// use open_cypher::parse;
    ///
    /// let query = parse("MATCH (n) RETURN n;").unwrap();
    /// println!("{}", query.display());
    /// ```
    fn display(&self) -> DisplayCypher<'_, Self>
    where
        Self: Sized,
    {
        DisplayCypher(self)
    }
}

/// A wrapper around a [`ToCypher`] implementor that provides a [`fmt::Display`] impl.
///
/// Created via [`ToCypher::display`].
pub struct DisplayCypher<'a, T: ?Sized>(&'a T);

impl<T: ToCypher + ?Sized> fmt::Display for DisplayCypher<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.write_cypher(f)
    }
}

impl<T: ToCypher> ToCypher for &T {
    fn write_cypher(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        (*self).write_cypher(w)
    }
}

impl<T: ToCypher> ToCypher for Box<T> {
    fn write_cypher(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        self.as_ref().write_cypher(w)
    }
}

impl<T: ToCypher> ToCypher for Option<T> {
    fn write_cypher(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        if let Some(v) = self {
            v.write_cypher(w)
        } else {
            Ok(())
        }
    }
}

impl ToCypher for Query {
    fn write_cypher(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        for (i, stmt) in self.statements.iter().enumerate() {
            if i > 0 {
                write!(w, "; ")?;
            }
            stmt.write_cypher(w)?;
        }
        write!(w, ";")
    }
}

impl ToCypher for QueryBody {
    fn write_cypher(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        match self {
            QueryBody::SingleQuery(sq) => sq.write_cypher(w),
            QueryBody::Regular(rq) => rq.write_cypher(w),
            QueryBody::Standalone(sc) => sc.write_cypher(w),
            QueryBody::SchemaCommand(sc) => sc.write_cypher(w),
            QueryBody::Show(s) => s.write_cypher(w),
            QueryBody::Use(u) => u.write_cypher(w),
        }
    }
}

impl ToCypher for SingleQuery {
    fn write_cypher(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        self.kind.write_cypher(w)
    }
}

impl ToCypher for SingleQueryKind {
    fn write_cypher(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        match self {
            SingleQueryKind::SinglePart(sp) => sp.write_cypher(w),
            SingleQueryKind::MultiPart(mp) => mp.write_cypher(w),
        }
    }
}

impl ToCypher for SinglePartQuery {
    fn write_cypher(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        for rc in &self.reading_clauses {
            rc.write_cypher(w)?;
            write!(w, " ")?;
        }
        self.body.write_cypher(w)
    }
}

impl ToCypher for SinglePartBody {
    fn write_cypher(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        match self {
            SinglePartBody::Return(r) => r.write_cypher(w),
            SinglePartBody::Updating {
                updating,
                return_clause,
            } => {
                for uc in updating {
                    uc.write_cypher(w)?;
                    write!(w, " ")?;
                }
                if let Some(rc) = return_clause {
                    write!(w, " ")?;
                    rc.write_cypher(w)?;
                }
                Ok(())
            }
        }
    }
}

impl ToCypher for MultiPartQuery {
    fn write_cypher(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        for part in &self.parts {
            part.write_cypher(w)?;
            write!(w, " ")?;
        }
        self.final_part.write_cypher(w)
    }
}

impl ToCypher for MultiPartQueryPart {
    fn write_cypher(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        for rc in &self.reading_clauses {
            rc.write_cypher(w)?;
            write!(w, " ")?;
        }
        for uc in &self.updating_clauses {
            uc.write_cypher(w)?;
            write!(w, " ")?;
        }
        self.with.write_cypher(w)
    }
}

impl ToCypher for ReadingClause {
    fn write_cypher(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        match self {
            ReadingClause::Match(m) => m.write_cypher(w),
            ReadingClause::Unwind(u) => u.write_cypher(w),
            ReadingClause::InQueryCall(i) => i.write_cypher(w),
            ReadingClause::CallSubquery(c) => c.write_cypher(w),
        }
    }
}

impl ToCypher for UpdatingClause {
    fn write_cypher(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        match self {
            UpdatingClause::Create(c) => c.write_cypher(w),
            UpdatingClause::Merge(m) => m.write_cypher(w),
            UpdatingClause::Delete(d) => d.write_cypher(w),
            UpdatingClause::Set(s) => s.write_cypher(w),
            UpdatingClause::Remove(r) => r.write_cypher(w),
            UpdatingClause::Foreach(f) => f.write_cypher(w),
        }
    }
}

impl ToCypher for RegularQuery {
    fn write_cypher(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        self.single_query.write_cypher(w)?;
        for u in &self.unions {
            write!(w, " ")?;
            u.write_cypher(w)?;
        }
        Ok(())
    }
}

impl ToCypher for Union {
    fn write_cypher(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        if self.all {
            write!(w, "UNION ALL ")?;
        } else {
            write!(w, "UNION ")?;
        }
        self.single_query.write_cypher(w)
    }
}

impl ToCypher for Match {
    fn write_cypher(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        if self.optional {
            write!(w, "OPTIONAL MATCH ")?;
        } else {
            write!(w, "MATCH ")?;
        }
        self.pattern.write_cypher(w)?;
        if let Some(wc) = &self.where_clause {
            write!(w, " WHERE ")?;
            wc.write_cypher(w)?;
        }
        Ok(())
    }
}

impl ToCypher for Create {
    fn write_cypher(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        write!(w, "CREATE ")?;
        self.pattern.write_cypher(w)
    }
}

impl ToCypher for Merge {
    fn write_cypher(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        write!(w, "MERGE ")?;
        self.pattern.write_cypher(w)?;
        for action in &self.actions {
            write!(w, " ")?;
            action.write_cypher(w)?;
        }
        Ok(())
    }
}

impl ToCypher for MergeAction {
    fn write_cypher(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        if self.on_match {
            write!(w, "ON MATCH SET")?;
        } else {
            write!(w, "ON CREATE SET")?;
        }
        for (i, item) in self.set_items.iter().enumerate() {
            if i > 0 {
                write!(w, ",")?;
            }
            write!(w, " ")?;
            item.write_cypher(w)?;
        }
        Ok(())
    }
}

impl ToCypher for Delete {
    fn write_cypher(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        if self.detach {
            write!(w, "DETACH DELETE ")?;
        } else {
            write!(w, "DELETE ")?;
        }
        for (i, target) in self.targets.iter().enumerate() {
            if i > 0 {
                write!(w, ", ")?;
            }
            target.write_cypher(w)?;
        }
        Ok(())
    }
}

impl ToCypher for Set {
    fn write_cypher(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        write!(w, "SET ")?;
        for (i, item) in self.items.iter().enumerate() {
            if i > 0 {
                write!(w, ", ")?;
            }
            item.write_cypher(w)?;
        }
        Ok(())
    }
}

impl ToCypher for SetItem {
    fn write_cypher(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        match self {
            SetItem::Property {
                property,
                value,
                operator,
            } => {
                property.write_cypher(w)?;
                write!(w, " ")?;
                operator.write_cypher(w)?;
                write!(w, " ")?;
                value.write_cypher(w)
            }
            SetItem::Variable {
                variable,
                value,
                operator,
            } => {
                variable.write_cypher(w)?;
                write!(w, " ")?;
                operator.write_cypher(w)?;
                write!(w, " ")?;
                value.write_cypher(w)
            }
            SetItem::Labels { variable, labels } => {
                variable.write_cypher(w)?;
                for label in labels {
                    write!(w, ":")?;
                    label.write_cypher(w)?;
                }
                Ok(())
            }
        }
    }
}

impl ToCypher for SetOperator {
    fn write_cypher(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        match self {
            SetOperator::Assign => write!(w, "="),
            SetOperator::Add => write!(w, "+="),
        }
    }
}

impl ToCypher for Remove {
    fn write_cypher(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        write!(w, "REMOVE ")?;
        for (i, item) in self.items.iter().enumerate() {
            if i > 0 {
                write!(w, ", ")?;
            }
            item.write_cypher(w)?;
        }
        Ok(())
    }
}

impl ToCypher for RemoveItem {
    fn write_cypher(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        match self {
            RemoveItem::Labels { variable, labels } => {
                variable.write_cypher(w)?;
                for label in labels {
                    write!(w, ":")?;
                    label.write_cypher(w)?;
                }
                Ok(())
            }
            RemoveItem::Property(expr) => expr.write_cypher(w),
        }
    }
}

impl ToCypher for With {
    fn write_cypher(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        write!(w, "WITH ")?;
        if self.distinct {
            write!(w, "DISTINCT ")?;
        }
        for (i, item) in self.items.iter().enumerate() {
            if i > 0 {
                write!(w, ", ")?;
            }
            item.write_cypher(w)?;
        }
        if let Some(order) = &self.order {
            write!(w, " ")?;
            order.write_cypher(w)?;
        }
        if let Some(skip) = &self.skip {
            write!(w, " SKIP ")?;
            skip.write_cypher(w)?;
        }
        if let Some(limit) = &self.limit {
            write!(w, " LIMIT ")?;
            limit.write_cypher(w)?;
        }
        if let Some(wc) = &self.where_clause {
            write!(w, " WHERE ")?;
            wc.write_cypher(w)?;
        }
        Ok(())
    }
}

impl ToCypher for Return {
    fn write_cypher(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        write!(w, "RETURN ")?;
        if self.distinct {
            write!(w, "DISTINCT ")?;
        }
        for (i, item) in self.items.iter().enumerate() {
            if i > 0 {
                write!(w, ", ")?;
            }
            item.write_cypher(w)?;
        }
        if let Some(order) = &self.order {
            write!(w, " ")?;
            order.write_cypher(w)?;
        }
        if let Some(skip) = &self.skip {
            write!(w, " SKIP ")?;
            skip.write_cypher(w)?;
        }
        if let Some(limit) = &self.limit {
            write!(w, " LIMIT ")?;
            limit.write_cypher(w)?;
        }
        Ok(())
    }
}

impl ToCypher for ProjectionItem {
    fn write_cypher(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        self.expression.write_cypher(w)?;
        if let Some(alias) = &self.alias {
            write!(w, " AS ")?;
            alias.write_cypher(w)?;
        }
        Ok(())
    }
}

impl ToCypher for Order {
    fn write_cypher(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        write!(w, "ORDER BY ")?;
        for (i, item) in self.items.iter().enumerate() {
            if i > 0 {
                write!(w, ", ")?;
            }
            item.write_cypher(w)?;
        }
        Ok(())
    }
}

impl ToCypher for SortItem {
    fn write_cypher(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        self.expression.write_cypher(w)?;
        if let Some(dir) = &self.direction {
            write!(w, " ")?;
            dir.write_cypher(w)?;
        }
        Ok(())
    }
}

impl ToCypher for SortDirection {
    fn write_cypher(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        match self {
            SortDirection::Ascending => write!(w, "ASC"),
            SortDirection::Descending => write!(w, "DESC"),
        }
    }
}

impl ToCypher for Unwind {
    fn write_cypher(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        write!(w, "UNWIND ")?;
        self.expression.write_cypher(w)?;
        write!(w, " AS ")?;
        self.variable.write_cypher(w)
    }
}

impl ToCypher for Pattern {
    fn write_cypher(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        for (i, part) in self.parts.iter().enumerate() {
            if i > 0 {
                write!(w, ", ")?;
            }
            part.write_cypher(w)?;
        }
        Ok(())
    }
}

impl ToCypher for PatternPart {
    fn write_cypher(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        if let Some(var) = &self.variable {
            var.write_cypher(w)?;
            write!(w, " = ")?;
        }
        self.anonymous.write_cypher(w)
    }
}

impl ToCypher for AnonymousPatternPart {
    fn write_cypher(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        self.element.write_cypher(w)
    }
}

impl ToCypher for PatternElement {
    fn write_cypher(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        match self {
            PatternElement::Path { start, chains } => {
                start.write_cypher(w)?;
                for chain in chains {
                    chain.write_cypher(w)?;
                }
                Ok(())
            }
            PatternElement::Parenthesized(inner) => {
                write!(w, "(")?;
                inner.write_cypher(w)?;
                write!(w, ")")
            }
        }
    }
}

impl ToCypher for NodePattern {
    fn write_cypher(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        write!(w, "(")?;
        if let Some(var) = &self.variable {
            var.write_cypher(w)?;
        }
        for label in &self.labels {
            write!(w, ":")?;
            label.write_cypher(w)?;
        }
        if let Some(props) = &self.properties {
            write!(w, " ")?;
            props.write_cypher(w)?;
        }
        write!(w, ")")
    }
}

impl ToCypher for PatternElementChain {
    fn write_cypher(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        self.relationship.write_cypher(w)?;
        self.node.write_cypher(w)
    }
}

impl ToCypher for RelationshipPattern {
    fn write_cypher(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        match self.direction {
            RelationshipDirection::Left => {
                write!(w, "<-")?;
                if let Some(detail) = &self.detail {
                    write!(w, "[")?;
                    detail.write_cypher(w)?;
                    write!(w, "]")?;
                } else {
                    write!(w, "[]")?;
                }
                write!(w, "-")
            }
            RelationshipDirection::Right => {
                write!(w, "-")?;
                if let Some(detail) = &self.detail {
                    write!(w, "[")?;
                    detail.write_cypher(w)?;
                    write!(w, "]")?;
                } else {
                    write!(w, "[]")?;
                }
                write!(w, "->")
            }
            RelationshipDirection::Both => {
                write!(w, "-")?;
                if let Some(detail) = &self.detail {
                    write!(w, "[")?;
                    detail.write_cypher(w)?;
                    write!(w, "]")?;
                } else {
                    write!(w, "[]")?;
                }
                write!(w, "-")
            }
            RelationshipDirection::Undirected => {
                if let Some(detail) = &self.detail {
                    write!(w, "-[")?;
                    detail.write_cypher(w)?;
                    write!(w, "]-")
                } else {
                    write!(w, "-[]-")
                }
            }
        }
    }
}

impl ToCypher for RelationshipDetail {
    fn write_cypher(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        if let Some(var) = &self.variable {
            var.write_cypher(w)?;
        }
        if !self.types.is_empty() {
            write!(w, ":")?;
            for (i, t) in self.types.iter().enumerate() {
                if i > 0 {
                    write!(w, "|")?;
                }
                t.write_cypher(w)?;
            }
        }
        if let Some(range) = &self.range {
            range.write_cypher(w)?;
        }
        if let Some(props) = &self.properties {
            write!(w, " ")?;
            props.write_cypher(w)?;
        }
        Ok(())
    }
}

impl ToCypher for RangeLiteral {
    fn write_cypher(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        write!(w, "*")?;
        match (self.start, self.end) {
            (Some(s), Some(e)) => write!(w, "{}..{}", s, e)?,
            (Some(s), None) => write!(w, "{}..", s)?,
            (None, Some(e)) => write!(w, "..{}", e)?,
            (None, None) => write!(w, "")?,
        }
        Ok(())
    }
}

impl ToCypher for RelationshipsPattern {
    fn write_cypher(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        self.start.write_cypher(w)?;
        for chain in &self.chains {
            chain.write_cypher(w)?;
        }
        Ok(())
    }
}

impl ToCypher for Properties {
    fn write_cypher(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        match self {
            Properties::Map(m) => m.write_cypher(w),
            Properties::Parameter(p) => p.write_cypher(w),
        }
    }
}

impl ToCypher for Expression {
    fn write_cypher(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        match self {
            Expression::Literal(l) => l.write_cypher(w),
            Expression::Variable(v) => v.write_cypher(w),
            Expression::Parameter(p) => p.write_cypher(w),
            Expression::PropertyLookup { base, property, .. } => {
                base.write_cypher(w)?;
                write!(w, ".")?;
                property.write_cypher(w)
            }
            Expression::NodeLabels { base, labels, .. } => {
                base.write_cypher(w)?;
                for label in labels {
                    write!(w, ":")?;
                    label.write_cypher(w)?;
                }
                Ok(())
            }
            Expression::BinaryOp { op, lhs, rhs, .. } => {
                lhs.write_cypher(w)?;
                write!(w, " ")?;
                op.write_cypher(w)?;
                write!(w, " ")?;
                rhs.write_cypher(w)
            }
            Expression::UnaryOp { op, operand, .. } => {
                op.write_cypher(w)?;
                operand.write_cypher(w)
            }
            Expression::Comparison { lhs, operators, .. } => {
                lhs.write_cypher(w)?;
                for (cop, rhs) in operators {
                    write!(w, " ")?;
                    cop.write_cypher(w)?;
                    write!(w, " ")?;
                    rhs.write_cypher(w)?;
                }
                Ok(())
            }
            Expression::ListIndex { list, index, .. } => {
                list.write_cypher(w)?;
                write!(w, "[")?;
                index.write_cypher(w)?;
                write!(w, "]")
            }
            Expression::ListSlice {
                list, start, end, ..
            } => {
                list.write_cypher(w)?;
                write!(w, "[")?;
                if let Some(s) = start {
                    s.write_cypher(w)?;
                }
                write!(w, "..")?;
                if let Some(e) = end {
                    e.write_cypher(w)?;
                }
                write!(w, "]")
            }
            Expression::In { lhs, rhs, .. } => {
                lhs.write_cypher(w)?;
                write!(w, " IN ")?;
                rhs.write_cypher(w)
            }
            Expression::IsNull {
                operand, negated, ..
            } => {
                operand.write_cypher(w)?;
                if *negated {
                    write!(w, " IS NOT NULL")
                } else {
                    write!(w, " IS NULL")
                }
            }
            Expression::FunctionCall(fi) => fi.write_cypher(w),
            Expression::CountStar { .. } => write!(w, "COUNT(*)"),
            Expression::Case(c) => c.write_cypher(w),
            Expression::ListComprehension(lc) => lc.write_cypher(w),
            Expression::PatternComprehension(pc) => pc.write_cypher(w),
            Expression::All(fe) => {
                write!(w, "ALL(")?;
                fe.write_cypher(w)?;
                write!(w, ")")
            }
            Expression::Any(fe) => {
                write!(w, "ANY(")?;
                fe.write_cypher(w)?;
                write!(w, ")")
            }
            Expression::None(fe) => {
                write!(w, "NONE(")?;
                fe.write_cypher(w)?;
                write!(w, ")")
            }
            Expression::Single(fe) => {
                write!(w, "SINGLE(")?;
                fe.write_cypher(w)?;
                write!(w, ")")
            }
            Expression::Parenthesized(inner) => {
                write!(w, "(")?;
                inner.write_cypher(w)?;
                write!(w, ")")
            }
            Expression::Pattern(p) => p.write_cypher(w),
            Expression::Exists(ex) => ex.write_cypher(w),
            Expression::MapProjection(mp) => mp.write_cypher(w),
        }
    }
}

impl ToCypher for Literal {
    fn write_cypher(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        match self {
            Literal::Number(n) => n.write_cypher(w),
            Literal::String(s) => s.write_cypher(w),
            Literal::Boolean(b) => write!(w, "{}", if *b { "true" } else { "false" }),
            Literal::Null => write!(w, "NULL"),
            Literal::List(l) => l.write_cypher(w),
            Literal::Map(m) => m.write_cypher(w),
        }
    }
}

impl ToCypher for NumberLiteral {
    fn write_cypher(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        match self {
            NumberLiteral::Integer(i) => write!(w, "{}", i),
            NumberLiteral::Float(f) => {
                if f.is_nan() {
                    write!(w, "NaN")
                } else if f.is_infinite() {
                    if *f > 0.0 {
                        write!(w, "Infinity")
                    } else {
                        write!(w, "-Infinity")
                    }
                } else {
                    write!(w, "{}", ryu::Buffer::new().format(*f))
                }
            }
        }
    }
}

impl ToCypher for StringLiteral {
    fn write_cypher(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        if let Some(raw) = &self.raw {
            write!(w, "{}", raw)
        } else {
            write!(
                w,
                "'{}'",
                self.value.replace('\\', "\\\\").replace('\'', "\\'")
            )
        }
    }
}

impl ToCypher for ListLiteral {
    fn write_cypher(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        write!(w, "[")?;
        for (i, elem) in self.elements.iter().enumerate() {
            if i > 0 {
                write!(w, ", ")?;
            }
            elem.write_cypher(w)?;
        }
        write!(w, "]")
    }
}

impl ToCypher for MapLiteral {
    fn write_cypher(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        write!(w, "{{")?;
        for (i, (key, value)) in self.entries.iter().enumerate() {
            if i > 0 {
                write!(w, ", ")?;
            }
            key.write_cypher(w)?;
            write!(w, ": ")?;
            value.write_cypher(w)?;
        }
        write!(w, "}}")
    }
}

impl ToCypher for Parameter {
    fn write_cypher(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        write!(w, "$")?;
        self.name.write_cypher(w)
    }
}

impl ToCypher for BinaryOperator {
    fn write_cypher(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        match self {
            BinaryOperator::Add => write!(w, "+"),
            BinaryOperator::Subtract => write!(w, "-"),
            BinaryOperator::Multiply => write!(w, "*"),
            BinaryOperator::Divide => write!(w, "/"),
            BinaryOperator::Modulo => write!(w, "%"),
            BinaryOperator::Power => write!(w, "^"),
            BinaryOperator::And => write!(w, "AND"),
            BinaryOperator::Or => write!(w, "OR"),
            BinaryOperator::Xor => write!(w, "XOR"),
        }
    }
}

impl ToCypher for UnaryOperator {
    fn write_cypher(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        match self {
            UnaryOperator::Negate => write!(w, "-"),
            UnaryOperator::Plus => write!(w, "+"),
            UnaryOperator::Not => write!(w, "NOT "),
        }
    }
}

impl ToCypher for ComparisonOperator {
    fn write_cypher(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        match self {
            ComparisonOperator::Eq => write!(w, "="),
            ComparisonOperator::Ne => write!(w, "<>"),
            ComparisonOperator::Lt => write!(w, "<"),
            ComparisonOperator::Gt => write!(w, ">"),
            ComparisonOperator::Le => write!(w, "<="),
            ComparisonOperator::Ge => write!(w, ">="),
            ComparisonOperator::StartsWith => write!(w, "STARTS WITH"),
            ComparisonOperator::EndsWith => write!(w, "ENDS WITH"),
            ComparisonOperator::Contains => write!(w, "CONTAINS"),
        }
    }
}

impl ToCypher for FunctionInvocation {
    fn write_cypher(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        for (i, part) in self.name.iter().enumerate() {
            if i > 0 {
                write!(w, ".")?;
            }
            part.write_cypher(w)?;
        }
        write!(w, "(")?;
        if self.distinct {
            write!(w, "DISTINCT ")?;
        }
        for (i, arg) in self.arguments.iter().enumerate() {
            if i > 0 {
                write!(w, ", ")?;
            }
            arg.write_cypher(w)?;
        }
        write!(w, ")")
    }
}

impl ToCypher for CaseExpression {
    fn write_cypher(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        write!(w, "CASE")?;
        if let Some(scrutinee) = &self.scrutinee {
            write!(w, " ")?;
            scrutinee.write_cypher(w)?;
        }
        for alt in &self.alternatives {
            write!(w, " WHEN ")?;
            alt.when.write_cypher(w)?;
            write!(w, " THEN ")?;
            alt.then.write_cypher(w)?;
        }
        if let Some(default) = &self.default {
            write!(w, " ELSE ")?;
            default.write_cypher(w)?;
        }
        write!(w, " END")
    }
}

impl ToCypher for ListComprehension {
    fn write_cypher(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        write!(w, "[")?;
        self.variable.write_cypher(w)?;
        write!(w, " IN ")?;
        if let Some(filter) = &self.filter {
            filter.write_cypher(w)?;
        }
        if let Some(map) = &self.map {
            write!(w, " | ")?;
            map.write_cypher(w)?;
        }
        write!(w, "]")
    }
}

impl ToCypher for PatternComprehension {
    fn write_cypher(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        write!(w, "[(")?;
        if let Some(var) = &self.variable {
            var.write_cypher(w)?;
            write!(w, " = ")?;
        }
        self.pattern.write_cypher(w)?;
        write!(w, ")")?;
        if let Some(wc) = &self.where_clause {
            write!(w, " WHERE ")?;
            wc.write_cypher(w)?;
        }
        write!(w, " | ")?;
        self.map.write_cypher(w)?;
        write!(w, "]")
    }
}

impl ToCypher for FilterExpression {
    fn write_cypher(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        self.variable.write_cypher(w)?;
        write!(w, " IN ")?;
        self.collection.write_cypher(w)?;
        if let Some(pred) = &self.predicate {
            write!(w, " WHERE ")?;
            pred.write_cypher(w)?;
        }
        Ok(())
    }
}

impl ToCypher for ExistsExpression {
    fn write_cypher(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        write!(w, "EXISTS ")?;
        self.inner.write_cypher(w)
    }
}

impl ToCypher for ExistsInner {
    fn write_cypher(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        match self {
            ExistsInner::Pattern(pat, where_clause) => {
                write!(w, "{{ ")?;
                pat.write_cypher(w)?;
                if let Some(wc) = where_clause {
                    write!(w, " WHERE ")?;
                    wc.write_cypher(w)?;
                }
                write!(w, " }}")
            }
            ExistsInner::RegularQuery(rq) => {
                write!(w, "{{ ")?;
                rq.write_cypher(w)?;
                write!(w, " }}")
            }
        }
    }
}

impl ToCypher for StandaloneCall {
    fn write_cypher(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        self.call.write_cypher(w)?;
        if let Some(yield_spec) = &self.yield_items {
            write!(w, " ")?;
            yield_spec.write_cypher(w)?;
        }
        Ok(())
    }
}

impl ToCypher for InQueryCall {
    fn write_cypher(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        self.call.write_cypher(w)?;
        if let Some(yield_items) = &self.yield_items {
            write!(w, " ")?;
            yield_items.write_cypher(w)?;
        }
        Ok(())
    }
}

impl ToCypher for ProcedureInvocation {
    fn write_cypher(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        self.name.write_cypher(w)
    }
}

impl ToCypher for YieldSpec {
    fn write_cypher(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        match self {
            YieldSpec::Star { .. } => write!(w, "YIELD *"),
            YieldSpec::Items(yi) => {
                write!(w, "YIELD ")?;
                yi.write_cypher(w)
            }
        }
    }
}

impl ToCypher for YieldItems {
    fn write_cypher(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        for (i, item) in self.items.iter().enumerate() {
            if i > 0 {
                write!(w, ", ")?;
            }
            item.write_cypher(w)?;
        }
        if let Some(wc) = &self.where_clause {
            write!(w, " WHERE ")?;
            wc.write_cypher(w)?;
        }
        Ok(())
    }
}

impl ToCypher for YieldItem {
    fn write_cypher(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        self.procedure_field.write_cypher(w)?;
        if let Some(alias) = &self.alias {
            write!(w, " AS ")?;
            alias.write_cypher(w)?;
        }
        Ok(())
    }
}

impl ToCypher for Variable {
    fn write_cypher(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        self.name.write_cypher(w)
    }
}

impl ToCypher for SymbolicName {
    fn write_cypher(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        let is_safe = is_safe_bare_identifier(&self.name);
        if is_safe {
            write!(w, "{}", self.name)
        } else {
            write!(w, "`{}`", self.name)
        }
    }
}

impl ToCypher for LabelName {
    fn write_cypher(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        self.name.write_cypher(w)
    }
}

impl ToCypher for RelTypeName {
    fn write_cypher(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        self.name.write_cypher(w)
    }
}

impl ToCypher for PropertyKeyName {
    fn write_cypher(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        self.name.write_cypher(w)
    }
}

fn is_safe_bare_identifier(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    let is_keyword = matches!(
        name.to_uppercase().as_str(),
        "MATCH"
            | "WHERE"
            | "RETURN"
            | "CREATE"
            | "DELETE"
            | "SET"
            | "REMOVE"
            | "WITH"
            | "ORDER"
            | "BY"
            | "ASC"
            | "DESC"
            | "SKIP"
            | "LIMIT"
            | "UNION"
            | "ALL"
            | "DISTINCT"
            | "OPTIONAL"
            | "UNWIND"
            | "MERGE"
            | "ON"
            | "AS"
            | "IN"
            | "IS"
            | "NULL"
            | "NOT"
            | "AND"
            | "OR"
            | "XOR"
            | "TRUE"
            | "FALSE"
            | "CASE"
            | "WHEN"
            | "THEN"
            | "ELSE"
            | "END"
            | "EXISTS"
            | "COUNT"
            | "YIELD"
            | "DETACH"
            | "START"
            | "CALL"
    );
    if is_keyword {
        return false;
    }
    let first = name.chars().next().unwrap();
    if !first.is_ascii_alphabetic() && first != '_' {
        return false;
    }
    name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

// ── ToCypher implementations for new AST types (Parsing 1.0) ──

impl ToCypher for Foreach {
    fn write_cypher(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        write!(w, "FOREACH (")?;
        self.variable.write_cypher(w)?;
        write!(w, " IN ")?;
        self.list.write_cypher(w)?;
        write!(w, " | ")?;
        for (i, update) in self.updates.iter().enumerate() {
            if i > 0 {
                write!(w, " ")?;
            }
            update.write_cypher(w)?;
        }
        write!(w, ")")
    }
}

impl ToCypher for ForeachUpdate {
    fn write_cypher(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        match self {
            ForeachUpdate::Create(c) => c.write_cypher(w),
            ForeachUpdate::Merge(m) => m.write_cypher(w),
            ForeachUpdate::Delete(d) => d.write_cypher(w),
            ForeachUpdate::Set(s) => s.write_cypher(w),
            ForeachUpdate::Remove(r) => r.write_cypher(w),
            ForeachUpdate::Foreach(f) => f.write_cypher(w),
        }
    }
}

impl ToCypher for CallSubquery {
    fn write_cypher(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        write!(w, "CALL {{ ")?;
        self.query.write_cypher(w)?;
        write!(w, " }}")?;
        if let Some(it) = &self.in_transactions {
            write!(w, " IN TRANSACTIONS")?;
            if let Some(rows) = &it.of_rows {
                write!(w, " OF ")?;
                rows.write_cypher(w)?;
                write!(w, " ROWS")?;
            }
            if let Some(on_err) = &it.on_error {
                write!(w, " ON ERROR ")?;
                on_err.write_cypher(w)?;
            }
        }
        Ok(())
    }
}

impl ToCypher for OnErrorBehavior {
    fn write_cypher(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        match self {
            OnErrorBehavior::Continue => write!(w, "CONTINUE"),
            OnErrorBehavior::Break => write!(w, "BREAK"),
            OnErrorBehavior::Fail => write!(w, "FAIL"),
        }
    }
}

impl ToCypher for SchemaCommand {
    fn write_cypher(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        match self {
            SchemaCommand::CreateIndex(ci) => ci.write_cypher(w),
            SchemaCommand::DropIndex(di) => di.write_cypher(w),
            SchemaCommand::CreateConstraint(cc) => cc.write_cypher(w),
            SchemaCommand::DropConstraint(dc) => dc.write_cypher(w),
        }
    }
}

impl ToCypher for CreateIndex {
    fn write_cypher(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        write!(w, "CREATE ")?;
        if let Some(kind) = &self.kind {
            kind.write_cypher(w)?;
            write!(w, " ")?;
        }
        write!(w, "INDEX ")?;
        if self.if_not_exists {
            write!(w, "IF NOT EXISTS ")?;
        }
        if let Some(name) = &self.name {
            name.write_cypher(w)?;
            write!(w, " ")?;
        }
        write!(w, "FOR ")?;
        self.target.write_cypher(w)?;
        if let Some(opts) = &self.options {
            write!(w, " OPTIONS ")?;
            opts.write_cypher(w)?;
        }
        Ok(())
    }
}

impl ToCypher for IndexKind {
    fn write_cypher(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        match self {
            IndexKind::Range => write!(w, "RANGE"),
            IndexKind::Text => write!(w, "TEXT"),
            IndexKind::Point => write!(w, "POINT"),
            IndexKind::Lookup => write!(w, "LOOKUP"),
            IndexKind::Fulltext => write!(w, "FULLTEXT"),
        }
    }
}

impl ToCypher for DropIndex {
    fn write_cypher(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        write!(w, "DROP INDEX ")?;
        if self.if_exists {
            write!(w, "IF EXISTS ")?;
        }
        self.name.write_cypher(w)
    }
}

impl ToCypher for CreateConstraint {
    fn write_cypher(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        write!(w, "CREATE CONSTRAINT ")?;
        if let Some(name) = &self.name {
            name.write_cypher(w)?;
            write!(w, " ")?;
        }
        write!(w, "FOR (")?;
        self.variable.write_cypher(w)?;
        write!(w, ") ")?;
        self.kind.write_cypher(w)
    }
}

impl ToCypher for ConstraintKind {
    fn write_cypher(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        match self {
            ConstraintKind::Unique => write!(w, "IS UNIQUE"),
            ConstraintKind::NodeKey { properties } => {
                write!(w, "NODE KEY IS UNIQUE")?;
                if !properties.is_empty() {
                    write!(w, " (")?;
                    for (i, p) in properties.iter().enumerate() {
                        if i > 0 {
                            write!(w, ", ")?;
                        }
                        p.write_cypher(w)?;
                    }
                    write!(w, ")")?;
                }
                Ok(())
            }
            ConstraintKind::NotNull => write!(w, "IS NOT NULL"),
            ConstraintKind::PropertyType { types } => {
                write!(w, "PROPERTY TYPE IS (")?;
                for (i, t) in types.iter().enumerate() {
                    if i > 0 {
                        write!(w, " | ")?;
                    }
                    t.write_cypher(w)?;
                }
                write!(w, ")")
            }
        }
    }
}

impl ToCypher for DropConstraint {
    fn write_cypher(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        write!(w, "DROP CONSTRAINT ")?;
        if self.if_exists {
            write!(w, "IF EXISTS ")?;
        }
        self.name.write_cypher(w)
    }
}

impl ToCypher for Show {
    fn write_cypher(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        write!(w, "SHOW ")?;
        self.kind.write_cypher(w)?;
        if let Some(yield_spec) = &self.yield_items {
            write!(w, " YIELD ")?;
            match yield_spec {
                ShowYieldSpec::Star { .. } => write!(w, "*")?,
                ShowYieldSpec::Items(items) => {
                    for (i, item) in items.iter().enumerate() {
                        if i > 0 {
                            write!(w, ", ")?;
                        }
                        item.write_cypher(w)?;
                    }
                }
            }
        }
        if let Some(wc) = &self.where_clause {
            write!(w, " WHERE ")?;
            wc.write_cypher(w)?;
        }
        if let Some(ret) = &self.return_clause {
            write!(w, " ")?;
            ret.write_cypher(w)?;
        }
        Ok(())
    }
}

impl ToCypher for ShowKind {
    fn write_cypher(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        match self {
            ShowKind::Indexes => write!(w, "INDEXES"),
            ShowKind::Constraints => write!(w, "CONSTRAINTS"),
            ShowKind::Functions => write!(w, "FUNCTIONS"),
            ShowKind::Procedures => write!(w, "PROCEDURES"),
            ShowKind::Databases => write!(w, "DATABASES"),
            ShowKind::Database(name) => {
                write!(w, "DATABASE ")?;
                name.write_cypher(w)
            }
        }
    }
}

impl ToCypher for ReturnBody {
    fn write_cypher(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        write!(w, "RETURN ")?;
        if self.distinct {
            write!(w, "DISTINCT ")?;
        }
        for (i, item) in self.items.iter().enumerate() {
            if i > 0 {
                write!(w, ", ")?;
            }
            item.write_cypher(w)?;
        }
        if let Some(order) = &self.order {
            write!(w, " ")?;
            order.write_cypher(w)?;
        }
        if let Some(skip) = &self.skip {
            write!(w, " SKIP ")?;
            skip.write_cypher(w)?;
        }
        if let Some(limit) = &self.limit {
            write!(w, " LIMIT ")?;
            limit.write_cypher(w)?;
        }
        Ok(())
    }
}

impl ToCypher for Use {
    fn write_cypher(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        write!(w, "USE ")?;
        self.graph.write_cypher(w)
    }
}

impl ToCypher for ShowYieldItem {
    fn write_cypher(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        self.procedure_field.write_cypher(w)?;
        if let Some(alias) = &self.alias {
            write!(w, " AS ")?;
            alias.write_cypher(w)?;
        }
        Ok(())
    }
}

impl ToCypher for MapProjection {
    fn write_cypher(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        self.base.write_cypher(w)?;
        write!(w, "{{")?;
        for (i, item) in self.items.iter().enumerate() {
            if i > 0 {
                write!(w, ", ")?;
            }
            item.write_cypher(w)?;
        }
        write!(w, "}}")
    }
}

impl ToCypher for MapProjectionItem {
    fn write_cypher(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        match self {
            MapProjectionItem::AllProperties { .. } => write!(w, ".*"),
            MapProjectionItem::PropertyLookup { property } => {
                write!(w, ".")?;
                property.write_cypher(w)
            }
            MapProjectionItem::Literal { key, value } => {
                key.write_cypher(w)?;
                write!(w, ": ")?;
                value.write_cypher(w)
            }
        }
    }
}
