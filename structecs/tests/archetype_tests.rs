#![cfg(feature = "archetype")]
#![allow(dead_code)]

use structecs::*;

/// Test concurrent insertions from multiple threads
#[test]
fn test_archetype_concurrent_insert() {
    use std::sync::Arc;
    use std::thread;

    #[derive(Extractable, Debug, Clone)]
    struct Entity {
        id: u32,
    }

    let archetype = Arc::new(Archetype::<u32, Entity>::default());

    let handles: Vec<_> = (0..10)
        .map(|i| {
            let archetype_clone = archetype.clone();
            thread::spawn(move || {
                for j in 0..10 {
                    let id = i * 10 + j;
                    archetype_clone.insert(id, Entity { id });
                }
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }

    // Verify all entities were inserted
    for i in 0..100 {
        assert!(archetype.get(&i).is_some());
        let entity = archetype.get(&i).unwrap();
        assert_eq!(entity.id, i);
    }
}

/// Test concurrent reads from multiple threads
#[test]
fn test_archetype_concurrent_read() {
    use std::sync::Arc;
    use std::thread;

    #[derive(Extractable, Debug)]
    struct Entity {
        value: u64,
    }

    let archetype = Arc::new(Archetype::<u32, Entity>::default());

    // Insert some entities
    for i in 0..100 {
        archetype.insert(
            i,
            Entity {
                value: i as u64 * 100,
            },
        );
    }

    let handles: Vec<_> = (0..10)
        .map(|_| {
            let archetype_clone = archetype.clone();
            thread::spawn(move || {
                for i in 0..100 {
                    let entity = archetype_clone.get(&i).unwrap();
                    assert_eq!(entity.value, i as u64 * 100);
                }
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }
}

/// Test concurrent insert and remove operations
#[test]
fn test_archetype_concurrent_insert_remove() {
    use std::sync::Arc;
    use std::thread;

    #[derive(Extractable, Debug)]
    struct Entity {
        id: u32,
    }

    let archetype = Arc::new(Archetype::<u32, Entity>::default());

    // Pre-insert some entities
    for i in 0..50 {
        archetype.insert(i, Entity { id: i });
    }

    let archetype1 = archetype.clone();
    let archetype2 = archetype.clone();

    let inserter = thread::spawn(move || {
        for i in 50..100 {
            archetype1.insert(i, Entity { id: i });
        }
    });

    let remover = thread::spawn(move || {
        for i in 0..50 {
            archetype2.remove(&i);
        }
    });

    inserter.join().unwrap();
    remover.join().unwrap();

    // Verify state
    for i in 0..50 {
        assert!(archetype.get(&i).is_none());
    }
    for i in 50..100 {
        assert!(archetype.get(&i).is_some());
    }
}

/// Test removing non-existent keys
#[test]
fn test_archetype_remove_nonexistent() {
    #[derive(Extractable, Debug)]
    struct Entity {
        id: u32,
    }

    let archetype = Archetype::<u32, Entity>::default();

    // Remove from empty archetype
    assert!(archetype.remove(&1).is_none());
    assert!(archetype.remove(&999).is_none());

    // Insert and remove
    archetype.insert(1, Entity { id: 1 });
    assert!(archetype.remove(&1).is_some());

    // Remove again - should be None
    assert!(archetype.remove(&1).is_none());
}

/// Test getting non-existent keys
#[test]
fn test_archetype_get_nonexistent() {
    #[derive(Extractable, Debug)]
    struct Entity {
        id: u32,
    }

    let archetype = Archetype::<u32, Entity>::default();

    // Get from empty archetype
    assert!(archetype.get(&1).is_none());
    assert!(archetype.get(&999).is_none());

    archetype.insert(1, Entity { id: 1 });

    // Get existing key
    assert!(archetype.get(&1).is_some());

    // Get non-existent key
    assert!(archetype.get(&2).is_none());
}

/// Test inserting large number of entities
#[test]
fn test_archetype_large_insert() {
    #[derive(Extractable, Debug)]
    struct Entity {
        value: usize,
    }

    let archetype = Archetype::<usize, Entity>::default();

    for i in 0..1000 {
        archetype.insert(i, Entity { value: i * 10 });
    }

    // Verify all entities
    for i in 0..1000 {
        assert_eq!(archetype.get(&i).unwrap().value, i * 10);
    }
}

/// Test archetype memory cleanup
#[test]
fn test_archetype_memory_cleanup() {
    use std::sync::atomic::{AtomicU32, Ordering};

    static DROP_COUNT: AtomicU32 = AtomicU32::new(0);

    #[derive(Extractable)]
    struct Entity {
        _data: Vec<u32>,
    }

    impl Drop for Entity {
        fn drop(&mut self) {
            DROP_COUNT.fetch_add(1, Ordering::SeqCst);
        }
    }

    DROP_COUNT.store(0, Ordering::SeqCst);

    {
        let archetype = Archetype::<u32, Entity>::default();

        for i in 0..10 {
            archetype.insert(
                i,
                Entity {
                    _data: vec![1, 2, 3],
                },
            );
        }

        // Remove half
        for i in 0..5 {
            archetype.remove(&i);
        }

        assert_eq!(DROP_COUNT.load(Ordering::SeqCst), 5);
    }

    // Archetype dropped - remaining entities should be cleaned up
    assert_eq!(DROP_COUNT.load(Ordering::SeqCst), 10);
}

/// Test archetype with nested extractable entities
#[test]
fn test_archetype_nested_entities() {
    #[derive(Extractable, Debug)]
    struct Base {
        id: u32,
    }

    #[derive(Extractable, Debug)]
    #[extractable(base)]
    struct Extended {
        name: String,
        base: Base,
    }

    let archetype = Archetype::<u32, Extended>::default();

    archetype.insert(
        1,
        Extended {
            name: "First".to_string(),
            base: Base { id: 100 },
        },
    );

    archetype.insert(
        2,
        Extended {
            name: "Second".to_string(),
            base: Base { id: 200 },
        },
    );

    let entity1 = archetype.get(&1).unwrap();
    assert_eq!(entity1.name, "First");

    // Extract nested component
    let base1 = entity1.extract::<Base>().unwrap();
    assert_eq!(base1.id, 100);
}

/// Test archetype with multiple overwrites
#[test]
fn test_archetype_multiple_overwrites() {
    #[derive(Extractable, Debug)]
    struct Entity {
        value: u32,
    }

    let archetype = Archetype::<u32, Entity>::default();

    // Insert, overwrite multiple times
    archetype.insert(1, Entity { value: 100 });
    archetype.insert(1, Entity { value: 200 });
    archetype.insert(1, Entity { value: 300 });

    assert_eq!(archetype.get(&1).unwrap().value, 300);
}

/// Test archetype with unit key
#[test]
fn test_archetype_unit_key() {
    #[derive(Extractable, Debug)]
    struct Entity {
        id: u32,
    }

    let archetype = Archetype::<(), Entity>::default();

    archetype.insert((), Entity { id: 42 });

    assert_eq!(archetype.get(&()).unwrap().id, 42);
}

/// Test archetype inner() access
#[test]
fn test_archetype_inner_access() {
    #[derive(Extractable, Debug)]
    struct Entity {
        id: u32,
    }

    let archetype = Archetype::<u32, Entity>::default();

    archetype.insert(1, Entity { id: 10 });
    archetype.insert(2, Entity { id: 20 });

    // Access inner map
    let inner = archetype.inner();
    let map = inner.read();

    assert_eq!(map.len(), 2);
    assert!(map.contains_key(&1));
    assert!(map.contains_key(&2));
}

/// Test archetype with different primitive key types
#[test]
fn test_archetype_primitive_keys() {
    #[derive(Extractable, Debug)]
    struct Entity {
        value: i32,
    }

    // u8 key
    let arch_u8 = Archetype::<u8, Entity>::default();
    arch_u8.insert(255, Entity { value: 1 });
    assert_eq!(arch_u8.get(&255).unwrap().value, 1);

    // i32 key
    let arch_i32 = Archetype::<i32, Entity>::default();
    arch_i32.insert(-100, Entity { value: 2 });
    assert_eq!(arch_i32.get(&-100).unwrap().value, 2);

    // u64 key
    let arch_u64 = Archetype::<u64, Entity>::default();
    arch_u64.insert(9999999, Entity { value: 3 });
    assert_eq!(arch_u64.get(&9999999).unwrap().value, 3);
}

/// Test archetype with tuple keys
#[test]
fn test_archetype_tuple_keys() {
    #[derive(Extractable, Debug)]
    struct Entity {
        value: String,
    }

    let archetype = Archetype::<(u32, u32), Entity>::default();

    archetype.insert(
        (1, 2),
        Entity {
            value: "A".to_string(),
        },
    );
    archetype.insert(
        (3, 4),
        Entity {
            value: "B".to_string(),
        },
    );

    assert_eq!(archetype.get(&(1, 2)).unwrap().value, "A");
    assert_eq!(archetype.get(&(3, 4)).unwrap().value, "B");
    assert!(archetype.get(&(5, 6)).is_none());
}

/// Test archetype extractable base type retrieval
#[test]
fn test_archetype_base_type_retrieval() {
    #[derive(Extractable, Debug, PartialEq)]
    struct Base {
        id: u32,
    }

    #[derive(Extractable, Debug)]
    #[extractable(base)]
    struct DerivedA {
        name: String,
        base: Base,
    }

    #[derive(Extractable, Debug)]
    #[extractable(base)]
    struct DerivedB {
        value: i32,
        base: Base,
    }

    let archetype = Archetype::<u32, Base>::default();

    archetype.insert(
        1,
        DerivedA {
            name: "A".to_string(),
            base: Base { id: 100 },
        },
    );

    archetype.insert(
        2,
        DerivedB {
            value: 42,
            base: Base { id: 200 },
        },
    );

    // Both should be retrievable as Base
    let base1 = archetype.get(&1).unwrap();
    assert_eq!(base1.id, 100);

    let base2 = archetype.get(&2).unwrap();
    assert_eq!(base2.id, 200);
}
