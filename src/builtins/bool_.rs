use std::any::Any;
use std::ops::Deref;

use once_cell::sync::OnceCell;

use macros::{soxmethod, soxtype};

use crate::builtins::method::{static_func, SoxMethod};
use crate::builtins::string::SoxString;
use crate::core::{
    SoxClassImpl, SoxObject, SoxObjectPayload, SoxRef, SoxResult, SoxType, SoxTypeSlot, StaticType,
    ToSoxResult, TryFromSoxObject,
};
use crate::interpreter::Interpreter;

#[soxtype]
#[derive(Debug, Clone, Copy)]
pub struct SoxBool {
    pub value: bool,
}

#[soxtype]
impl SoxBool {
    pub fn new(val: bool) -> Self {
        SoxBool { value: val }
    }

    #[soxmethod]
    pub fn bool_(&self) -> Self {
        self.clone()
    }
}

impl TryFromSoxObject for SoxBool {
    fn try_from_sox_object(i: &Interpreter, obj: SoxObject) -> SoxResult<Self> {
        if let Some(bool_val) = obj.as_bool() {
            Ok(bool_val.val.deref().clone())
        } else {
            let err_msg = SoxString {
                value: String::from("failed to get boolean from supplied object"),
            };
            let ob = SoxRef::new(err_msg);
            Err(SoxObject::String(ob))
        }
    }
}

impl ToSoxResult for SoxBool {
    fn to_sox_result(self, i: &Interpreter) -> SoxResult {
        let obj = self.into_ref();
        Ok(obj)
    }
}

impl SoxObjectPayload for SoxBool {
    fn to_sox_type_value(obj: SoxObject) -> SoxRef<Self> {
        obj.as_bool().unwrap()
    }

    fn to_sox_object(&self, ref_type: SoxRef<Self>) -> SoxObject {
        SoxObject::Boolean(ref_type)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn into_ref(self) -> SoxObject {
        SoxRef::new(self).to_sox_object()
    }

    fn class(&self, i: &Interpreter) -> &'static SoxType {
        i.types.bool_type
    }
}

impl StaticType for SoxBool {
    const NAME: &'static str = "bool";

    fn static_cell() -> &'static OnceCell<SoxType> {
        static CELL: OnceCell<SoxType> = OnceCell::new();
        &CELL
    }

    fn create_slots() -> SoxTypeSlot {
        SoxTypeSlot { call: None }
    }
}

impl From<bool> for SoxBool {
    fn from(b: bool) -> Self {
        Self { value: b }
    }
}
