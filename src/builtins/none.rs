use std::any::Any;
use std::ops::Deref;
use crate::builtins::bool_::SoxBool;
use crate::builtins::method::{static_func, SoxMethod};
use crate::builtins::r#type::{SoxType, SoxTypeSlot};
use crate::core::{Representable, SoxClassImpl, SoxObject, SoxObjectPayload, SoxRef, SoxResult, StaticType, ToSoxResult, TryFromSoxObject};
use crate::interpreter::Interpreter;
use once_cell::sync::OnceCell;
use crate::builtins::string::SoxString;

#[derive(Debug, Clone, Copy)]
pub struct SoxNone;

impl SoxNone {
    pub fn bool(&self) -> SoxBool {
        SoxBool::new(false)
    }

    pub fn equals(&self, rhs: SoxObject) -> SoxBool {
        match rhs.as_none() {
            Some(_) => SoxBool::new(true),
            None => SoxBool::new(false),
        }
    }
}

impl SoxClassImpl for SoxNone {
    const METHOD_DEFS: &'static [(&'static str, SoxMethod)] = &[
        (
            "bool",
            SoxMethod {
                func: static_func(SoxNone::bool),
            },
        ),
        (
            "equals",
            SoxMethod {
                func: static_func(SoxNone::equals),
            },
        )];
}
impl SoxObjectPayload for SoxNone {
    fn to_sox_type_value(obj: SoxObject) -> SoxRef<Self> {
        obj.as_none().unwrap()
    }

    fn to_sox_object(&self, ref_type: SoxRef<Self>) -> SoxObject {
        SoxObject::None(ref_type)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn into_ref(self) -> SoxObject {
        SoxRef::new(self).to_sox_object()
    }

    fn class(&self, i: &Interpreter) -> &'static SoxType {
        i.types.none_type
    }
}

impl StaticType for SoxNone {
    const NAME: &'static str = "none";

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


impl TryFromSoxObject for SoxNone {
    fn try_from_sox_object(_i: &Interpreter, obj: SoxObject) -> SoxResult<Self> {
        if let Some(val) = obj.as_none() {
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

impl ToSoxResult for SoxNone {
    fn to_sox_result(self, _i: &Interpreter) -> SoxResult {
        let obj = self.into_ref();
        Ok(obj)
    }
}



impl Representable for SoxNone {
    fn repr(&self, i: &Interpreter) -> String {
        "None".to_string()
    }
}
