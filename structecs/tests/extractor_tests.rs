#![allow(dead_code)]

use structecs::*;

/// Test basic offset calculation with simple struct
#[test]
fn test_extractor_simple_offset() {
    #[derive(Extractable, PartialEq, Debug)]
    struct Simple {
        a: u32,
        b: u64,
        c: String,
    }

    let simple = Simple {
        a: 42,
        b: 100,
        c: "test".to_string(),
    };

    let acquirable = Acquirable::new(simple);
    let extracted = acquirable.extract::<Simple>().unwrap();

    assert_eq!(extracted.a, 42);
    assert_eq!(extracted.b, 100);
    assert_eq!(extracted.c, "test");
}

/// Test offset calculation with nested structures
#[test]
fn test_extractor_nested_offset() {
    #[derive(Extractable, PartialEq, Debug)]
    struct Inner {
        value: i32,
    }

    #[derive(Extractable, PartialEq, Debug)]
    #[extractable(inner)]
    struct Middle {
        data: String,
        inner: Inner,
    }

    #[derive(Extractable, PartialEq, Debug)]
    #[extractable(middle)]
    struct Outer {
        id: u64,
        middle: Middle,
    }

    let outer = Outer {
        id: 999,
        middle: Middle {
            data: "test".to_string(),
            inner: Inner { value: -42 },
        },
    };

    let acquirable = Acquirable::new(outer);

    // Extract from each level
    let inner = acquirable.extract::<Inner>().unwrap();
    assert_eq!(inner.value, -42);

    let middle = acquirable.extract::<Middle>().unwrap();
    assert_eq!(middle.data, "test");
    assert_eq!(middle.inner.value, -42);

    let outer = acquirable.extract::<Outer>().unwrap();
    assert_eq!(outer.id, 999);
}

/// Test offset calculation with multiple fields at same level
#[test]
fn test_extractor_multiple_fields() {
    #[derive(Extractable, PartialEq, Debug)]
    struct ComponentA {
        a: u8,
    }

    #[derive(Extractable, PartialEq, Debug)]
    struct ComponentB {
        b: u16,
    }

    #[derive(Extractable, PartialEq, Debug)]
    struct ComponentC {
        c: u32,
    }

    #[derive(Extractable, PartialEq, Debug)]
    #[extractable(comp_a, comp_b, comp_c)]
    struct Entity {
        id: u64,
        comp_a: ComponentA,
        comp_b: ComponentB,
        comp_c: ComponentC,
    }

    let entity = Entity {
        id: 123,
        comp_a: ComponentA { a: 1 },
        comp_b: ComponentB { b: 2 },
        comp_c: ComponentC { c: 3 },
    };

    let acquirable = Acquirable::new(entity);

    // Extract all components in different order
    let comp_c = acquirable.extract::<ComponentC>().unwrap();
    assert_eq!(comp_c.c, 3);

    let comp_a = acquirable.extract::<ComponentA>().unwrap();
    assert_eq!(comp_a.a, 1);

    let comp_b = acquirable.extract::<ComponentB>().unwrap();
    assert_eq!(comp_b.b, 2);

    let entity = acquirable.extract::<Entity>().unwrap();
    assert_eq!(entity.id, 123);
}

/// Test offset calculation with complex alignment requirements
#[test]
fn test_extractor_alignment() {
    #[repr(align(16))]
    #[derive(Extractable, PartialEq, Debug)]
    struct Aligned16 {
        data: [u8; 16],
    }

    #[derive(Extractable, PartialEq, Debug)]
    #[extractable(aligned)]
    struct Container {
        small: u8,
        aligned: Aligned16,
    }

    let mut data = [0u8; 16];
    (0..16).for_each(|i| {
        data[i] = i as u8;
    });

    let container = Container {
        small: 42,
        aligned: Aligned16 { data },
    };

    let acquirable = Acquirable::new(container);

    let aligned = acquirable.extract::<Aligned16>().unwrap();
    assert_eq!(aligned.data[0], 0);
    assert_eq!(aligned.data[15], 15);

    let container = acquirable.extract::<Container>().unwrap();
    assert_eq!(container.small, 42);
}

/// Test offset calculation with large nested structures
#[test]
fn test_extractor_large_nested() {
    #[derive(Extractable, PartialEq, Debug)]
    struct Level1 {
        buffer: [u64; 10],
    }

    #[derive(Extractable, PartialEq, Debug)]
    #[extractable(level1)]
    struct Level2 {
        data: Vec<u32>,
        level1: Level1,
    }

    #[derive(Extractable, PartialEq, Debug)]
    #[extractable(level2)]
    struct Level3 {
        flag: bool,
        level2: Level2,
    }

    let mut buffer = [0u64; 10];
    (0..10).for_each(|i| {
        buffer[i] = i as u64 * 100;
    });

    let entity = Level3 {
        flag: true,
        level2: Level2 {
            data: vec![1, 2, 3, 4, 5],
            level1: Level1 { buffer },
        },
    };

    let acquirable = Acquirable::new(entity);

    let level1 = acquirable.extract::<Level1>().unwrap();
    assert_eq!(level1.buffer[5], 500);

    let level2 = acquirable.extract::<Level2>().unwrap();
    assert_eq!(level2.data, vec![1, 2, 3, 4, 5]);

    let level3 = acquirable.extract::<Level3>().unwrap();
    assert!(level3.flag);
}

