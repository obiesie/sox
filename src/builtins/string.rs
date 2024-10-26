use std::any::Any;
use std::ops::Deref;
pub use once_cell::sync::{Lazy, OnceCell};
use macros::{soxmethod, soxtype};
use crate::builtins::bool::SoxBool;
use crate::builtins::method::{static_func, SoxMethod};
use crate::builtins::r#type::{SoxType, SoxTypeSlot};
use crate::core::{Representable, SoxClassImpl, SoxResult, ToSoxResult, TryFromSoxObject};
use crate::core::{SoxObject, SoxObjectPayload, SoxRef, StaticType};
use crate::interpreter::Interpreter;

//
#[derive(Clone, Debug)]
pub struct SoxString {
    pub value: String,
}

#[soxtype]
impl SoxString {
    pub fn new<T: Into<String>>(val: T) -> Self {
        SoxString { value: val.into() }
    }

    #[soxmethod]
    pub fn equals(&self, rhs: SoxObject) -> SoxBool {
        match rhs.as_string() {
            Some(other) => SoxBool::new(self.value == other.value),
            None => SoxBool::new(false),
        }
    }
}

// impl SoxClassImpl for SoxString {
//     const METHOD_DEFS: &'static [(&'static str, SoxMethod)] = &[  (
//         "equals",
//         SoxMethod {
//             func: static_func(SoxString::equals),
//         },
//     )];
// }
impl StaticType for SoxString {
    const NAME: &'static str = "string";

    fn static_cell() -> &'static OnceCell<SoxType> {
        static CELL: OnceCell<SoxType> = OnceCell::new();
        &CELL
    }

    fn create_slots() -> SoxTypeSlot {
        SoxTypeSlot { 
            call: None,
            methods: Self::METHOD_DEFS,
            
        }
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
        i.types.str_type
    }
}

impl From<String> for SoxString {
    fn from(s: String) -> Self {
        let val = Self { value: s };
        val
    }
}

impl TryFromSoxObject for SoxString {
    fn try_from_sox_object(_i: &Interpreter, obj: SoxObject) -> SoxResult<Self> {
        if let Some(val) = obj.as_string() {
            Ok(val.val.deref().clone())
        } else {
            let err_msg = SoxString {
                value: String::from("failed to get boolean from supplied object"),
            };
            let ob = SoxRef::new(err_msg);
            Err(SoxObject::String(ob))
        }
    }
}

impl ToSoxResult for SoxString {
    fn to_sox_result(self, _i: &Interpreter) -> SoxResult {
        let obj = self.into_ref();
        Ok(obj)
    }
}



impl Representable for SoxString {
    fn repr(&self, _i: &Interpreter) -> String {
        self.value.to_string()
    }
}
#[cfg(test)]
mod tests {}
