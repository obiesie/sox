use std::rc::Rc;
use crate::core::SoxObject;

pub type SoxIntRef = Rc<SoxInt>;

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

    pub fn into_ref(self) -> SoxIntRef {
        return Rc::new(self);
    }

    pub fn into_sox_obj(self) -> SoxObject {
        return SoxObject::Int(self.into_ref());
    }
}

