use std::any::Any;
use std::rc::Rc;
use crate::core::{SoxObject, SoxObjectPayload, SoxObjectRef};

#[derive(Debug)]
pub struct SoxInt {
    value: i64,
}

impl SoxInt {
    pub fn new(val: i64) -> Self {
        SoxInt {
            value: val,
        }
    }
}

impl SoxObjectPayload for SoxInt {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn into_sox_object(self) -> SoxObjectRef {
        let sox_object = SoxObject::new(self);
        let sox_object_ref = Rc::new(sox_object);
        return sox_object_ref
    }
}