/// Test that extraction fails for non-existent types
#[test]
fn test_extractor_wrong_type() {
    #[derive(Extractable)]
    struct EntityA {
        id: u32,
    }

    #[derive(Extractable)]
    struct EntityB {
        id: u32,
    }

    #[derive(Extractable)]
    struct EntityC {
        id: u32,
    }

    let entity = Acquirable::new(EntityA { id: 1 });

    // Should succeed for EntityA
    assert!(entity.extract::<EntityA>().is_some());

    // Should fail for EntityB and EntityC
    assert!(entity.extract::<EntityB>().is_none());
    assert!(entity.extract::<EntityC>().is_none());
}

/// Test offset calculation with mixed sized fields
#[test]
fn test_extractor_mixed_sizes() {
    #[derive(Extractable, PartialEq, Debug)]
    struct Tiny {
        a: u8,
    }

    #[derive(Extractable, PartialEq, Debug)]
    struct Small {
        b: u16,
    }

    #[derive(Extractable, PartialEq, Debug)]
    struct Medium {
        c: u32,
    }

    #[derive(Extractable, PartialEq, Debug)]
    struct Large {
        d: u64,
    }

    #[derive(Extractable, PartialEq, Debug)]
    #[extractable(tiny, small, medium, large)]
    struct MixedSize {
        tiny: Tiny,
        small: Small,
        medium: Medium,
        large: Large,
    }

    let entity = MixedSize {
        tiny: Tiny { a: 1 },
        small: Small { b: 2 },
        medium: Medium { c: 3 },
        large: Large { d: 4 },
    };

    let acquirable = Acquirable::new(entity);

    assert_eq!(acquirable.extract::<Tiny>().unwrap().a, 1);
    assert_eq!(acquirable.extract::<Small>().unwrap().b, 2);
    assert_eq!(acquirable.extract::<Medium>().unwrap().c, 3);
    assert_eq!(acquirable.extract::<Large>().unwrap().d, 4);
}

/// Test offset calculation with tuple-like structs
#[test]
fn test_extractor_tuple_struct() {
    #[derive(Extractable, PartialEq, Debug)]
    struct Point(i32, i32);

    let point = Point(10, 20);
    let acquirable = Acquirable::new(point);
    let extracted = acquirable.extract::<Point>().unwrap();

    assert_eq!(extracted.0, 10);
    assert_eq!(extracted.1, 20);
}

/// Test offset calculation across multiple extractions
#[test]
fn test_extractor_multiple_extractions() {
    #[derive(Extractable, PartialEq, Debug)]
    struct Component {
        value: i32,
    }

    #[derive(Extractable, PartialEq, Debug)]
    #[extractable(component)]
    struct Entity {
        id: u64,
        component: Component,
    }

    let entity = Entity {
        id: 999,
        component: Component { value: 42 },
    };

    let acquirable = Acquirable::new(entity);

    // Multiple extractions should all work correctly
    for _ in 0..10 {
        let comp = acquirable.extract::<Component>().unwrap();
        assert_eq!(comp.value, 42);

        let ent = acquirable.extract::<Entity>().unwrap();
        assert_eq!(ent.id, 999);
    }
}

/// Test offset calculation with Option fields
#[test]
fn test_extractor_option_fields() {
    #[derive(Extractable, PartialEq, Debug)]
    struct Entity {
        required: u32,
        optional: Option<String>,
    }

    let entity1 = Entity {
        required: 42,
        optional: Some("test".to_string()),
    };

    let acquirable1 = Acquirable::new(entity1);
    let extracted1 = acquirable1.extract::<Entity>().unwrap();
    assert_eq!(extracted1.required, 42);
    assert_eq!(extracted1.optional, Some("test".to_string()));

    let entity2 = Entity {
        required: 100,
        optional: None,
    };

    let acquirable2 = Acquirable::new(entity2);
    let extracted2 = acquirable2.extract::<Entity>().unwrap();
    assert_eq!(extracted2.required, 100);
    assert_eq!(extracted2.optional, None);
}

/// Test offset calculation with Vec fields
#[test]
fn test_extractor_vec_fields() {
    #[derive(Extractable, PartialEq, Debug)]
    struct Entity {
        id: u32,
        items: Vec<i32>,
    }

    let entity = Entity {
        id: 1,
        items: vec![10, 20, 30, 40, 50],
    };

    let acquirable = Acquirable::new(entity);
    let extracted = acquirable.extract::<Entity>().unwrap();

    assert_eq!(extracted.id, 1);
    assert_eq!(extracted.items, vec![10, 20, 30, 40, 50]);
}
