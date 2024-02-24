use crate::core::SoxObject;
use std::ops::{Add, Deref};
use std::rc::Rc;

// pub type SoxStringRef = Rc<SoxString>;
//
// #[derive(Clone, Debug)]
// pub struct SoxString {
//     pub value: String,
// }
//
// impl SoxString {
//     pub fn new<T: Into<String>>(val: T) -> Self {
//         SoxString {
//             value: val.into(),
//         }
//     }
//
//     pub fn into_ref(self) -> SoxStringRef {
//         return Rc::new(self);
//     }
//
//     pub fn into_sox_obj(self) -> SoxObject {
//         return SoxObject::String(self.into_ref());
//     }
// }
//
// impl Add for SoxString{
//     type Output = SoxString;
//
//     fn add(self, rhs: Self) -> Self::Output {
//         return SoxString{ value: self.value + rhs.value.as_str() };
//     }
// }
