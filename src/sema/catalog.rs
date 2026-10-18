//! The functions and procedures semantic analysis knows.
//!
//! A [`Catalog`] maps function and procedure names to [`Signature`]s. The
//! default catalog holds the built-in functions of Neo4j 5 and GQL; plugin
//! functions (APOC, GDS) and procedures are registered by the caller.
//!
//! Analysis checks calls of known functions and procedures against their
//! signatures. Calls of unknown ones pass, unless the catalog is
//! [strict](Catalog::set_strict), which reports them.

use std::collections::HashMap;
use std::sync::LazyLock;

use crate::ast::expr::FunctionInvocation;
use crate::error::ValueTypes;

/// The parameters, result and kind of a function or procedure.
///
/// # Example
///
/// ```
/// use decypher::ValueTypes;
/// use decypher::sema::Signature;
///
/// // left(string, length) :: STRING
/// let left = Signature::new(&[ValueTypes::STRING, ValueTypes::INTEGER])
///     .returns(ValueTypes::STRING);
/// assert_eq!((left.min_args(), left.max_args()), (2, Some(2)));
///
/// // coalesce(value, …) takes one or more arguments.
/// let coalesce = Signature::new(&[ValueTypes::ANY]).variadic();
/// assert_eq!((coalesce.min_args(), coalesce.max_args()), (1, None));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signature {
    params: Vec<ValueTypes>,
    required: usize,
    variadic: bool,
    returns: ValueTypes,
    element: ValueTypes,
    aggregate: bool,
}

impl Signature {
    /// A signature taking one required argument per entry of `params`, of
    /// the types given, and returning any value.
    pub fn new(params: &[ValueTypes]) -> Self {
        Self {
            params: params.to_vec(),
            required: params.len(),
            variadic: false,
            returns: ValueTypes::ANY,
            element: ValueTypes::ANY,
            aggregate: false,
        }
    }

    /// A signature taking any number of arguments of any type.
    pub fn any() -> Self {
        Self::new(&[ValueTypes::ANY]).optional(1).variadic()
    }

    /// Makes the last `count` parameters optional.
    pub fn optional(mut self, count: usize) -> Self {
        self.required = self.params.len().saturating_sub(count);
        self
    }

    /// Lets the last parameter repeat any number of times.
    pub fn variadic(mut self) -> Self {
        self.variadic = true;
        self
    }

    /// Sets the result type.
    pub fn returns(mut self, types: ValueTypes) -> Self {
        self.returns = types;
        self
    }

    /// Makes the result a list of `element` values.
    pub fn returns_list_of(mut self, element: ValueTypes) -> Self {
        self.returns = ValueTypes::LIST;
        self.element = element;
        self
    }

    /// Marks the function as an aggregate.
    pub fn aggregate(mut self) -> Self {
        self.aggregate = true;
        self
    }

    /// The fewest arguments a call passes.
    pub fn min_args(&self) -> usize {
        self.required
    }

    /// The most arguments a call passes, or `None` when unbounded.
    pub fn max_args(&self) -> Option<usize> {
        (!self.variadic).then_some(self.params.len())
    }

    /// The types accepted for the argument at `index`.
    pub fn param(&self, index: usize) -> ValueTypes {
        match self.params.get(index) {
            Some(types) => *types,
            None if self.variadic => self.params.last().copied().unwrap_or(ValueTypes::ANY),
            None => ValueTypes::ANY,
        }
    }

    /// The result type.
    pub fn result(&self) -> ValueTypes {
        self.returns
    }

    /// The element type of a list result.
    pub fn result_element(&self) -> ValueTypes {
        self.element
    }

    /// Whether the function is an aggregate.
    pub fn is_aggregate(&self) -> bool {
        self.aggregate
    }
}

/// The functions and procedures known to semantic analysis, used by
/// [`crate::sema::analyze_with`] and HIR lowering.
///
/// Names are dot-joined (`apoc.agg.median`) and case-insensitive.
///
/// # Example
///
/// ```
/// use decypher::ValueTypes;
/// use decypher::sema::{Catalog, Signature};
///
/// let mut catalog = Catalog::default();
/// catalog
///     .register_aggregate("apoc.agg.median")
///     .register_procedure(
///         "db.labels",
///         Signature::new(&[]).returns(ValueTypes::STRING),
///     );
/// assert!(catalog.is_aggregate("APOC.AGG.MEDIAN"));
/// assert!(catalog.function("toUpper").is_some());
/// ```
#[derive(Debug, Clone)]
pub struct Catalog {
    /// Whether the built-in functions are known.
    builtins: bool,
    /// Registered functions, which take precedence over built-in ones.
    functions: HashMap<String, Signature>,
    procedures: HashMap<String, Signature>,
    strict: bool,
}

