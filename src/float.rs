use std::any::Any;
use std::rc::Rc;
use crate::core::{SoxObject, SoxObjectPayload, SoxObjectRef, SoxType};

#[derive(Debug)]
pub struct SoxFloat {
    value: f64,
}

impl SoxFloat {
    pub fn new(val: f64) -> Self {
        SoxFloat {
            value: val,
        }
    }
}

impl SoxObjectPayload for SoxFloat {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn into_sox_object(self) -> SoxObjectRef {
        let sox_object = SoxObject::new(self);
        let sox_object_ref = Rc::new(sox_object);
        return sox_object_ref
    }
}

