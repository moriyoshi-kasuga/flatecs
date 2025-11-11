//! Thread-safe collection for storing typed entities with compile-time validation.
//!
//! The `Archetype` type provides a thread-safe key-value store that guarantees all inserted
//! values contain a specific base type (`Base`) at compile time. This is particularly useful
//! for managing collections of heterogeneous entities that share a common extractable component.
//!
//! # Design Philosophy
//!
//! Unlike traditional ECS archetypes, this implementation is **optional** and **minimal**:
//! - Users can access the underlying `Arc<RwLock<HashMap>>` via `inner()` for custom operations
//! - Additional API methods are added only when commonly needed
//! - The collection stores `Acquirable<Base>`, allowing extraction back to specific types
//!
//! # Compile-time Safety
//!
//! When inserting a value of type `U`, the compiler ensures that `U` contains `Base` as an
//! extractable component. This check happens at compile time (in debug builds), preventing
//! runtime type errors.
//!
//! # Example
//!
//! ```rust
//! use structecs::{Archetype, Extractable, Acquirable};
//!
//! #[derive(Extractable)]
//! struct Entity {
//!     id: u32,
//! }
//!
//! #[derive(Extractable)]
//! #[extractable(entity)]
//! struct Player {
//!     name: String,
//!     entity: Entity,
//! }
//!
//! // Create an archetype that stores entities by their ID
//! let entities: Archetype<u32, Entity> = Archetype::default();
//!
//! // Insert a Player (which contains Entity)
//! let player = Player {
//!     name: "Alice".to_string(),
//!     entity: Entity { id: 1 },
//! };
//! entities.insert(1, player);
//!
//! // Retrieve as base type
//! let entity = entities.get(&1).unwrap();
//!
//! // Extract back to specific type
//! let player_ref = entity.extract::<Player>().unwrap();
//! assert_eq!(player_ref.name, "Alice");
//! ```
//!
//! # Thread Safety
//!
//! `Archetype` is `Clone` (cheap Arc clone) and `Send + Sync`. Multiple clones share the same
//! underlying data, protected by a `RwLock` for concurrent access.

use std::{hash::Hash, sync::Arc};

use parking_lot::RwLock;
use rustc_hash::FxHashMap;

use crate::{Acquirable, Extractable};

/// A thread-safe collection that stores `Acquirable<Base>` values indexed by `Key`.
///
/// Insertion is compile-time checked to ensure inserted values contain `Base` as an extractable component.
#[derive(Debug)]
pub struct Archetype<Key: Copy + Eq + Hash, Base: Extractable> {
    map: Arc<RwLock<FxHashMap<Key, Acquirable<Base>>>>,
}

impl<Key: Copy + Eq + Hash, Base: Extractable> Default for Archetype<Key, Base> {
    fn default() -> Self {
        Self {
            map: Arc::new(RwLock::new(FxHashMap::default())),
        }
    }
}

impl<Key: Copy + Eq + Hash, Base: Extractable> Clone for Archetype<Key, Base> {
    fn clone(&self) -> Self {
        Self {
            map: Arc::clone(&self.map),
        }
    }
}

impl<Key: Copy + Eq + Hash, Base: Extractable> Archetype<Key, Base> {
    pub fn insert<U: Extractable>(&self, key: Key, value: U) -> Acquirable<U> {
        #[cfg(debug_assertions)]
        const {
            if !crate::ExtractionMetadata::is_has::<U, Base>() {
                panic!("Type U must contain Base as extractable component")
            }
        }

        let acquirable = Acquirable::new(value);
        // SAFETY: The compile-time check above ensures that U contains Base as an extractable component.
        // Therefore, extracting Base from U is guaranteed to succeed.
        let insert = unsafe { acquirable.inner.extract::<Base>().unwrap_unchecked() };

        let mut map = self.map.write();
        map.insert(key, insert);

        acquirable
    }

    pub fn get(&self, key: &Key) -> Option<Acquirable<Base>> {
        let map = self.map.read();
        map.get(key).cloned()
    }

    pub fn remove(&self, key: &Key) -> Option<Acquirable<Base>> {
        let mut map = self.map.write();
        map.remove(key)
    }

    pub fn contains_key(&self, key: &Key) -> bool {
        let map = self.map.read();
        map.contains_key(key)
    }

    pub fn len(&self) -> usize {
        let map = self.map.read();
        map.len()
    }

    pub fn is_empty(&self) -> bool {
        let map = self.map.read();
        map.is_empty()
    }

    pub fn clear(&self) {
        let mut map = self.map.write();
        map.clear();
    }

    pub fn read(&self) -> parking_lot::RwLockReadGuard<'_, FxHashMap<Key, Acquirable<Base>>> {
        self.map.read()
    }

    pub fn write(&self) -> parking_lot::RwLockWriteGuard<'_, FxHashMap<Key, Acquirable<Base>>> {
        self.map.write()
    }

    pub fn inner(&self) -> &Arc<RwLock<FxHashMap<Key, Acquirable<Base>>>> {
        &self.map
    }

    pub fn into_inner(self) -> Arc<RwLock<FxHashMap<Key, Acquirable<Base>>>> {
        self.map
    }
}

#[cfg(test)]
mod tests {
    use crate as structecs;
    use crate::*;

    #[derive(Extractable, Debug, PartialEq, Eq)]
    struct TestEntity {
        id: u32,
    }

    #[derive(Extractable, Debug, PartialEq, Eq)]
    #[extractable(entity)]
    struct NamedEntity {
        name: String,
        entity: TestEntity,
    }
    #[test]
    fn test_archetype_insert_get() {
        let archetype: Archetype<u32, TestEntity> = Archetype::default();
        let named_entity = NamedEntity {
            name: "Test".to_string(),
            entity: TestEntity { id: 1 },
        };
        archetype.insert(1, named_entity);
        let retrieved = archetype.get(&1).unwrap();
        let extracted = retrieved.extract::<NamedEntity>().unwrap();
        assert_eq!(
            *extracted,
            NamedEntity {
                name: "Test".to_string(),
                entity: TestEntity { id: 1 },
            }
        );
    }

    #[derive(Extractable, Debug, PartialEq, Eq)]
    struct AnotherEntity {
        value: u32,
    }

    #[test]
    fn is_compileerror_when_inserting_wrong_type() {
        let _archetype: Archetype<u32, TestEntity> = Archetype::default();
        let _another_entity = AnotherEntity { value: 42 };
        // The following line should cause a compile-time error
        // because AnotherEntity does not contain TestEntity as an extractable component.
        // Uncommenting the line below should result in a compilation failure.
        // _archetype.insert(2, _another_entity);
    }
}
