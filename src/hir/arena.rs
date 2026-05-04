use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Id(pub usize);

pub type ScopeId = Id;
pub type BindingId = Id;
pub type ExprId = Id;
pub type LabelId = Id;
pub type RelTypeId = Id;
pub type PropertyKeyId = Id;
pub type ParameterId = Id;
pub type FunctionId = Id;

/// Simple arena: linear allocation, indexed access.
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
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn alloc(&mut self, value: T) -> Id {
        let id = Id(self.entries.len());
        self.entries.push(value);
        id
    }

    pub fn get(&self, id: Id) -> &T {
        &self.entries[id.0]
    }

    pub fn get_mut(&mut self, id: Id) -> &mut T {
        &mut self.entries[id.0]
    }

    pub fn iter(&self) -> impl Iterator<Item = (Id, &T)> {
        self.entries.iter().enumerate().map(|(i, v)| (Id(i), v))
    }
}

impl<T> Default for Arena<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Interner: maps strings to compact IDs, deduplicates.
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
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
            next: 0,
        }
    }

    pub fn intern(&mut self, name: &str, mk: impl FnOnce(usize) -> T) -> T {
        if let Some(&id) = self.map.get(name) {
            return id;
        }
        let id = mk(self.next);
        self.next += 1;
        self.map.insert(name.to_string(), id);
        id
    }

    pub fn resolve(&self, name: &str) -> Option<T> {
        self.map.get(name).copied()
    }

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

/// All HIR arenas in one place.
#[derive(Debug, Clone)]
pub struct HirArenas {
    pub scopes: Arena<super::binding::Scope>,
    pub bindings: Arena<super::binding::Binding>,
    pub expressions: Arena<super::expr::HirExpr>,

    pub labels: Interner<LabelId>,
    pub relationship_types: Interner<RelTypeId>,
    pub property_keys: Interner<PropertyKeyId>,
    pub parameters: Interner<ParameterId>,
    pub functions: Interner<FunctionId>,
}

impl HirArenas {
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
