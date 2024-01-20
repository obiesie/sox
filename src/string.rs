use std::rc::Rc;
use crate::core::SoxObj;

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

    pub fn into_sox_obj(self) -> SoxObj {
        return SoxObj::String(self.into_ref());
    }
}