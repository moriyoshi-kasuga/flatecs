#![allow(dead_code)]

use structecs::*;

/// Test basic extractable metadata functionality
#[test]
fn test_extractable_simple_struct() {
    #[derive(Extractable, PartialEq, Debug)]
    struct Simple {
        id: u32,
        name: String,
    }

    let simple = Simple {
        id: 42,
        name: "test".to_string(),
    };

    let acquirable = Acquirable::new(simple);
    let extracted = acquirable.extract::<Simple>().unwrap();

    assert_eq!(extracted.id, 42);
    assert_eq!(extracted.name, "test");
}

/// Test nested extraction with single level
#[test]
fn test_extractable_nested_single_level() {
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

    // Extract inner component
    let extracted_inner = acquirable.extract::<Inner>().unwrap();
    assert_eq!(extracted_inner.value, 100);

    // Extract outer component
    let extracted_outer = acquirable.extract::<Outer>().unwrap();
    assert_eq!(extracted_outer.name, "outer");
    assert_eq!(extracted_outer.inner.value, 100);
}

/// Test deeply nested extraction (3 levels)
#[test]
fn test_extractable_deeply_nested() {
    #[derive(Extractable, PartialEq, Debug)]
    struct Level1 {
        value: u8,
    }

    #[derive(Extractable, PartialEq, Debug)]
    #[extractable(level1)]
    struct Level2 {
        data: String,
        level1: Level1,
    }

    #[derive(Extractable, PartialEq, Debug)]
    #[extractable(level2)]
    struct Level3 {
        flag: bool,
        level2: Level2,
    }

    let entity = Level3 {
        flag: true,
        level2: Level2 {
            data: "middle".to_string(),
            level1: Level1 { value: 255 },
        },
    };

    let acquirable = Acquirable::new(entity);

    // Extract from all levels
    let l1 = acquirable.extract::<Level1>().unwrap();
    assert_eq!(l1.value, 255);

    let l2 = acquirable.extract::<Level2>().unwrap();
    assert_eq!(l2.data, "middle");

    let l3 = acquirable.extract::<Level3>().unwrap();
    assert!(l3.flag);
}

/// Test multiple extractable fields at the same level
#[test]
fn test_extractable_multiple_fields() {
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
    struct Entity {
        id: u64,
        comp_a: ComponentA,
        comp_b: ComponentB,
    }

    let entity = Entity {
        id: 999,
        comp_a: ComponentA { a: 10 },
        comp_b: ComponentB {
            b: "data".to_string(),
        },
    };

    let acquirable = Acquirable::new(entity);

    // Extract both components
    let a = acquirable.extract::<ComponentA>().unwrap();
    assert_eq!(a.a, 10);

    let b = acquirable.extract::<ComponentB>().unwrap();
    assert_eq!(b.b, "data");

    // Extract the entity itself
    let entity = acquirable.extract::<Entity>().unwrap();
    assert_eq!(entity.id, 999);
}

/// Test extraction failure for non-existent type
#[test]
fn test_extraction_failure() {
    #[derive(Extractable)]
    struct EntityA {
        id: u32,
    }

    #[derive(Extractable)]
    struct EntityB {
        id: u32,
    }

    let entity = Acquirable::new(EntityA { id: 1 });

    // Attempting to extract EntityB should return None
    assert!(entity.extract::<EntityB>().is_none());
}

/// Test with empty struct
#[test]
fn test_extractable_empty_struct() {
    #[derive(Extractable, PartialEq, Debug)]
    struct Empty {}

    let empty = Empty {};
    let acquirable = Acquirable::new(empty);
    let extracted = acquirable.extract::<Empty>().unwrap();

    assert_eq!(*extracted, Empty {});
}

/// Test with tuple struct
#[test]
fn test_extractable_tuple_struct() {
    #[derive(Extractable, PartialEq, Debug)]
    struct TupleStruct(u32, String);

    let tuple = TupleStruct(42, "test".to_string());
    let acquirable = Acquirable::new(tuple);
    let extracted = acquirable.extract::<TupleStruct>().unwrap();

    assert_eq!(extracted.0, 42);
    assert_eq!(extracted.1, "test");
}

/// Test extraction with references remains valid
#[test]
fn test_extraction_reference_lifetime() {
    #[derive(Extractable, Debug)]
    struct Entity {
        data: Vec<u32>,
    }

    let entity = Acquirable::new(Entity {
        data: vec![1, 2, 3, 4, 5],
    });

    let extracted = entity.extract::<Entity>().unwrap();
    assert_eq!(extracted.data.len(), 5);
    assert_eq!(extracted.data[2], 3);

    // The reference should remain valid even after multiple extractions
    let extracted2 = entity.extract::<Entity>().unwrap();
    assert_eq!(extracted2.data.len(), 5);

    // Both references should point to the same data
    assert_eq!(extracted.data[0], extracted2.data[0]);
}

/// Test metadata flattening with complex hierarchy
#[test]
fn test_metadata_flattening_complex() {
    #[derive(Extractable, PartialEq, Debug)]
    struct Base {
        base_id: u32,
    }

    #[derive(Extractable, PartialEq, Debug)]
    #[extractable(base)]
    struct Middle {
        middle_data: String,
        base: Base,
    }

    #[derive(Extractable, PartialEq, Debug)]
    #[extractable(middle)]
    struct Top {
        top_flag: bool,
        middle: Middle,
    }

    let top = Top {
        top_flag: true,
        middle: Middle {
            middle_data: "mid".to_string(),
            base: Base { base_id: 42 },
        },
    };

    let acquirable = Acquirable::new(top);

    // Should be able to extract Base directly from Top via flattened metadata
    let base = acquirable.extract::<Base>().unwrap();
    assert_eq!(base.base_id, 42);

    // Should also extract intermediate types
    let middle = acquirable.extract::<Middle>().unwrap();
    assert_eq!(middle.middle_data, "mid");
    assert_eq!(middle.base.base_id, 42);
}

/// Test with mixed extractable and non-extractable fields
#[test]
fn test_mixed_extractable_fields() {
    #[derive(Extractable, PartialEq, Debug)]
    struct Component {
        value: i32,
    }

    #[derive(Extractable, PartialEq, Debug)]
    #[extractable(component)]
    struct Entity {
        id: u64,
        name: String,
        component: Component,
        data: Vec<u8>,
    }

    let entity = Entity {
        id: 100,
        name: "test".to_string(),
        component: Component { value: 42 },
        data: vec![1, 2, 3],
    };

    let acquirable = Acquirable::new(entity);

    // Extract the component
    let comp = acquirable.extract::<Component>().unwrap();
    assert_eq!(comp.value, 42);

    // Extract the whole entity
    let ent = acquirable.extract::<Entity>().unwrap();
    assert_eq!(ent.id, 100);
    assert_eq!(ent.name, "test");
    assert_eq!(ent.data, vec![1, 2, 3]);
}

/// Test cloning acquirable with extractable data
#[test]
fn test_clone_with_extraction() {
    #[derive(Extractable, PartialEq, Debug)]
    struct Entity {
        id: u32,
    }

    let entity = Acquirable::new(Entity { id: 42 });
    let cloned = entity.clone();

    // Both should extract the same data
    let extracted1 = entity.extract::<Entity>().unwrap();
    let extracted2 = cloned.extract::<Entity>().unwrap();

    assert_eq!(extracted1.id, 42);
    assert_eq!(extracted2.id, 42);

    // They should point to the same underlying data
    assert!(entity.ptr_eq(&cloned));
}
