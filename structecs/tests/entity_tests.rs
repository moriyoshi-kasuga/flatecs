#![allow(dead_code)]

use structecs::*;

/// Test basic EntityData creation and extraction
#[test]
fn test_entity_data_creation() {
    #[derive(Extractable, PartialEq, Debug)]
    struct TestEntity {
        id: u32,
        name: String,
    }

    let entity = TestEntity {
        id: 42,
        name: "test".to_string(),
    };

    let acquirable = Acquirable::new(entity);
    let extracted = acquirable.extract::<TestEntity>().unwrap();

    assert_eq!(extracted.id, 42);
    assert_eq!(extracted.name, "test");
}

/// Test EntityData with nested components
#[test]
fn test_entity_data_nested_extraction() {
    #[derive(Extractable, PartialEq, Debug)]
    struct Inner {
        value: i32,
    }

    #[derive(Extractable, PartialEq, Debug)]
    #[extractable(inner)]
    struct Outer {
        name: String,
        inner: Inner,
    }

    let outer = Outer {
        name: "outer".to_string(),
        inner: Inner { value: 100 },
    };

    let acquirable = Acquirable::new(outer);

    // Extract nested component
    let inner = acquirable.extract::<Inner>().unwrap();
    assert_eq!(inner.value, 100);

    // Extract outer component
    let outer = acquirable.extract::<Outer>().unwrap();
    assert_eq!(outer.name, "outer");
}

/// Test EntityData drop behavior
#[test]
fn test_entity_data_drop() {
    use std::sync::atomic::{AtomicBool, Ordering};

    static DROPPED: AtomicBool = AtomicBool::new(false);

    #[derive(Extractable)]
    struct Entity {
        _data: Vec<u32>,
    }

    impl Drop for Entity {
        fn drop(&mut self) {
            DROPPED.store(true, Ordering::SeqCst);
        }
    }

    {
        let entity = Acquirable::new(Entity {
            _data: vec![1, 2, 3],
        });
        assert!(!DROPPED.load(Ordering::SeqCst));
        drop(entity);
    }

    // Entity should be dropped
    assert!(DROPPED.load(Ordering::SeqCst));
}

/// Test EntityData drop with multiple references
#[test]
fn test_entity_data_drop_with_clones() {
    use std::sync::atomic::{AtomicU32, Ordering};

    static DROP_COUNT: AtomicU32 = AtomicU32::new(0);

    #[derive(Extractable)]
    struct Entity {
        id: u32,
    }

    impl Drop for Entity {
        fn drop(&mut self) {
            DROP_COUNT.fetch_add(1, Ordering::SeqCst);
        }
    }

    DROP_COUNT.store(0, Ordering::SeqCst);

    {
        let entity1 = Acquirable::new(Entity { id: 1 });
        let entity2 = entity1.clone();
        let entity3 = entity1.clone();

        drop(entity1);
        // Entity should not be dropped yet
        assert_eq!(DROP_COUNT.load(Ordering::SeqCst), 0);

        drop(entity2);
        // Entity should still not be dropped
        assert_eq!(DROP_COUNT.load(Ordering::SeqCst), 0);

        drop(entity3);
        // Now entity should be dropped
        assert_eq!(DROP_COUNT.load(Ordering::SeqCst), 1);
    }
}

/// Test EntityData extraction returns None for wrong type
#[test]
fn test_entity_data_wrong_type() {
    #[derive(Extractable)]
    struct EntityA {
        id: u32,
    }

    #[derive(Extractable)]
    struct EntityB {
        id: u32,
    }

    let entity = Acquirable::new(EntityA { id: 1 });

    // Should return None for EntityB
    assert!(entity.extract::<EntityB>().is_none());

    // Should return Some for EntityA
    assert!(entity.extract::<EntityA>().is_some());
}

