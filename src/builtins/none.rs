use std::any::Any;
use std::sync::OnceLock;

use crate::builtins::method::SoxMethod;
use crate::builtins::r#type::{SoxType, SoxTypeSlot};
use crate::core::{SoxClassImpl, SoxObject, SoxObjectPayload, SoxRef, StaticType};
use crate::interpreter::Interpreter;

#[derive(Debug, Clone, Copy)]
pub struct SoxNone;

impl SoxNone {
    pub fn bool(&self) -> bool {
        false
    }
}

impl SoxClassImpl for SoxNone {
    const METHOD_DEFS: &'static [(&'static str, SoxMethod)] = &[];

    fn static_cell() -> &'static OnceLock<SoxType> {
        static CELL: OnceLock<SoxType> = OnceLock::new();
        &CELL
    }
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

    
    
    fn create_slots() -> SoxTypeSlot {
        SoxTypeSlot { call: None }
    }
}
