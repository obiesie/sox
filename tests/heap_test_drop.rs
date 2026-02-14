use sox::builtins::core::SoxObjectPayload;
use sox::builtins::r#type::{SoxType, SoxTypeSlot};
use sox::builtins::string::SoxString;
use sox::heap::Heap;
use sox::object::core::{init_type_type, SoxObjectRef, SoxRef};

#[test]
fn test_drop_fn_called() {
    use std::sync::atomic::{AtomicBool, Ordering};

    static CALLED: AtomicBool = AtomicBool::new(false);

    #[derive(Clone)]
    struct Droppable;

    impl SoxObjectPayload for Droppable {
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
    }

    fn drop_droppable(_obj: &SoxObjectRef) {
        CALLED.store(true, Ordering::SeqCst);
    }

    let mut heap = Heap::new();

    // Create a type with our drop function
    let type_type = init_type_type();
    let droppable_type = SoxRef::new_ref(
        SoxType {
            base: None,
            methods: Default::default(),
            slots: SoxTypeSlot {
                drop: Some(drop_droppable),
                ..Default::default()
            },
            attributes: Default::default(),
            name: Some("Droppable".to_owned()),
        },
        type_type.clone(),
    );

    // Allocate an instance
    let _obj1 = heap.alloc(Droppable, droppable_type.clone());

    // Collect garbage - obj1 is not a root, so it should be collected
    // and its drop function called
    heap.collect(|_mark| {});

    assert!(CALLED.load(Ordering::SeqCst), "drop_fn was not called");
}

#[test]
fn test_string_drop() {
    // This test ensures that SoxString implements the drop slot.
    // We can't easily detect if String::drop was called without a custom allocator or Miri,
    // but we can check if the slot is populated.

    use sox::builtins::core::StaticType;
    let slots = SoxString::create_slots();
    assert!(slots.drop.is_some(), "SoxString must have a drop slot");

    // Also verify for other types that manage resources
    use sox::builtins::chunk::Chunk;
    use sox::builtins::closure::SoxUpvalue;
    use sox::builtins::function::{SoxFn, SoxFunction};
    use sox::builtins::module::SoxModule;
    use sox::builtins::r#type::SoxInstance;

    assert!(
        Chunk::create_slots().drop.is_some(),
        "Chunk must have a drop slot"
    );
    assert!(
        SoxUpvalue::create_slots().drop.is_some(),
        "SoxUpvalue must have a drop slot"
    );
    assert!(
        SoxFn::create_slots().drop.is_some(),
        "SoxFn must have a drop slot"
    );
    assert!(
        SoxFunction::create_slots().drop.is_some(),
        "SoxFunction must have a drop slot"
    );
    assert!(
        SoxModule::create_slots().drop.is_some(),
        "SoxModule must have a drop slot"
    );
    assert!(
        SoxInstance::create_slots().drop.is_some(),
        "SoxInstance must have a drop slot"
    );
}
