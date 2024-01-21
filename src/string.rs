use std::rc::Rc;
use crate::core::SoxObject;

pub type SoxStringRef = Rc<SoxString>;

#[derive(Debug)]
pub struct SoxString {
    value: String,
}

impl SoxString {
    pub fn new(val: String) -> Self {
        SoxString {
            value: val,
        }
    }

    pub fn into_ref(self) -> SoxStringRef {
        return Rc::new(self);
    }

    pub fn into_sox_obj(self) -> SoxObject {
        return SoxObject::String(self.into_ref());
    }
}