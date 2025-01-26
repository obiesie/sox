use once_cell::sync::OnceCell;
use std::any::Any;
use polars::export::num::Zero;
use macros::{soxmethod, soxtype};
use crate::builtins::bool::SoxBool;
use crate::builtins::method::{static_func, SoxMethod};
use crate::builtins::r#type::{SoxType, SoxTypeSlot};
use crate::builtins::string::SoxString;
use crate::builtins::core::{SoxClassImpl, SoxObjectPayload, SoxResult, StaticType, ToSoxResult, TryFromSoxObject};
use crate::builtins::int::SoxInt;
use crate::interpreter::Interpreter;
use crate::object::core::{Sox, SoxObjectRef, SoxRef};
use crate::object::protocols::comparable::{Comparable, ComparableMethods};
use crate::object::protocols::number::{AsNumber, NumberMethods};
use crate::object::protocols::repr::Representable;

#[derive(Debug, Clone, Copy)]
pub struct SoxFloat {
    pub value: f64,
}

#[soxtype]
impl SoxFloat {
    pub fn new(val: f64) -> Self {
        SoxFloat { value: val }
    }

    #[soxmethod]
    pub fn bool(&self) -> SoxBool {
        SoxBool::new(!self.value.is_zero())
    }
}


impl SoxObjectPayload for SoxFloat {
    
    fn as_any(&self) -> &dyn Any {
        self
    }

   
}

impl StaticType for SoxFloat {
    const NAME: &'static str = "float";

    fn static_cell() -> &'static OnceCell<SoxRef<SoxType>> {
        static CELL: OnceCell<SoxRef<SoxType>> = OnceCell::new();
        &CELL
    }

    fn create_slots() -> SoxTypeSlot {
        SoxTypeSlot {
            call: None,
            repr: Some(Self::slot_repr),
            number: Some(Self::as_number()),
            comparable: None,
            methods: Self::METHOD_DEFS,
        }
    }
}

impl TryFromSoxObject for SoxFloat {
    fn try_from_sox_object(_i: &Interpreter, obj: SoxObjectRef) -> SoxResult<Self> {
        if let Some(val) = obj.payload::<SoxFloat>() {
            Ok(val.clone())
        } else {
            let err_msg = SoxString {
                value: String::from("failed to get boolean from supplied object"),
            };
            let ob = SoxRef::new_ref(err_msg, _i.types.float_type.to_owned());
            Err(ob.into())
        }
    }
}

impl ToSoxResult for SoxFloat {
    fn to_sox_result(self, _i: &Interpreter) -> SoxResult {
        let obj = SoxRef::new_ref(self, _i.types.float_type.to_owned());
        Ok(obj.into())
    }
}


impl From<f64> for SoxFloat {
    fn from(f: f64) -> Self {
        Self { value: f }
    }
}

impl Representable for SoxFloat {
    fn repr(zelf: &Sox<Self>, _i: &Interpreter) -> String {
        zelf.value.to_string()
    }
}

impl AsNumber for SoxFloat {
    fn as_number() -> NumberMethods {
        NumberMethods {
            add: Some(|a, b, i| Self::perform_operation(a, b, i, |a, b| a + b)),
            minus: Some(|a, b, i| Self::perform_operation(a, b, i, |a, b| a - b)),
            star: Some(|a, b, i| Self::perform_operation(a, b, i, |a, b| a * b)),
            slash: Some(|a, b, i| Self::perform_operation(a, b, i, |a, b| a / b)),
            rem: Some(|a, b, i| Self::perform_operation(a, b, i, |a, b| a % b)),
            neg: Some(|a, i: &Interpreter| SoxFloat::new(-a.payload::<SoxFloat>().unwrap().value).to_sox_result(i)),

        }
    }
}

impl Comparable for SoxFloat {
    fn as_comparable() -> ComparableMethods {
        ComparableMethods {
            lt: Some(|a, b, i| Self::compare(a, b, i, |a, b| a < b)),
            gt: Some(|a, b, i| Self::compare(a, b, i, |a, b| a > b)),
            eq: None,
            ne: Some(|a, b, i| Self::compare(a, b, i, |a, b| a != b)),
            ge: Some(|a, b, i| Self::compare(a, b, i, |a, b| a >= b)),
            le: Some(|a, b, i| Self::compare(a, b, i, |a, b| a <= b)),
        }
    }
}


impl SoxFloat{
    fn perform_operation(
        a: SoxObjectRef,
        b: SoxObjectRef,
        i: &Interpreter,
        op: fn(f64, f64) -> f64,
    ) -> SoxResult {
        if let (Some(a), Some(b)) = (a.payload::<SoxFloat>(), b.payload::<SoxFloat>()) {
            let v = SoxFloat::new(op(a.value, b.value));
            v.to_sox_result(i)
        } else {
            Ok(i.runtime_error("Operands must be two numbers or two strings".into()))
        }
    }

    fn compare<F>(a: SoxObjectRef, other: SoxObjectRef, i: &Interpreter, cmp_fn: F) -> SoxResult
    where
        F: FnOnce(i64, i64) -> bool,
    {
        if let (Some(a), Some(other_int)) = (a.payload::<SoxInt>(), other.payload::<SoxInt>()) {
            let result = cmp_fn(a.value, other_int.value);
            SoxBool::new(result).to_sox_result(i)
        } else {
            SoxBool::new(false).to_sox_result(i)
        }
    }
    
   
}