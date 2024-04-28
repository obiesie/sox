use std::any::Any;
use std::rc::Rc;

use once_cell::sync::OnceCell;

use macros::soxtype;

use crate::core::{SoxClassImpl, SoxObject, SoxObjectPayload, SoxRef, SoxResult, SoxType, SoxTypeSlot, StaticType};
use crate::interpreter::Interpreter;
use crate::method::SoxMethod;

pub type SoxIntRef = Rc<SoxInt>;

#[soxtype]
#[derive(Debug, Clone, Copy)]
pub struct SoxInt {
    pub value: i64,
}

#[soxtype]
impl SoxInt {
    pub fn new(val: i64) -> Self {
        SoxInt {
            value: val,
        }
    }

   
}

impl SoxObjectPayload for SoxInt{
   
    fn to_sox_type_value(obj: SoxObject) -> SoxRef<Self> {
       obj.as_int().unwrap() 
    }

    fn to_sox_object(&self, ref_type: SoxRef<Self>) -> SoxObject {
        SoxObject::Int(ref_type)
        
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn into_ref(self) -> SoxObject {
        SoxRef::new(self).to_sox_object()
    }

   


    fn class(&self, i: &Interpreter) -> &'static SoxType {
        todo!()
    }
}


impl StaticType for SoxInt {
    const NAME: &'static str = "int";

    fn static_cell() -> &'static OnceCell<SoxType> {
        static CELL: OnceCell<SoxType> = OnceCell::new();
        &CELL
    }

    fn create_slots() -> SoxTypeSlot {
        todo!()
    }
}


impl From<i64> for SoxInt {
    fn from(i: i64) -> Self {
        Self {
            value: i
        }
    }
}