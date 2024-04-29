use std::any::Any;

use once_cell::sync::OnceCell;

use macros::soxtype;

use crate::builtins::method::SoxMethod;
use crate::core::{SoxClassImpl, SoxObject, SoxObjectPayload, SoxRef, SoxType, SoxTypeSlot, StaticType};
use crate::interpreter::Interpreter;

#[soxtype]
#[derive(Debug, Clone)]
pub struct SoxNone;


#[soxtype]
impl SoxNone{
    pub fn bool(&self) -> bool{
        false
    }
}


impl SoxObjectPayload for SoxNone {

    fn to_sox_type_value(obj: SoxObject) -> SoxRef<Self> {
        obj.as_none().unwrap()
    }

    fn to_sox_object(&self, ref_type: SoxRef<Self>) -> SoxObject {
        SoxObject::None
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn into_ref(self) -> SoxObject {
        SoxObject::None
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
        SoxTypeSlot{
            call: None
        }
    }
}