impl Catalog {
    /// A catalog without any functions or procedures, not even the
    /// built-in ones.
    pub fn empty() -> Self {
        Self {
            builtins: false,
            functions: HashMap::new(),
            procedures: HashMap::new(),
            strict: false,
        }
    }

    /// Registers or replaces the function `name`.
    pub fn register_function(&mut self, name: &str, signature: Signature) -> &mut Self {
        self.functions.insert(name.to_uppercase(), signature);
        self
    }

    /// Registers `name` as an aggregate taking any arguments.
    pub fn register_aggregate(&mut self, name: &str) -> &mut Self {
        self.register_function(name, Signature::any().aggregate())
    }

    /// Registers or replaces the procedure `name`.
    pub fn register_procedure(&mut self, name: &str, signature: Signature) -> &mut Self {
        self.procedures.insert(name.to_uppercase(), signature);
        self
    }

    /// With `strict`, calls of unknown functions and procedures are
    /// reported (`ErrorKind::UnknownFunction`, `ErrorKind::ProcedureNotFound`).
    /// Off by default, so queries using unregistered plugins pass.
    pub fn set_strict(&mut self, strict: bool) -> &mut Self {
        self.strict = strict;
        self
    }

    /// Whether unknown functions and procedures are reported.
    pub fn is_strict(&self) -> bool {
        self.strict
    }

    /// The signature of the function `name`.
    pub fn function(&self, name: &str) -> Option<&Signature> {
        let key = name.to_uppercase();
        self.functions
            .get(&key)
            .or_else(|| self.builtins.then(|| BUILTINS.get(&key)).flatten())
    }

    /// The signature of the procedure `name`.
    pub fn procedure(&self, name: &str) -> Option<&Signature> {
        self.procedures.get(&name.to_uppercase())
    }

    /// Whether `name` is an aggregate function.
    pub fn is_aggregate(&self, name: &str) -> bool {
        self.function(name).is_some_and(Signature::is_aggregate)
    }

    /// Whether the function called by `fc` is an aggregate.
    pub(crate) fn is_aggregate_call(&self, fc: &FunctionInvocation) -> bool {
        self.is_aggregate(&qualified_name(fc))
    }
}

/// The dot-joined name of a call (`apoc.agg.median`).
pub(crate) fn qualified_name(fc: &FunctionInvocation) -> String {
    fc.name
        .iter()
        .map(|segment| segment.name.as_str())
        .collect::<Vec<_>>()
        .join(".")
}

impl Default for Catalog {
    /// The built-in functions of Neo4j 5 and GQL; no procedures.
    fn default() -> Self {
        Self {
            builtins: true,
            ..Self::empty()
        }
    }
}

/// The built-in functions, by upper-case name.
static BUILTINS: LazyLock<HashMap<String, Signature>> = LazyLock::new(|| {
    builtins()
        .into_iter()
        .map(|(name, signature)| (name.to_uppercase(), signature))
        .collect()
});

