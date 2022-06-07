use core::fmt;
use core::hash::{Hash, Hasher};
use std::collections::HashMap;
use std::ops::Deref;

use gc_arena::{Gc, MutationContext};
use gc_arena_derive::Collect;

use crate::object::ObjString;

/// Represents a symbol
#[derive(Debug, Copy, Clone, Collect, PartialOrd, Ord)]
#[collect(no_drop)]
pub struct Symbol<'gc>(Token<'gc>);

impl<'gc> Symbol<'gc> {
    /// Generates a new uninterned symbol
    pub fn uninterned(token: Token<'gc>) -> Self {
        Self(token)
    }

    fn as_token(&self) -> Token<'gc> {
        self.0
    }
}

impl PartialEq for Symbol<'_> {
    fn eq(&self, other: &Self) -> bool {
        Gc::ptr_eq(self.as_token().0, other.as_token().0)
    }
}

impl Eq for Symbol<'_> {}

impl Hash for Symbol<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        Gc::as_ptr(self.as_token().0).hash(state);
    }
}

impl fmt::Display for Symbol<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl Deref for Symbol<'_> {
    type Target = ObjString;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Used for the symbol pool key so that the actual string hashes are compared
/// instead of pointer values
#[derive(Debug, Copy, Clone, Collect)]
#[collect(no_drop)]
pub struct Token<'gc>(Gc<'gc, ObjString>);

impl PartialEq for Token<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq(&other.0)
    }
}

impl Eq for Token<'_> {}

impl PartialOrd for Token<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

impl Ord for Token<'_> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}

impl Hash for Token<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl Deref for Token<'_> {
    type Target = ObjString;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'gc> Token<'gc> {
    pub fn new(mc: MutationContext<'gc, '_>, string: ObjString) -> Self {
        Self(Gc::allocate(mc, string))
    }
}

impl<'gc> From<Gc<'gc, ObjString>> for Token<'gc> {
    fn from(value: Gc<'gc, ObjString>) -> Self {
        Self(value)
    }
}

#[derive(Clone, Collect, Debug, Default)]
#[collect(no_drop)]
pub struct SymbolTable<'gc>(HashMap<Token<'gc>, Symbol<'gc>>);

impl<'gc> SymbolTable<'gc> {
    pub fn intern(&mut self, token: Token<'gc>) -> Symbol<'gc> {
        *self
            .0
            .entry(token)
            .or_insert_with(|| Symbol::uninterned(token))
    }
}
