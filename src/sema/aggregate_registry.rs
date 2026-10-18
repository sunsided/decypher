use std::collections::HashSet;

/// Registry of aggregate function names, used by HIR lowering and by
/// [`crate::sema::check_aggregation_with`] to tell aggregates from plain
/// functions.
///
/// Names are stored and matched using the dot-joined form (e.g.
/// `"apoc.agg.percentiles"`). Backtick-escaped segments whose unescaped
/// text contains a literal dot (e.g. `` `apoc.agg`.percentiles ``) are
/// normalised to the same dot-joined form and will match the same registry
/// entry. If your codebase uses such names as both aggregates and
/// non-aggregates, the aggregate classification may produce false positives.
#[derive(Debug, Clone)]
pub struct AggregateRegistry {
    names: HashSet<String>,
}

impl AggregateRegistry {
    /// Registers `name` (dot-joined, case-insensitive) as an aggregate.
    pub fn register(&mut self, name: &str) -> &mut Self {
        self.names.insert(name.to_uppercase());
        self
    }

    /// Whether `qualified_name` (dot-joined, case-insensitive) is an aggregate.
    pub fn contains(&self, qualified_name: &str) -> bool {
        self.names.contains(&qualified_name.to_uppercase())
    }

    /// Whether the function called by `fc` is an aggregate.
    pub(crate) fn is_aggregate_call(&self, fc: &crate::ast::expr::FunctionInvocation) -> bool {
        let name = fc
            .name
            .iter()
            .map(|segment| segment.name.as_str())
            .collect::<Vec<_>>()
            .join(".");
        self.contains(&name)
    }
}

impl Default for AggregateRegistry {
    fn default() -> Self {
        let names = [
            "COUNT",
            "SUM",
            "AVG",
            "MIN",
            "MAX",
            "COLLECT",
            "PERCENTILE_CONT",
            "PERCENTILE_DISC",
            "STDEV",
            "STDEVP",
            "VAR",
            "VARP",
        ]
        .iter()
        .map(|&s| s.to_string())
        .collect();
        Self { names }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_contains_builtins() {
        let r = AggregateRegistry::default();
        for name in &[
            "COUNT",
            "SUM",
            "AVG",
            "MIN",
            "MAX",
            "COLLECT",
            "PERCENTILE_CONT",
            "PERCENTILE_DISC",
            "STDEV",
            "STDEVP",
            "VAR",
            "VARP",
        ] {
            assert!(r.contains(name), "{name} should be a built-in aggregate");
        }
    }

    #[test]
    fn test_register_custom() {
        let mut r = AggregateRegistry::default();
        assert!(!r.contains("apoc.agg.percentiles"));
        r.register("apoc.agg.percentiles");
        assert!(r.contains("apoc.agg.percentiles"));
    }

    #[test]
    fn test_lookup_is_case_insensitive() {
        let r = AggregateRegistry::default();
        assert!(r.contains("count"));
        assert!(r.contains("Count"));
        assert!(r.contains("COUNT"));
    }

    #[test]
    fn test_qualified_name_not_in_default() {
        let r = AggregateRegistry::default();
        assert!(!r.contains("apoc.coll.count"));
    }
}
