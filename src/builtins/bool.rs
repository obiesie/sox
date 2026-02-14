use std::any::Any;

use crate::builtins::core::{
    SoxClassImpl, SoxObjectPayload, SoxResult, StaticType, ToSoxResult, TryFromSoxObject,
};
use crate::builtins::method::{static_func, SoxMethod};
use crate::builtins::r#type::{SoxType, SoxTypeSlot};
use crate::builtins::string::SoxString;
use crate::object::core::{Sox, SoxObjectRef, SoxRef};
use crate::object::protocols::comparable::{Comparable, ComparableMethods};
use crate::object::protocols::repr::Representable;
use crate::runtime::Runtime;
use macros::{soxmethod, soxtype};
use once_cell::sync::OnceCell;

#[derive(Debug, Clone, Copy)]
pub struct SoxBool {
    pub value: bool,
}

#[soxtype]
impl SoxBool {
    pub fn new(val: bool) -> Self {
        SoxBool { value: val }
    }

    pub fn not(&self) -> Self {
        SoxBool::new(!self.value)
    }

    #[soxmethod]
    pub fn bool(&self) -> Self {
        self.clone()
    }

    #[soxmethod]
    pub fn equals(&self, rhs: SoxObjectRef) -> Self {
        let other = rhs.payload::<SoxBool>();
        if let Some(other) = other {
            SoxBool::new(self.value == other.value)
        } else {
            SoxBool::new(false)
        }
    }
}

impl Representable for SoxBool {
    fn repr(zelf: &Sox<Self>, _i: &Runtime) -> String {
        zelf.value.to_string()
    }
}

impl TryFromSoxObject for SoxBool {
    fn try_from_sox_object(i: &mut Runtime, obj: SoxObjectRef) -> SoxResult<Self> {
        if let Some(bool_val) = obj.payload::<SoxBool>() {
            Ok(bool_val.clone())
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

impl ToSoxResult for SoxBool {
    fn to_sox_result(self, i: &mut Runtime) -> SoxResult {
        let obj = i.alloc(self, i.types.bool_type.to_owned());
        Ok(SoxObjectRef::from(obj))
    }
}

impl SoxObjectPayload for SoxBool {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl StaticType for SoxBool {
    const NAME: &'static str = "boolean";

    fn static_cell() -> &'static OnceCell<SoxRef<SoxType>> {
        static BOOL_CELL: OnceCell<SoxRef<SoxType>> = OnceCell::new();
        &BOOL_CELL
    }

    fn create_slots() -> SoxTypeSlot {
        SoxTypeSlot {
            call: None,
            repr: Some(Self::slot_repr),
            trace: None,
            drop: None,
            number: None,
            comparable: Some(Self::as_comparable()),
            methods: Self::METHOD_DEFS,
        }
    }
}

impl From<bool> for SoxBool {
    fn from(b: bool) -> Self {
        Self { value: b }
    }
}

impl Comparable for SoxBool {
    fn as_comparable() -> ComparableMethods {
        ComparableMethods {
            lt: None,
            gt: None,
            eq: Some(|a, b, i| SoxBool::new(Self::eq(a, b, i)).to_sox_result(i)),
            ne: Some(|a, b, i| SoxBool::new(!Self::eq(a, b, i)).to_sox_result(i)),
            ge: None,
            le: None,
        }
    }
}

impl SoxBool {
    fn eq(a: SoxObjectRef, other: SoxObjectRef, _i: &Runtime) -> bool {
        if let (Some(a), Some(other)) = (a.payload::<SoxBool>(), other.payload::<SoxBool>()) {
            let result = a.value == other.value;
            result
        } else {
            false
        }
    }
}
