use std::any::Any;
use std::fmt::Debug;
use once_cell::sync::OnceCell;
use macros::soxtype;
use crate::core::{SoxClassImpl, SoxObject, SoxObjectPayload, SoxRef, SoxType, SoxTypeSlot, StaticType};
use crate::interpreter::Interpreter;
use crate::builtins::method::SoxMethod;



#[derive(Clone, Debug)]
pub enum Exception {
    RuntimeErr{msg: String},
    ArgumentErr{msg: String},
    
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

impl SoxObjectPayload for Exception{
    fn to_sox_type_value(obj: SoxObject) -> SoxRef<Self> {
        todo!()
    }

    fn to_sox_object(&self, ref_type: SoxRef<Self>) -> SoxObject {
        todo!()
    }

    fn as_any(&self) -> &dyn Any {
        todo!()
    }

    fn into_ref(self) -> SoxObject {
        todo!()
    }

    fn class(&self, i: &Interpreter) -> &'static SoxType {
        todo!()
    }
}


impl StaticType for Exception{
    const NAME: &'static str = "";

    fn static_cell() -> &'static OnceCell<SoxType> {
        todo!()
    }

    fn create_slots() -> SoxTypeSlot {
        todo!()
    }
}

impl SoxClassImpl for Exception{
    const METHOD_DEFS: &'static [(&'static str, SoxMethod)] = &[];
}