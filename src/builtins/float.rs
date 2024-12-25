use once_cell::sync::OnceCell;
use std::any::Any;
use macros::{soxmethod, soxtype};
use crate::builtins::bool::SoxBool;
use crate::builtins::method::{static_func, SoxMethod};
use crate::builtins::r#type::{SoxType, SoxTypeSlot};
use crate::builtins::string::SoxString;
use crate::builtins::core::{SoxClassImpl, SoxObjectPayload, SoxResult, StaticType, ToSoxResult, TryFromSoxObject};
use crate::interpreter::Interpreter;
use crate::object::core::{Sox, SoxObjectRef, SoxRef};
use crate::slots::repr::Representable;

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
    pub fn equals(&self, other: SoxObjectRef) -> SoxBool {
        if let Some(other_float) = other.payload::<SoxFloat>() {
            SoxBool::from(other_float.value == self.value)
        } else {
            SoxBool::from(false)
        }
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
