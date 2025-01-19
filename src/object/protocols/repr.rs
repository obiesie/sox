use crate::builtins::core::{SoxObjectPayload, SoxResult};
use crate::object::core::{Sox, SoxObjectRef};
use crate::interpreter::Interpreter;

pub trait Representable {
    fn slot_repr(zelf: &SoxObjectRef, i: &Interpreter) -> SoxResult<String>
    where
        Self: SoxObjectPayload,
    {
        let zelf = zelf.downcast_ref().unwrap();
        Ok(Self::repr(zelf, i))
    }
    fn repr(zelf: &Sox<Self>, i: &Interpreter) -> String
    where
        Self: SoxObjectPayload;
}