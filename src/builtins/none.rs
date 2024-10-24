use std::any::Any;

use once_cell::sync::OnceCell;
use crate::builtins::bool_::SoxBool;
use crate::builtins::method::{static_func, SoxMethod};
use crate::builtins::r#type::{SoxType, SoxTypeSlot};
use crate::core::{Representable, SoxClassImpl, SoxObject, SoxObjectPayload, SoxRef, StaticType};
use crate::interpreter::Interpreter;

#[derive(Debug, Clone, Copy)]
pub struct SoxNone;

impl SoxNone {
    pub fn bool(&self) -> bool {
        false
    }

    pub fn equals(&self, rhs: SoxObject) -> SoxBool {
        match rhs.as_none() {
            Some(_) => SoxBool::new(true),
            None => SoxBool::new(false),
        }
    }
}

impl SoxClassImpl for SoxNone {
    const METHOD_DEFS: &'static [(&'static str, SoxMethod)] = &[  (
        "equals",
        SoxMethod {
            func: static_func(SoxBool::equals),
        },
    )];
}
impl SoxObjectPayload for SoxNone {
    fn to_sox_type_value(obj: SoxObject) -> SoxRef<Self> {
        obj.as_none().unwrap()
    }

    fn to_sox_object(&self, ref_type: SoxRef<Self>) -> SoxObject {
        SoxObject::None(ref_type)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn into_ref(self) -> SoxObject {
        SoxRef::new(self).to_sox_object()
    }

    fn class(&self, i: &Interpreter) -> &'static SoxType {
        i.types.none_type
    }
}

impl StaticType for SoxNone {
    const NAME: &'static str = "none";

    fn static_cell() -> &'static OnceCell<SoxType> {
        static CELL: OnceCell<SoxType> = OnceCell::new();
        &CELL
    }

    fn create_slots() -> SoxTypeSlot {
        SoxTypeSlot { call: None,             methods: Self::METHOD_DEFS,
        }
    }
}

impl Representable for SoxNone {
    fn repr(&self, i: &Interpreter) -> String {
        "None".to_string()
    }
}
