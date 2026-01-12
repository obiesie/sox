use crate::builtins::bool::SoxBool;
use crate::builtins::core::{
    SoxClassImpl, SoxObjectPayload, SoxResult, StaticType, ToSoxResult, TryFromSoxObject,
};
use crate::builtins::method::{static_func, SoxMethod};
use crate::builtins::r#type::{SoxType, SoxTypeSlot};
use crate::builtins::string::SoxString;
use crate::interpreter::Interpreter;
use crate::object::core::{Sox, SoxObjectRef, SoxRef};
use crate::object::protocols::comparable::{Comparable, ComparableMethods};
use crate::object::protocols::repr::Representable;
use macros::{soxmethod, soxtype};
use once_cell::sync::OnceCell;
use std::any::Any;

#[derive(Debug, Clone, Copy, Eq, PartialOrd, PartialEq)]
pub struct SoxNone;

#[soxtype]
impl SoxNone {
    #[soxmethod]
    pub fn bool(&self) -> SoxBool {
        SoxBool::new(false)
    }
}

impl SoxObjectPayload for SoxNone {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl StaticType for SoxNone {
    const NAME: &'static str = "none";

    fn static_cell() -> &'static OnceCell<SoxRef<SoxType>> {
        static CELL: OnceCell<SoxRef<SoxType>> = OnceCell::new();
        &CELL
    }

    fn create_slots() -> SoxTypeSlot {
        SoxTypeSlot {
            call: None,
            repr: Some(Self::slot_repr),
            number: None,
            comparable: None,
            methods: Self::METHOD_DEFS,
        }
    }
}

impl TryFromSoxObject for SoxNone {
    fn try_from_sox_object(i: &Interpreter, obj: SoxObjectRef) -> SoxResult<Self> {
        if let Some(val) = obj.payload::<SoxNone>() {
            Ok(val.clone())
        } else {
            let err_msg = SoxString {
                value: String::from("failed to get boolean from supplied object"),
            };
            let ob = SoxRef::new_ref(err_msg, i.types.none_type.to_owned());
            Err(ob.into())
        }
    }
}

impl ToSoxResult for SoxNone {
    fn to_sox_result(self, i: &Interpreter) -> SoxResult {
        let obj = SoxRef::new_ref(self, i.types.none_type.to_owned());
        Ok(obj.into())
    }
}

impl Comparable for SoxNone {
    fn as_comparable() -> ComparableMethods {
        ComparableMethods {
            lt: None,
            gt: None,
            eq: Some(|a, b, i| Self::compare(a, b, i, |a, b| a == b)),
            ne: None,
            ge: None,
            le: None,
        }
    }
}

impl Representable for SoxNone {
    fn repr(_zelf: &Sox<Self>, _i: &Interpreter) -> String {
        "None".to_string()
    }
}

impl SoxNone {
    fn compare<F>(a: SoxObjectRef, other: SoxObjectRef, i: &Interpreter, cmp_fn: F) -> SoxResult
    where
        F: FnOnce(&SoxNone, &SoxNone) -> bool,
    {
        if let (Some(a), Some(other)) = (a.payload::<SoxNone>(), other.payload::<SoxNone>()) {
            let result = cmp_fn(a, other);
            SoxBool::new(result).to_sox_result(i)
        } else {
            SoxBool::new(false).to_sox_result(i)
        }
    }
}