fn builtins() -> Vec<(String, Signature)> {
    const A: ValueTypes = ValueTypes::ANY;
    const B: ValueTypes = ValueTypes::BOOLEAN;
    const I: ValueTypes = ValueTypes::INTEGER;
    const F: ValueTypes = ValueTypes::FLOAT;
    const N: ValueTypes = ValueTypes::NUMBER;
    const S: ValueTypes = ValueTypes::STRING;
    const L: ValueTypes = ValueTypes::LIST;
    const M: ValueTypes = ValueTypes::MAP;
    const NODE: ValueTypes = ValueTypes::NODE;
    const REL: ValueTypes = ValueTypes::RELATIONSHIP;
    const PATH: ValueTypes = ValueTypes::PATH;
    const P: ValueTypes = ValueTypes::POINT;
    const T: ValueTypes = ValueTypes::TEMPORAL;
    const D: ValueTypes = ValueTypes::DURATION;
    let f = Signature::new;

    let mut functions: Vec<(String, Signature)> = [
        // Aggregates
        ("avg", f(&[N | D]).returns(N | D).aggregate()),
        ("collect", f(&[A]).returns_list_of(A).aggregate()),
        ("count", f(&[A]).returns(I).aggregate()),
        ("max", f(&[A]).aggregate()),
        ("min", f(&[A]).aggregate()),
        ("percentileCont", f(&[N, N]).returns(F).aggregate()),
        ("percentileDisc", f(&[N, N]).returns(N).aggregate()),
        ("stDev", f(&[N]).returns(F).aggregate()),
        ("stDevP", f(&[N]).returns(F).aggregate()),
        ("sum", f(&[N | D]).returns(N | D).aggregate()),
        ("var", f(&[N]).returns(F).aggregate()),
        ("varP", f(&[N]).returns(F).aggregate()),
        // Predicates
        ("exists", f(&[A]).returns(B)),
        ("isEmpty", f(&[L | M | S]).returns(B)),
        // Scalar
        ("char_length", f(&[S]).returns(I)),
        ("character_length", f(&[S]).returns(I)),
        ("coalesce", f(&[A]).variadic()),
        ("elementId", f(&[NODE | REL]).returns(S)),
        ("endNode", f(&[REL]).returns(NODE)),
        ("head", f(&[L])),
        ("id", f(&[NODE | REL]).returns(I)),
        ("last", f(&[L])),
        ("length", f(&[PATH | S | L]).returns(I)),
        ("nullIf", f(&[A, A])),
        ("properties", f(&[NODE | REL | M]).returns(M)),
        ("randomUUID", f(&[]).returns(S)),
        ("size", f(&[L | S]).returns(I)),
        ("startNode", f(&[REL]).returns(NODE)),
        ("timestamp", f(&[]).returns(I)),
        ("toBoolean", f(&[S | B | I]).returns(B)),
        ("toBooleanOrNull", f(&[A]).returns(B)),
        ("toFloat", f(&[S | N]).returns(F)),
        ("toFloatOrNull", f(&[A]).returns(F)),
        ("toInteger", f(&[S | N | B]).returns(I)),
        ("toIntegerOrNull", f(&[A]).returns(I)),
        ("type", f(&[REL]).returns(S)),
        ("valueType", f(&[A]).returns(S)),
        // Lists
        ("keys", f(&[NODE | REL | M]).returns_list_of(S)),
        ("labels", f(&[NODE]).returns_list_of(S)),
        ("nodes", f(&[PATH]).returns_list_of(NODE)),
        ("range", f(&[I, I, I]).optional(1).returns_list_of(I)),
        ("relationships", f(&[PATH]).returns_list_of(REL)),
        ("reverse", f(&[L | S]).returns(L | S)),
        ("tail", f(&[L]).returns(L)),
        ("toBooleanList", f(&[L]).returns_list_of(B)),
        ("toFloatList", f(&[L]).returns_list_of(F)),
        ("toIntegerList", f(&[L]).returns_list_of(I)),
        ("toStringList", f(&[L]).returns_list_of(S)),
        // Numeric
        ("abs", f(&[N]).returns(N)),
        ("ceil", f(&[N]).returns(F)),
        ("floor", f(&[N]).returns(F)),
        ("isNaN", f(&[N]).returns(B)),
        ("rand", f(&[]).returns(F)),
        ("round", f(&[N, I, S]).optional(2).returns(F)),
        ("sign", f(&[N]).returns(I)),
        ("e", f(&[]).returns(F)),
        ("pi", f(&[]).returns(F)),
        ("atan2", f(&[N, N]).returns(F)),
        // Strings
        ("btrim", f(&[S, S]).optional(1).returns(S)),
        ("left", f(&[S, I]).returns(S)),
        ("lower", f(&[S]).returns(S)),
        ("ltrim", f(&[S, S]).optional(1).returns(S)),
        ("normalize", f(&[S, A]).optional(1).returns(S)),
        ("replace", f(&[S, S, S]).returns(S)),
        ("right", f(&[S, I]).returns(S)),
        ("rtrim", f(&[S, S]).optional(1).returns(S)),
        ("split", f(&[S, S | L]).returns_list_of(S)),
        ("substring", f(&[S, I, I]).optional(1).returns(S)),
        ("toLower", f(&[S]).returns(S)),
        ("toString", f(&[A]).returns(S)),
        ("toStringOrNull", f(&[A]).returns(S)),
        ("toUpper", f(&[S]).returns(S)),
        // `trim(BOTH 'x' FROM s)` has its own syntax.
        ("trim", Signature::any().returns(S)),
        ("upper", f(&[S]).returns(S)),
        // Spatial
        ("point", f(&[M]).returns(P)),
        ("distance", f(&[P, P]).returns(F)),
        ("point.distance", f(&[P, P]).returns(F)),
        ("point.withinBBox", f(&[P, P, P]).returns(B)),
        // Durations
        ("duration", f(&[A]).returns(D)),
        ("datetime.fromepoch", f(&[N, N]).returns(T)),
        ("datetime.fromepochmillis", f(&[N]).returns(T)),
        // Vectors
        ("vector.similarity.cosine", f(&[L, L]).returns(F)),
        ("vector.similarity.euclidean", f(&[L, L]).returns(F)),
        // Graphs
        ("graph.names", f(&[]).returns_list_of(S)),
        ("graph.byName", f(&[S])),
        ("graph.byElementId", f(&[S])),
        ("graph.propertiesByName", f(&[S]).returns(M)),
        ("db.nameFromElementId", f(&[S]).returns(S)),
        // LOAD CSV
        ("file", f(&[]).returns(S)),
        ("linenumber", f(&[]).returns(I)),
    ]
    .into_iter()
    .map(|(name, signature)| (name.to_owned(), signature))
    .collect();
    for name in [
        "exp", "log", "log10", "sqrt", "acos", "asin", "atan", "cos", "cot", "degrees", "haversin",
        "radians", "sin", "tan",
    ] {
        functions.push((name.to_owned(), f(&[N]).returns(F)));
    }
    for name in ["between", "inMonths", "inDays", "inSeconds"] {
        functions.push((format!("duration.{name}"), f(&[A, A]).returns(D)));
    }
    for temporal in ["date", "datetime", "localdatetime", "localtime", "time"] {
        functions.push((temporal.to_owned(), f(&[A]).optional(1).returns(T)));
        for clock in ["realtime", "statement", "transaction"] {
            functions.push((
                format!("{temporal}.{clock}"),
                f(&[A]).optional(1).returns(T),
            ));
        }
        functions.push((
            format!("{temporal}.truncate"),
            f(&[S, A, M]).optional(1).returns(T),
        ));
    }
    functions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_has_builtin_aggregates() {
        let catalog = Catalog::default();
        for name in ["count", "SUM", "Avg", "collect", "percentileCont", "stDevP"] {
            assert!(catalog.is_aggregate(name), "{name} should be an aggregate");
        }
        assert!(!catalog.is_aggregate("toUpper"));
        assert!(!catalog.is_aggregate("apoc.coll.count"));
    }

    #[test]
    fn registers_plugin_functions_and_procedures() {
        let mut catalog = Catalog::default();
        catalog
            .register_aggregate("apoc.agg.percentiles")
            .register_procedure("db.labels", Signature::new(&[]));
        assert!(catalog.is_aggregate("APOC.agg.percentiles"));
        assert!(catalog.procedure("DB.LABELS").is_some());
        assert!(Catalog::empty().function("count").is_none());
    }

    #[test]
    fn builtin_names_parse_as_calls() {
        for (name, signature) in builtins() {
            let arguments = vec!["1"; signature.min_args()].join(", ");
            let query = format!("RETURN {name}({arguments}) AS x");
            let ast = crate::parse(query.as_str()).unwrap_or_else(|e| panic!("{query}: {e}"));
            let call = crate::ast::ToCypher::to_cypher(&ast);
            assert!(
                call.contains(&format!("{name}(")),
                "{query} printed as {call}"
            );
        }
    }

    #[test]
    fn signatures_count_optional_and_variadic_arguments() {
        let catalog = Catalog::default();
        let round = catalog.function("round").unwrap();
        assert_eq!((round.min_args(), round.max_args()), (1, Some(3)));
        let coalesce = catalog.function("coalesce").unwrap();
        assert_eq!((coalesce.min_args(), coalesce.max_args()), (1, None));
        assert_eq!(coalesce.param(5), ValueTypes::ANY);
        let truncate = catalog.function("datetime.truncate").unwrap();
        assert_eq!((truncate.min_args(), truncate.max_args()), (2, Some(3)));
    }
}
