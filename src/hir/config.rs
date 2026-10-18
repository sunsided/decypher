use crate::sema::Catalog;

/// Options for HIR lowering.
#[derive(Debug, Clone, Default)]
pub struct LowerConfig {
    /// The functions and procedures calls are checked against, and which
    /// functions aggregate.
    pub catalog: Catalog,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_has_builtins() {
        let config = LowerConfig::default();
        assert!(config.catalog.is_aggregate("COUNT"));
        assert!(!config.catalog.is_aggregate("apoc.coll.count"));
    }

    #[test]
    fn test_custom_aggregate_registration() {
        let mut config = LowerConfig::default();
        config.catalog.register_aggregate("custom.agg");
        assert!(config.catalog.is_aggregate("custom.agg"));
    }
}
