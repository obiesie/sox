use crate::builtins::bool::SoxBool;
use crate::builtins::core::{
    SoxClassImpl, SoxObjectPayload, SoxResult, StaticType, ToSoxResult, TryFromSoxObject,
};
use crate::builtins::method::static_func;
use crate::builtins::method::SoxMethod;
use crate::builtins::r#type::{SoxType, SoxTypeSlot};
use crate::builtins::string::SoxString;
use crate::object::core::{Sox, SoxObjectRef, SoxRef};
use crate::object::protocols::comparable::{Comparable, ComparableMethods};
use crate::object::protocols::number::{AsNumber, NumberMethods};
use crate::object::protocols::repr::Representable;
use crate::runtime::Runtime;
use macros::{soxmethod, soxtype};
use once_cell::sync::OnceCell;
use polars::export::num::Zero;
use std::any::Any;

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
            trace: None,
            drop: None,
            number: Some(Self::as_number()),
            comparable: None,
            methods: Self::METHOD_DEFS,
        }
    }
}

impl TryFromSoxObject for SoxFloat {
    fn try_from_sox_object(i: &mut Runtime, obj: SoxObjectRef) -> SoxResult<Self> {
        if let Some(val) = obj.payload::<SoxFloat>() {
            Ok(val.clone())
        } else {
            let err_msg = SoxString {
                value: String::from("failed to get boolean from supplied object"),
            };
            let ob = i.alloc(
                err_msg,
                crate::builtins::string::SoxString::init_builtin_type().to_owned(),
            );
            Err(ob.into())
        }
    }
}

impl ToSoxResult for SoxFloat {
    fn to_sox_result(self, _i: &mut Runtime) -> SoxResult {
        let obj = _i.alloc(self, _i.types.float_type.to_owned());
        Ok(SoxObjectRef::from(obj))
    }
}

impl From<f64> for SoxFloat {
    fn from(f: f64) -> Self {
        Self { value: f }
    }
}

impl Representable for SoxFloat {
    fn repr(zelf: &Sox<Self>, _i: &Runtime) -> String {
        zelf.value.to_string()
    }
}

impl AsNumber for SoxFloat {
    fn as_number() -> NumberMethods {
        NumberMethods {
            add: Some(|a, b, i| Self::binary_op(a, b, i, |a, b| a + b)),
            minus: Some(|a, b, i| Self::binary_op(a, b, i, |a, b| a - b)),
            star: Some(|a, b, i| Self::binary_op(a, b, i, |a, b| a * b)),
            slash: Some(|a, b, i| Self::binary_op(a, b, i, |a, b| a / b)),
            rem: Some(|a, b, i| Self::binary_op(a, b, i, |a, b| a % b)),
            neg: Some(|a, i: &mut Runtime| {
                if let Some(val) = a.payload::<SoxFloat>() {
                    SoxFloat::new(-val.value).to_sox_result(i)
                } else {
                    unreachable!()
                }
            }),
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

impl SoxFloat {
    pub(crate) fn binary_op<F>(
        a: SoxObjectRef,
        b: SoxObjectRef,
        i: &mut Runtime,
        op: F,
    ) -> SoxResult
    where
        F: FnOnce(f64, f64) -> f64,
    {
        if let (Some(a), Some(b)) = (a.payload::<SoxFloat>(), b.payload::<SoxFloat>()) {
            let v = SoxFloat::new(op(a.value, b.value));
            v.to_sox_result(i)
        } else {
            Ok(i.runtime_error("Operands must be two numbers or two strings".to_string()))
        }
    }

    fn compare<F>(a: SoxObjectRef, other: SoxObjectRef, i: &mut Runtime, cmp_fn: F) -> SoxResult
    where
        F: FnOnce(f64, f64) -> bool,
    {
        if let (Some(a), Some(other_float)) = (a.payload::<SoxFloat>(), other.payload::<SoxFloat>())
        {
            let result = cmp_fn(a.value, other_float.value);
            SoxBool::new(result).to_sox_result(i)
        } else {
            SoxBool::new(false).to_sox_result(i)
        }
    }
}
