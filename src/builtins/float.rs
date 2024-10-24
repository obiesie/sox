use std::any::Any;

use once_cell::sync::OnceCell;

use macros::soxtype;
use crate::builtins::bool_::SoxBool;
use crate::builtins::method::{static_func, SoxMethod};
use crate::builtins::r#type::{SoxType, SoxTypeSlot};
use crate::core::{Representable, SoxClassImpl, SoxObject, SoxObjectPayload, SoxRef, StaticType};
use crate::interpreter::Interpreter;

#[soxtype]
#[derive(Debug, Clone, Copy)]
pub struct SoxFloat {
    pub value: f64,
}

impl SoxFloat {
    pub fn new(val: f64) -> Self {
        SoxFloat { value: val }
    }

    pub fn equals(&self, other: &SoxObject) -> SoxBool {
        if let Some(other_float) = other.as_float() {
           SoxBool::from(other_float.value == self.value) 
        } else {
            SoxBool::from(false)
        }
    }
    
    
}

impl SoxClassImpl for SoxFloat {
    const METHOD_DEFS: &'static [(&'static str, SoxMethod)] = &[  (
        "equals",
        SoxMethod {
            func: static_func(SoxBool::equals),
        },
    )];
}

impl SoxObjectPayload for SoxFloat {
    fn to_sox_type_value(obj: SoxObject) -> SoxRef<Self> {
        obj.as_float().unwrap()
    }

    fn to_sox_object(&self, ref_type: SoxRef<Self>) -> SoxObject {
        SoxObject::Float(ref_type)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn into_ref(self) -> SoxObject {
        SoxRef::new(self).to_sox_object()
    }

    fn class(&self, i: &Interpreter) -> &'static SoxType {
        i.types.float_type
    }
}

impl StaticType for SoxFloat {
    const NAME: &'static str = "float";

    fn static_cell() -> &'static OnceCell<SoxType> {
        static CELL: OnceCell<SoxType> = OnceCell::new();
        &CELL
    }

    fn create_slots() -> SoxTypeSlot {
        SoxTypeSlot { call: None,             methods: Self::METHOD_DEFS,
        }
    }
}

impl From<f64> for SoxFloat {
    fn from(f: f64) -> Self {
        Self { value: f }
    }
}

impl Representable for SoxFloat {
    fn repr(&self, i: &Interpreter) -> String {
        self.value.to_string()
    }
}
