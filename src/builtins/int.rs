use std::any::Any;
use std::io::Repeat;
use std::ops::Deref;
use std::rc::Rc;

use once_cell::sync::OnceCell;

use macros::soxtype;
use crate::builtins::bool_::SoxBool;
use crate::builtins::float::SoxFloat;
use crate::builtins::method::{static_func, SoxMethod};
use crate::builtins::r#type::{SoxType, SoxTypeSlot};
use crate::builtins::string::SoxString;
use crate::core::{Representable, SoxClassImpl, SoxObject, SoxObjectPayload, SoxRef, SoxResult, StaticType, ToSoxResult, TryFromSoxObject};
use crate::interpreter::Interpreter;

pub type SoxIntRef = Rc<SoxInt>;

#[soxtype]
#[derive(Debug, Clone, Copy)]
pub struct SoxInt {
    pub value: i64,
}

impl SoxInt {
    pub fn new(val: i64) -> Self {
        SoxInt { value: val }
    }

    pub fn equals(&self, rhs: SoxObject) -> SoxBool {
        if let Some(rhs_int) = rhs.as_int() {
            SoxBool::new(self.value == rhs_int.value)
        } else {
            SoxBool::new(false)
        }
    }
       
}

impl SoxClassImpl for SoxInt {
    const METHOD_DEFS: &'static [(&'static str, SoxMethod)] = &[  (
        "equals",
        SoxMethod {
            func: static_func(SoxInt::equals),
        },
    )];
}

impl SoxObjectPayload for SoxInt {
    fn to_sox_type_value(obj: SoxObject) -> SoxRef<Self> {
        obj.as_int().unwrap()
    }

    fn to_sox_object(&self, ref_type: SoxRef<Self>) -> SoxObject {
        SoxObject::Int(ref_type)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn into_ref(self) -> SoxObject {
        SoxRef::new(self).to_sox_object()
    }

    fn class(&self, i: &Interpreter) -> &'static SoxType {
        i.types.int_type
    }
}

impl StaticType for SoxInt {
    const NAME: &'static str = "int";

    fn static_cell() -> &'static OnceCell<SoxType> {
        static CELL: OnceCell<SoxType> = OnceCell::new();
        &CELL
    }

    fn create_slots() -> SoxTypeSlot {
        SoxTypeSlot { call: None,             methods: Self::METHOD_DEFS,
        }
    }
}


impl TryFromSoxObject for SoxInt {
    fn try_from_sox_object(_i: &Interpreter, obj: SoxObject) -> SoxResult<Self> {
        if let Some(val) = obj.as_int() {
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

impl ToSoxResult for SoxInt {
    fn to_sox_result(self, _i: &Interpreter) -> SoxResult {
        let obj = self.into_ref();
        Ok(obj)
    }
}


impl From<i64> for SoxInt {
    fn from(i: i64) -> Self {
        Self { value: i }
    }
}

impl Representable for SoxInt {
    fn repr(&self, i: &Interpreter) -> String {
        self.value.to_string()
    }
}
