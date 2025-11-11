#![allow(dead_code)]

use structecs::*;

#[test]
fn sample_usage() {
    #[derive(Extractable, PartialEq, Debug)]
    struct Entity {
        id: u32,
    }

    #[derive(Extractable, PartialEq, Debug)]
    #[extractable(entity)]
    struct NamedEntity {
        name: String,
        entity: Entity,
    }

    let named = NamedEntity {
        name: "Test".to_string(),
        entity: Entity { id: 42 },
    };
    let acquirable = Acquirable::new(named);
    let extracted_entity = acquirable.extract::<Entity>().unwrap();
    assert_eq!(*extracted_entity, Entity { id: 42 });
}

#[test]
fn test_weak_acquirable() {
    #[derive(Extractable, PartialEq, Debug)]
    struct Entity {
        id: u32,
    }

    // Test upgrade when entity is alive
    let entity = Acquirable::new(Entity { id: 42 });
    let weak = entity.downgrade();

    assert!(weak.upgrade().is_some());
    assert_eq!(weak.upgrade().unwrap().id, 42);

    // Test upgrade after entity is dropped
    drop(entity);
    assert!(weak.upgrade().is_none());
}

#[test]
fn test_weak_acquirable_clone() {
    #[derive(Extractable, PartialEq, Debug)]
    struct Entity {
        id: u32,
    }

    let entity = Acquirable::new(Entity { id: 42 });
    let weak1 = entity.downgrade();
    let weak2 = weak1.clone();

    // Both weak references should work
    assert!(weak1.upgrade().is_some());
    assert!(weak2.upgrade().is_some());

    drop(entity);

    // Both should fail after entity is dropped
    assert!(weak1.upgrade().is_none());
    assert!(weak2.upgrade().is_none());
}

#[test]
fn test_ptr_eq() {
    #[derive(Extractable)]
    struct Entity {
        id: u32,
    }

    let entity1 = Acquirable::new(Entity { id: 42 });
    let entity2 = entity1.clone();
    let entity3 = Acquirable::new(Entity { id: 42 });

    // Same entity
    assert!(entity1.ptr_eq(&entity2));

    // Different entities
    assert!(!entity1.ptr_eq(&entity3));
}

#[test]
fn test_reference_counting() {
    #[derive(Extractable)]
    struct Entity {
        id: u32,
    }

    let entity = Acquirable::new(Entity { id: 42 });
    assert_eq!(entity.strong_count(), 1);
    assert_eq!(entity.weak_count(), 0);

    let entity2 = entity.clone();
    assert_eq!(entity.strong_count(), 2);
    assert_eq!(entity.weak_count(), 0);

    let weak = entity.downgrade();
    assert_eq!(entity.strong_count(), 2);
    assert_eq!(entity.weak_count(), 1);

    let _weak2 = weak.clone();
    assert_eq!(entity.strong_count(), 2);
    assert_eq!(entity.weak_count(), 2);

    drop(weak);
    assert_eq!(entity.strong_count(), 2);
    assert_eq!(entity.weak_count(), 1);

    drop(entity2);
    assert_eq!(entity.strong_count(), 1);
    assert_eq!(entity.weak_count(), 1);
}

#[test]
fn test_circular_reference_prevention() {
    use std::cell::RefCell;

    #[derive(Extractable)]
    struct Node {
        id: u32,
        // Use weak reference to prevent circular reference
        parent: RefCell<Option<WeakAcquirable<Node>>>,
    }

    let parent = Acquirable::new(Node {
        id: 1,
        parent: RefCell::new(None),
    });

    let child = Acquirable::new(Node {
        id: 2,
        parent: RefCell::new(Some(parent.downgrade())),
    });

    // Parent is still alive
    assert!(child.parent.borrow().as_ref().unwrap().upgrade().is_some());

    drop(parent);

    // Parent is dropped, weak reference returns None
    assert!(child.parent.borrow().as_ref().unwrap().upgrade().is_none());
}

#[test]
fn test_handler_no_circular_reference() {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU32, Ordering};

    #[derive(Extractable)]
    struct Entity {
        id: u32,
    }

    // Counter to verify handler is called correctly
    let call_count = Arc::new(AtomicU32::new(0));
    let call_count_clone = call_count.clone();

    // Create handler that captures external state
    let handler = ComponentHandler::<Entity>::for_type::<Entity>(move |entity, ()| {
        call_count_clone.fetch_add(entity.id, Ordering::SeqCst);
    });

    let entity = Acquirable::new(Entity { id: 1 });

    // Call handler multiple times
    handler.call(&entity, ());
    handler.call(&entity, ());
    handler.call(&entity, ());

    assert_eq!(call_count.load(Ordering::SeqCst), 3);

    // Entity should be droppable even with handler still alive
    drop(entity);

    // Handler should still be usable with new entities
    let entity2 = Acquirable::new(Entity { id: 10 });
    handler.call(&entity2, ());

    assert_eq!(call_count.load(Ordering::SeqCst), 13);
}

#[test]
fn test_handler_long_running_usage() {
    #[derive(Extractable, Clone)]
    struct Entity {
        value: String,
    }

    // Create a handler that performs complex operations
    let handler = ComponentHandler::<Entity, u32, String>::for_type::<Entity>(|entity, input| {
        format!("{}-{}", entity.value, input)
    });

    // Test with many entities over time
    for i in 0..100 {
        let entity = Acquirable::new(Entity {
            value: format!("entity_{}", i),
        });

        let result = handler.call(&entity, i);
        assert_eq!(result, format!("entity_{}-{}", i, i));
    }
}

#[test]
fn test_handler_reference_counting() {
    #[derive(Extractable)]
    struct Entity {
        id: u32,
    }

    let entity = Acquirable::new(Entity { id: 42 });
    assert_eq!(entity.strong_count(), 1);

    let _handler = ComponentHandler::<Entity>::for_type::<Entity>(|_entity, ()| {
        // Handler closure should not increase entity's reference count
    });

    // Handler creation should not affect entity's reference count
    assert_eq!(entity.strong_count(), 1);

    // Calling handler with entity reference should not affect count
    _handler.call(&entity, ());
    assert_eq!(entity.strong_count(), 1);
}

#[test]
fn test_handler_with_multiple_entity_types() {
    #[derive(Extractable)]
    struct Base {
        id: u32,
    }

    #[derive(Extractable)]
    #[extractable(base)]
    struct TypeA {
        base: Base,
        name: String,
    }

    #[derive(Extractable)]
    #[extractable(base)]
    struct TypeB {
        base: Base,
        value: i32,
    }

    let handler_a =
        ComponentHandler::<Base, (), String>::for_type::<TypeA>(|entity, ()| entity.name.clone());

    let handler_b = ComponentHandler::<Base, (), i32>::for_type::<TypeB>(|entity, ()| entity.value);

    let entity_a = Acquirable::new(TypeA {
        base: Base { id: 1 },
        name: "A".to_string(),
    });

    let entity_b = Acquirable::new(TypeB {
        base: Base { id: 2 },
        value: 100,
    });

    assert_eq!(handler_a.call(&entity_a, ()), "A");
    assert_eq!(handler_b.call(&entity_b, ()), 100);
}
