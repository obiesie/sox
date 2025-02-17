use crate::builtins::core::SoxResult;
use crate::interpreter::Interpreter;
use crate::object::core::SoxObjectRef;

#[derive(Clone, Debug, Default)]
pub struct NumberMethods {
    pub add: Option<fn(a: SoxObjectRef, b: SoxObjectRef, i: &Interpreter) -> SoxResult>,
    pub minus: Option<fn(a: SoxObjectRef, b: SoxObjectRef, i: &Interpreter) -> SoxResult>,
    pub star: Option<fn(a: SoxObjectRef, b: SoxObjectRef, i: &Interpreter) -> SoxResult>,
    pub slash: Option<fn(a: SoxObjectRef, b: SoxObjectRef, i: &Interpreter) -> SoxResult>,
    pub rem: Option<fn(a: SoxObjectRef, b: SoxObjectRef, i: &Interpreter) -> SoxResult>,
    pub neg: Option<fn(a: SoxObjectRef, i: &Interpreter) -> SoxResult>,
}

pub trait AsNumber: Sized {
    const DEFAULT_NUMBER_METHODS: NumberMethods = NumberMethods {
        add: None,
        minus: None,
        star: None,
        slash: None,
        rem: None,
        neg: None,
    };

    fn as_number() -> NumberMethods;
}
