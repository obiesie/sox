use std::any::Any;

use once_cell::sync::OnceCell;

use macros::soxtype;

use crate::core::{SoxClassImpl, SoxObject, SoxObjectPayload, SoxRef, SoxType, SoxTypeSlot, StaticType};
use crate::interpreter::Interpreter;
use crate::builtins::method::SoxMethod;

#[soxtype]
#[derive(Debug, Clone, Copy)]
pub struct SoxFloat {
    pub value: f64,
}

#[soxtype]
impl SoxFloat {
    pub fn new(val: f64) -> Self {
        SoxFloat {
            value: val,
        }
    }
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
        SoxTypeSlot{
            call: None
        }
    }
}


impl From<f64> for SoxFloat {
    fn from(f: f64) -> Self {
        Self {
            value: f
        }
    }
}
