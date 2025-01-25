use crate::builtins::core::{SoxClassImpl, SoxObjectPayload, StaticType};
use crate::builtins::method::SoxMethod;
use crate::interpreter::Interpreter;

use crate::builtins::r#type::{SoxType, SoxTypeSlot};
use crate::object::core::{Sox, SoxObjectRef, SoxRef};
use crate::object::protocols::repr::Representable;
use once_cell::sync::OnceCell;
use std::any::Any;
use std::fmt::Debug;
use std::ops::Deref;

#[derive(Clone, Debug)]
pub enum Exception {
    Err(RuntimeError),
    Return(SoxObjectRef),
}

impl Representable for Exception {
    fn repr(zelf: &Sox<Self>, _i: &Interpreter) -> String {
        match zelf.deref() {
            Exception::Err(v) => v.msg.to_string(),
            Exception::Return(_) => "Return".to_string(),
        }
    }
}
impl From<RuntimeError> for Exception {
    fn from(value: RuntimeError) -> Self {
        Exception::Err(value)
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
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
    fn as_any(&self) -> &dyn Any {
        todo!()
    }
}

impl StaticType for Exception {
    const NAME: &'static str = "exception";

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

impl SoxClassImpl for Exception {
    const METHOD_DEFS: &'static [(&'static str, SoxMethod)] = &[];
}
