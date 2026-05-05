//! Arena allocators and string interners for the HIR.
//!
//! The HIR avoids heap-allocated strings and heap-indirection by storing
//! all heap objects inside typed [`Arena`]s and referencing them via
//! compact integer [`Id`] handles. String-valued entities (label names,
//! relationship types, etc.) are additionally deduplicated by [`Interner`].

use std::collections::HashMap;

/// A compact arena index.
///
/// Arenas use plain `usize` indices wrapped in this newtype so that
/// different ID spaces (`ScopeId`, `BindingId`, `ExprId`, …) are
/// type-distinct and cannot be accidentally interchanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Id(pub usize);

/// Arena index for a [`crate::hir::binding::Scope`].
pub type ScopeId = Id;
/// Arena index for a [`crate::hir::binding::Binding`].
pub type BindingId = Id;
/// Arena index for a [`crate::hir::expr::HirExpr`].
pub type ExprId = Id;
/// Arena index for an interned label name.
pub type LabelId = Id;
/// Arena index for an interned relationship-type name.
pub type RelTypeId = Id;
/// Arena index for an interned property-key name.
pub type PropertyKeyId = Id;
/// Arena index for an interned parameter name.
pub type ParameterId = Id;
/// Arena index for an interned function name.
pub type FunctionId = Id;

/// A growable arena that owns its entries and grants `O(1)` indexed access.
///
/// Entries are allocated in FIFO order; the returned [`Id`] can be used to
/// retrieve the entry later via [`Arena::get`].
pub struct Arena<T> {
    entries: Vec<T>,
}

impl<T: std::fmt::Debug> std::fmt::Debug for Arena<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Arena")
            .field("len", &self.entries.len())
            .finish()
    }
}

impl<T: Clone> Clone for Arena<T> {
    fn clone(&self) -> Self {
        Self {
            entries: self.entries.clone(),
        }
    }
}

impl<T> Arena<T> {
    /// Create an empty arena.
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Allocate `value` in the arena and return its [`Id`].
    ///
    /// IDs are assigned sequentially starting from `0`.
    pub fn alloc(&mut self, value: T) -> Id {
        let id = Id(self.entries.len());
        self.entries.push(value);
        id
    }

    /// Return a shared reference to the entry at `id`.
    ///
    /// # Panics
    ///
    /// Panics if `id` is out of bounds (i.e. was not produced by this arena).
    pub fn get(&self, id: Id) -> &T {
        &self.entries[id.0]
    }

    /// Return a mutable reference to the entry at `id`.
    ///
    /// # Panics
    ///
    /// Panics if `id` is out of bounds.
    pub fn get_mut(&mut self, id: Id) -> &mut T {
        &mut self.entries[id.0]
    }

    /// Iterate over all `(Id, &T)` pairs in allocation order.
    pub fn iter(&self) -> impl Iterator<Item = (Id, &T)> {
        self.entries.iter().enumerate().map(|(i, v)| (Id(i), v))
    }
}

impl<T> Default for Arena<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// A string-keyed interner that maps names to compact typed IDs.
///
/// Repeated calls to [`Interner::intern`] with the same string return the
/// same ID, deduplicating storage.
pub struct Interner<T: Copy + Clone> {
    map: HashMap<String, T>,
    next: usize,
}

impl<T: Copy + Clone> Clone for Interner<T> {
    fn clone(&self) -> Self {
        Self {
            map: self.map.clone(),
            next: self.next,
        }
    }
}

impl<T: Copy + Clone> std::fmt::Debug for Interner<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Interner")
            .field("len", &self.map.len())
            .finish()
    }
}

impl<T: Copy + Clone> Interner<T> {
    /// Create an empty interner.
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
            next: 0,
        }
    }

    /// Intern `name`, returning its ID.
    ///
    /// If `name` has been interned before the existing ID is returned and
    /// `mk` is **not** called. Otherwise `mk` is called with the next
    /// sequential index to produce a fresh ID.
    pub fn intern(&mut self, name: &str, mk: impl FnOnce(usize) -> T) -> T {
        if let Some(&id) = self.map.get(name) {
            return id;
        }
        let id = mk(self.next);
        self.next += 1;
        self.map.insert(name.to_string(), id);
        id
    }

    /// Look up `name`, returning its ID if already interned.
    pub fn resolve(&self, name: &str) -> Option<T> {
        self.map.get(name).copied()
    }

    /// Reverse-lookup: find the name for the given `id`.
    ///
    /// This is a linear scan and intended for debugging only.
    pub fn name_of(&self, id: T) -> Option<&str>
    where
        T: PartialEq,
    {
        self.map
            .iter()
            .find(|(_, v)| **v == id)
            .map(|(k, _)| k.as_str())
    }
}

impl<T: Copy + Clone> Default for Interner<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Bundles all HIR arenas together for convenient passing by reference.
///
/// All arena and interner fields are public so that the lowering pass and
/// consumers can allocate into and read from them freely.
#[derive(Debug, Clone)]
pub struct HirArenas {
    /// Scope arena (`Scope` objects).
    pub scopes: Arena<super::binding::Scope>,
    /// Binding arena (`Binding` objects — resolved variables).
    pub bindings: Arena<super::binding::Binding>,
    /// Expression arena (`HirExpr` nodes).
    pub expressions: Arena<super::expr::HirExpr>,

    /// Interner for node/relationship label names.
    pub labels: Interner<LabelId>,
    /// Interner for relationship type names.
    pub relationship_types: Interner<RelTypeId>,
    /// Interner for property key names.
    pub property_keys: Interner<PropertyKeyId>,
    /// Interner for query parameter names.
    pub parameters: Interner<ParameterId>,
    /// Interner for function and procedure names.
    pub functions: Interner<FunctionId>,
}

impl HirArenas {
    /// Create a new, empty set of HIR arenas.
    pub fn new() -> Self {
        Self {
            scopes: Arena::new(),
            bindings: Arena::new(),
            expressions: Arena::new(),
            labels: Interner::new(),
            relationship_types: Interner::new(),
            property_keys: Interner::new(),
            parameters: Interner::new(),
            functions: Interner::new(),
        }
    }
}

impl Default for HirArenas {
    fn default() -> Self {
        Self::new()
    }
}
