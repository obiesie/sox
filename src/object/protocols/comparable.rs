use crate::builtins::core::SoxResult;
use crate::object::core::SoxObjectRef;
use crate::runtime::Runtime;

pub trait Comparable {
    fn as_comparable() -> ComparableMethods;
}

#[derive(Clone, Debug, Default)]
pub struct ComparableMethods {
    pub lt: Option<fn(a: SoxObjectRef, b: SoxObjectRef, i: &mut Runtime) -> SoxResult>,
    pub gt: Option<fn(a: SoxObjectRef, b: SoxObjectRef, i: &mut Runtime) -> SoxResult>,
    pub eq: Option<fn(a: SoxObjectRef, b: SoxObjectRef, i: &mut Runtime) -> SoxResult>,
    pub ne: Option<fn(a: SoxObjectRef, b: SoxObjectRef, i: &mut Runtime) -> SoxResult>,
    pub ge: Option<fn(a: SoxObjectRef, b: SoxObjectRef, i: &mut Runtime) -> SoxResult>,
    pub le: Option<fn(a: SoxObjectRef, b: SoxObjectRef, i: &mut Runtime) -> SoxResult>,
}