/// Test EntityData with large data structures
#[test]
fn test_entity_data_large_structure() {
    #[derive(Extractable, PartialEq, Debug)]
    struct LargeEntity {
        data: Vec<u8>,
        buffer: [u64; 100],
    }

    let mut buffer = [0u64; 100];
    (0..100).for_each(|i| {
        buffer[i] = i as u64;
    });

    let entity = LargeEntity {
        data: vec![1, 2, 3, 4, 5],
        buffer,
    };

    let acquirable = Acquirable::new(entity);
    let extracted = acquirable.extract::<LargeEntity>().unwrap();

    assert_eq!(extracted.data, vec![1, 2, 3, 4, 5]);
    assert_eq!(extracted.buffer[50], 50);
    assert_eq!(extracted.buffer[99], 99);
}

/// Test EntityData Send + Sync implementation
#[test]
fn test_entity_data_send_sync() {
    use std::thread;

    #[derive(Extractable, PartialEq, Debug)]
    struct Entity {
        id: u32,
    }

    let entity = Acquirable::new(Entity { id: 42 });
    let entity_clone = entity.clone();

    // Send to another thread
    let handle = thread::spawn(move || {
        let extracted = entity_clone.extract::<Entity>().unwrap();
        extracted.id
    });

    let result = handle.join().unwrap();
    assert_eq!(result, 42);

    // Original should still be valid
    let extracted = entity.extract::<Entity>().unwrap();
    assert_eq!(extracted.id, 42);
}

/// Test EntityData with multiple threads accessing simultaneously
#[test]
fn test_entity_data_concurrent_access() {
    use std::thread;

    #[derive(Extractable, PartialEq, Debug)]
    struct Entity {
        value: u64,
    }

    let entity = Acquirable::new(Entity { value: 100 });

    let handles: Vec<_> = (0..10)
        .map(|_| {
            let entity_clone = entity.clone();
            thread::spawn(move || {
                let extracted = entity_clone.extract::<Entity>().unwrap();
                extracted.value
            })
        })
        .collect();

    for handle in handles {
        let result = handle.join().unwrap();
        assert_eq!(result, 100);
    }
}

/// Test EntityData with complex nested structure
#[test]
fn test_entity_data_complex_nested() {
    #[derive(Extractable, PartialEq, Debug)]
    struct ComponentA {
        a: u32,
    }

    #[derive(Extractable, PartialEq, Debug)]
    struct ComponentB {
        b: String,
    }

    #[derive(Extractable, PartialEq, Debug)]
    #[extractable(comp_a, comp_b)]
    struct ComponentC {
        c: bool,
        comp_a: ComponentA,
        comp_b: ComponentB,
    }

    #[derive(Extractable, PartialEq, Debug)]
    #[extractable(comp_c)]
    struct Entity {
        id: u64,
        comp_c: ComponentC,
    }

    let entity = Entity {
        id: 999,
        comp_c: ComponentC {
            c: true,
            comp_a: ComponentA { a: 10 },
            comp_b: ComponentB {
                b: "data".to_string(),
            },
        },
    };

    let acquirable = Acquirable::new(entity);

    // Extract from all levels
    let comp_a = acquirable.extract::<ComponentA>().unwrap();
    assert_eq!(comp_a.a, 10);

    let comp_b = acquirable.extract::<ComponentB>().unwrap();
    assert_eq!(comp_b.b, "data");

    let comp_c = acquirable.extract::<ComponentC>().unwrap();
    assert!(comp_c.c);

    let entity = acquirable.extract::<Entity>().unwrap();
    assert_eq!(entity.id, 999);
}

/// Test EntityData with zero-sized types
#[test]
fn test_entity_data_zero_sized() {
    #[derive(Extractable, PartialEq, Debug)]
    struct ZeroSized;

    let acquirable = Acquirable::new(ZeroSized);
    let extracted = acquirable.extract::<ZeroSized>().unwrap();

    assert_eq!(*extracted, ZeroSized);
}
