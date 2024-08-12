use crate::builtins::method::SoxMethod;
use crate::core::{SoxClassImpl, SoxObject, SoxObjectPayload, SoxRef, StaticType};
use crate::interpreter::Interpreter;

use crate::builtins::r#type::{SoxType, SoxTypeSlot};
use std::any::Any;
use std::fmt::Debug;
use std::sync::OnceLock;

#[derive(Clone, Debug)]
pub enum Exception {
    Err(RuntimeError),
    Return(SoxObject),
}

impl From<RuntimeError> for Exception {
    fn from(value: RuntimeError) -> Self {
        Exception::Err(value)
    }
}

#[derive(Clone, Debug, Default)]
pub struct RuntimeError {
    pub msg: String,
}

impl From<Exception> for RuntimeError {
    fn from(value: Exception) -> Self {
        if let Exception::Err(v) = value {
            v
        } else {
            RuntimeError { msg: "".into() }
        }
    }
}

impl SoxObjectPayload for Exception {
    fn to_sox_type_value(_obj: SoxObject) -> SoxRef<Self> {
        todo!()
    }

    fn to_sox_object(&self, ref_type: SoxRef<Self>) -> SoxObject {
        SoxObject::Exception(ref_type)
    }

    fn as_any(&self) -> &dyn Any {
        todo!()
    }

    fn class(&self, i: &Interpreter) -> &'static SoxType {
        i.types.exception_type
    }
}

impl StaticType for Exception {
    const NAME: &'static str = "";
    

    fn create_slots() -> SoxTypeSlot {
        SoxTypeSlot { call: None }
    }
}

impl SoxClassImpl for Exception {
    const METHOD_DEFS: &'static [(&'static str, SoxMethod)] = &[];

    fn static_cell() -> &'static OnceLock<SoxType> {
        static CELL: OnceLock<SoxType> = OnceLock::new();
        &CELL
    }
}
