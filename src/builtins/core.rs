use std::any::Any;
use std::collections::HashMap;

pub use once_cell::sync::{Lazy, OnceCell};

use crate::builtins::method::SoxMethod;
use crate::builtins::r#type::{SoxType, SoxTypeSlot};
use crate::object::core::{Sox, SoxObjectRef, SoxRef};
use crate::runtime::Runtime;

pub type SoxResult<T = SoxObjectRef> = Result<T, SoxObjectRef>;

pub trait SoxNativeFunction {
    fn call(&self, args: i64) -> SoxObjectRef;
}

pub trait SoxClassImpl {
    const METHOD_DEFS: &'static [(&'static str, SoxMethod)];
}

pub trait StaticType {
    const NAME: &'static str;
    fn init_manually(typ: SoxRef<SoxType>) -> &'static Sox<SoxType> {
        let cell = Self::static_cell();
        cell.set(typ)
            .unwrap_or_else(|_| panic!("double initialization from init_manually"));
        cell.get().unwrap()
    }
    fn static_cell() -> &'static OnceCell<SoxRef<SoxType>>;
    fn init_builtin_type() -> &'static Sox<SoxType>
    where
        Self: SoxClassImpl,
    {
        let typ = Self::create_static_type();
        let cell = Self::static_cell();
        cell.set(typ)
            .unwrap_or_else(|_| panic!("Double initialization"));
        let v = cell.get().unwrap();
        v
    }

    fn create_slots() -> SoxTypeSlot;
    fn create_static_type() -> SoxRef<SoxType>
    where
        Self: SoxClassImpl,
    {
        let methods = Self::METHOD_DEFS;
        let slots = Self::create_slots();
        SoxType::new_static_type(
            Self::NAME,
            None,
            methods
                .iter()
                .map(move |v| (v.0.to_string(), v.1.clone()))
                .collect::<HashMap<String, SoxMethod>>(),
            slots,
            Default::default(),
            Self::static_metaclass().to_owned(),
        )
    }
    fn static_metaclass() -> &'static Sox<SoxType> {
        SoxType::static_type()
    }

    fn static_type() -> &'static Sox<SoxType> {
        Self::static_cell()
            .get()
            .expect("static type has not been initialized. e.g. the native types defined in different module may be used before importing library.")
    }
}

unsafe impl Send for SoxType {}

unsafe impl Sync for SoxType {}

impl ToSoxResult for SoxObjectRef {
    fn to_sox_result(self, _i: &mut Runtime) -> SoxResult {
        Ok(self)
    }
}

pub trait TryFromSoxObject: Sized {
    fn try_from_sox_object(i: &mut Runtime, obj: SoxObjectRef) -> SoxResult<Self>;
}

pub trait ToSoxResult: Sized {
    fn to_sox_result(self, i: &mut Runtime) -> SoxResult;
}

impl ToSoxResult for SoxResult {
    fn to_sox_result(self, _i: &mut Runtime) -> SoxResult {
        self
    }
}

pub trait SoxObjectPayload: Any + Sized + 'static {
    fn as_any(&self) -> &dyn Any;
}
