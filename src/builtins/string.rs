use std::any::Any;
use std::fmt;
pub use once_cell::sync::{Lazy, OnceCell};
use macros::{soxmethod, soxtype};
use crate::builtins::bool::SoxBool;
use crate::builtins::method::{static_func, SoxMethod};
use crate::builtins::r#type::{SoxType, SoxTypeSlot};
use crate::builtins::core::{SoxClassImpl, SoxResult, ToSoxResult, TryFromSoxObject};
use crate::builtins::core::{SoxObjectPayload, StaticType};
use crate::interpreter::Interpreter;
use crate::object::core::{Sox, SoxObjectRef, SoxRef};
use crate::slots::repr::Representable;

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
    pub fn equals(&self, rhs: SoxObjectRef) -> SoxBool {
        match rhs.payload::<SoxString>() {
            Some(other) => SoxBool::new(self.value == other.value),
            None => SoxBool::new(false),
        }
    }
    
    pub fn as_str(&self) -> &str {
        self.value.as_str()
    }
}


impl StaticType for SoxString {
    const NAME: &'static str = "string";

    fn static_cell() -> &'static OnceCell<SoxRef<SoxType>> {
        static CELL: OnceCell<SoxRef<SoxType>> = OnceCell::new();
        &CELL
    }

    fn create_slots() -> SoxTypeSlot {
        SoxTypeSlot { 
            call: None,
            repr: Some(Self::slot_repr),
            methods: Self::METHOD_DEFS,
            
        }
    }
}

impl SoxObjectPayload for SoxString {

    fn as_any(&self) -> &dyn Any {
        self
    }

}

impl From<String> for SoxString {
    fn from(s: String) -> Self {
        let val = Self { value: s };
        val
    }
}

impl TryFromSoxObject for SoxString {
    fn try_from_sox_object(_i: &Interpreter, obj: SoxObjectRef) -> SoxResult<Self> {
        if let Some(val) = obj.payload::<SoxString>() {
            Ok(val.clone())
        } else {
            let err_msg = SoxString {
                value: String::from("failed to get boolean from supplied object"),
            };
            let ob = SoxRef::new_ref(err_msg, _i.types.str_type.to_owned());
            Err(ob.into())
        }
    }
}

impl ToSoxResult for SoxString {
    fn to_sox_result(self, _i: &Interpreter) -> SoxResult {
        let obj = SoxRef::new_ref(self, _i.types.str_type.to_owned());
        Ok(obj.into())
    }
}


impl fmt::Display for SoxString {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self.as_str(), f)
    }
}


impl Representable for SoxString {
    fn repr(zelf: &Sox<Self>, _i: &Interpreter) -> String {
        zelf.value.to_string()
    }
}
#[cfg(test)]
mod tests {}
