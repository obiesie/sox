use crate::builtins::core::{SoxObjectPayload, SoxResult};
use crate::interpreter::Interpreter;
use crate::object::core::{Sox, SoxObjectRef};

pub trait Representable {
    fn slot_repr(zelf: &SoxObjectRef, i: &Interpreter) -> SoxResult<String>
    where
        Self: SoxObjectPayload,
    {
        let tmp = zelf.downcast_ref();
        if tmp.is_none() {
            println!("{:?}", zelf);
        }
        let zelf = tmp.unwrap();
        Ok(Self::repr(zelf, i))
    }
    fn repr(zelf: &Sox<Self>, i: &Interpreter) -> String
    where
        Self: SoxObjectPayload;
}
