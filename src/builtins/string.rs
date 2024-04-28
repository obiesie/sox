use std::any::{Any, TypeId};
use std::ops::{Add, Deref};
use std::ptr::NonNull;
use std::rc::Rc;

pub use once_cell::sync::{Lazy, OnceCell};

use macros::{soxmethod, soxtype};

use crate::bool_::SoxBool;
use crate::core::{SoxObject, SoxObjectPayload, SoxRef, SoxResult, SoxType, SoxTypeSlot, StaticType, ToSoxResult, TryFromSoxObject};
use crate::core::SoxClassImpl;
use crate::interpreter::Interpreter;
use crate::method::{SoxMethod, static_func};

pub type SoxStringRef = Rc<SoxString>;

//
#[soxtype]
#[derive(Clone, Debug)]
pub struct SoxString {
    pub value: String,

}

#[soxtype]
impl SoxString {
    pub fn new<T: Into<String>>(val: T) -> Self {
        SoxString {
            value: val.into(),
        }
    }
}


impl StaticType for SoxString {
    const NAME: &'static str = "string";

    fn static_cell() -> &'static OnceCell<SoxType> {
        static CELL: OnceCell<SoxType> = OnceCell::new();
        &CELL
    }

    fn create_slots() -> SoxTypeSlot {
        todo!()
    }
}

impl SoxObjectPayload for SoxString {
   

    fn to_sox_type_value(obj: SoxObject) -> SoxRef<Self> {
        obj.as_string().unwrap()
    }

    fn to_sox_object(&self, ref_type: SoxRef<Self>) -> SoxObject {
        SoxObject::String(ref_type)
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


impl From<String> for SoxString {
    fn from(s: String) -> Self {
        let val = Self {
            value: s
        };
        val
    }
}


#[cfg(test)]
mod tests {}
