use crate::builtins::bool::SoxBool;
use crate::builtins::core::{SoxClassImpl, SoxResult, ToSoxResult, TryFromSoxObject};
use crate::builtins::core::{SoxObjectPayload, StaticType};
use crate::builtins::method::static_func;
use crate::builtins::method::SoxMethod;
use crate::builtins::r#type::{SoxType, SoxTypeSlot};
use crate::object::core::{Sox, SoxObjectRef, SoxRef};
use crate::object::protocols::comparable::{Comparable, ComparableMethods};
use crate::object::protocols::number::{AsNumber, NumberMethods};
use crate::object::protocols::repr::Representable;
use crate::runtime::Runtime;
use macros::{soxmethod, soxtype};
pub use once_cell::sync::{Lazy, OnceCell};
use std::any::Any;
use std::fmt;

//
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SoxString {
    pub value: String,
}

#[soxtype]
impl SoxString {
    pub fn new<T: Into<String>>(val: T) -> Self {
        SoxString { value: val.into() }
    }

    pub fn as_str(&self) -> &str {
        self.value.as_str()
    }

    #[soxmethod]
    pub fn bool(&self) -> SoxBool {
        SoxBool::new(self.value.as_str() != "")
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
            trace: None,
            drop: Some(Self::slot_drop),
            number: Some(Self::as_number()),
            comparable: Some(Self::as_comparable()),
            methods: Self::METHOD_DEFS,
        }
    }
}

impl SoxString {
    fn slot_drop(obj: &SoxObjectRef) {
        if obj.payload::<SoxString>().is_some() {
            unsafe {
                let inner = obj.ptr.as_ptr() as *mut crate::object::core::SoxObjectInner<SoxString>;
                std::ptr::drop_in_place(&mut (*inner).payload);
            }
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
    fn try_from_sox_object(i: &mut Runtime, obj: SoxObjectRef) -> SoxResult<Self> {
        if let Some(val) = obj.payload::<SoxString>() {
            Ok(val.clone())
        } else {
            let err_msg = SoxString {
                value: String::from("failed to get boolean from supplied object"),
            };
            let ob = i.alloc(err_msg, i.types.str_type.to_owned());
            Err(ob.into())
        }
    }
}

impl ToSoxResult for SoxString {
    fn to_sox_result(self, i: &mut Runtime) -> SoxResult {
        let obj = i.alloc(self, i.types.str_type.to_owned());
        Ok(SoxObjectRef::from(obj))
    }
}

impl fmt::Display for SoxString {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self.as_str(), f)
    }
}

impl Representable for SoxString {
    fn repr(zelf: &Sox<Self>, _i: &Runtime) -> String {
        zelf.value.to_string()
    }
}

impl AsNumber for SoxString {
    fn as_number() -> NumberMethods {
        NumberMethods {
            add: Some(|a, b, i| {
                Self::perform_operation(a, b, i, |mut a, b| {
                    a.push_str(&b);
                    a
                })
            }),

            ..Self::DEFAULT_NUMBER_METHODS
        }
    }
}

impl Comparable for SoxString {
    fn as_comparable() -> ComparableMethods {
        ComparableMethods {
            lt: Some(|a, b, i| Self::compare(a, b, i, |a, b| a < b)),
            gt: Some(|a, b, i| Self::compare(a, b, i, |a, b| a > b)),
            eq: Some(|a, b, i| Self::compare(a, b, i, |a, b| a == b)),
            ne: Some(|a, b, i| Self::compare(a, b, i, |a, b| a != b)),
            ge: Some(|a, b, i| Self::compare(a, b, i, |a, b| a >= b)),
            le: Some(|a, b, i| Self::compare(a, b, i, |a, b| a <= b)),
        }
    }
}

impl SoxString {
    fn perform_operation(
        a: SoxObjectRef,
        b: SoxObjectRef,
        i: &mut Runtime,
        op: fn(String, String) -> String,
    ) -> SoxResult {
        if let (Some(a), Some(b)) = (a.payload::<SoxString>(), b.payload::<SoxString>()) {
            let v = SoxString::new(op(a.value.clone(), b.value.clone()));
            v.to_sox_result(i)
        } else {
            Ok(i.runtime_error("Operands must be two numbers or two strings".to_string()))
        }
    }

    fn compare<F>(a: SoxObjectRef, other: SoxObjectRef, i: &mut Runtime, cmp_fn: F) -> SoxResult
    where
        F: FnOnce(&str, &str) -> bool,
    {
        if let (Some(a), Some(other)) = (a.payload::<SoxString>(), other.payload::<SoxString>()) {
            let result = cmp_fn(a.value.as_str(), other.value.as_str());
            SoxBool::new(result).to_sox_result(i)
        } else {
            SoxBool::new(false).to_sox_result(i)
        }
    }
}

#[cfg(test)]
mod tests {}
