use crate::builtins::core::SoxResult;
use crate::object::core::SoxObjectRef;
use crate::runtime::Runtime;

#[derive(Clone, Debug, Default)]
pub struct NumberMethods {
    pub add: Option<fn(a: SoxObjectRef, b: SoxObjectRef, i: &mut Runtime) -> SoxResult>,
    pub minus: Option<fn(a: SoxObjectRef, b: SoxObjectRef, i: &mut Runtime) -> SoxResult>,
    pub star: Option<fn(a: SoxObjectRef, b: SoxObjectRef, i: &mut Runtime) -> SoxResult>,
    pub slash: Option<fn(a: SoxObjectRef, b: SoxObjectRef, i: &mut Runtime) -> SoxResult>,
    pub rem: Option<fn(a: SoxObjectRef, b: SoxObjectRef, i: &mut Runtime) -> SoxResult>,
    pub neg: Option<fn(a: SoxObjectRef, i: &mut Runtime) -> SoxResult>,
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
