use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A typed identifier for any timeline aggregate. Stored as a UUID
/// text column; the `T` marker keeps a space id from being passed
/// where a message id is wanted, with zero runtime cost.
#[derive(Serialize, Deserialize)]
#[serde(transparent)]
pub struct Id<T> {
    value: Uuid,
    #[serde(skip)]
    _marker: std::marker::PhantomData<fn() -> T>,
}

impl<T> Id<T> {
    pub fn new() -> Self {
        Self::from_uuid(Uuid::new_v4())
    }

    pub fn from_uuid(value: Uuid) -> Self {
        Self {
            value,
            _marker: std::marker::PhantomData,
        }
    }

    pub fn as_uuid(&self) -> Uuid {
        self.value
    }
}

impl<T> Default for Id<T> {
    fn default() -> Self {
        Self::new()
    }
}

// PhantomData<fn() -> T> would force T: Clone/Copy/etc. onto the derive
// bounds; deriving by hand keeps Id<T> usable for any marker T.
impl<T> Clone for Id<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> Copy for Id<T> {}
impl<T> PartialEq for Id<T> {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}
impl<T> Eq for Id<T> {}
impl<T> std::hash::Hash for Id<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.value.hash(state);
    }
}
impl<T> std::fmt::Display for Id<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.value, f)
    }
}
impl<T> std::fmt::Debug for Id<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Id({})", self.value)
    }
}
