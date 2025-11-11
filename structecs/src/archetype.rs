use std::{hash::Hash, sync::Arc};

use parking_lot::RwLock;
use rustc_hash::FxHashMap;

use crate::{Acquirable, Extractable};

#[derive(Debug, Default, Clone)]
pub struct Archetype<Key: Copy + Eq + Hash, Base: Extractable> {
    map: Arc<RwLock<FxHashMap<Key, Acquirable<Base>>>>,
}

impl<Key: Copy + Eq + Hash, Base: Extractable> Archetype<Key, Base> {
    pub fn insert<U: Extractable>(&self, key: Key, value: U) -> Option<Acquirable<Base>> {
        let acquirable = Acquirable::new(value);
        // TODO: compile time check that U: Base or U derives from Base
        let acquirable = unsafe { acquirable.extract::<Base>().unwrap_unchecked() };

        let mut map = self.map.write();
        map.insert(key, acquirable)
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
