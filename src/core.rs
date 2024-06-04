use std::any::Any;
use std::collections::HashMap;
use std::fmt::Debug;
use std::ops::Deref;
use std::rc::Rc;

pub use once_cell::sync::{Lazy, OnceCell};

use crate::builtins::bool_::SoxBool;
use crate::builtins::exceptions::Exception;
use crate::builtins::float::SoxFloat;
use crate::builtins::function::SoxFunction;
use crate::builtins::int::SoxInt;
use crate::builtins::method::{FuncArgs, SoxMethod};
use crate::builtins::none::SoxNone;
use crate::builtins::r#type::{SoxClassInstance, SoxType, SoxTypeSlot};
use crate::builtins::string::SoxString;
use crate::interpreter::Interpreter;

#[derive(Clone, Debug)]
pub enum SoxObject {
    Int(SoxRef<SoxInt>),
    String(SoxRef<SoxString>),
    Float(SoxRef<SoxFloat>),
    Boolean(SoxRef<SoxBool>),
    Function(SoxRef<SoxFunction>),
    Exception(SoxRef<Exception>),
    None(SoxRef<SoxNone>),
    Class(SoxRef<SoxType>),
    ClassInstance(SoxRef<SoxClassInstance>),
}

impl SoxObject {
    pub fn sox_type(&self, i: &Interpreter) -> &'static SoxType {
        let typ = match &self {
            SoxObject::Int(v) => v.class(i),
            SoxObject::String(v) => v.class(i),
            SoxObject::Float(v) => v.class(i),
            SoxObject::Boolean(v) => v.class(i),
            SoxObject::Function(v) => v.class(i),
            SoxObject::Exception(v) => v.class(i),
            SoxObject::None(v) => v.class(i),
            SoxObject::Class(v) => v.class(i),
            SoxObject::ClassInstance(v) => v.class(i),
        };
        return typ;
    }

    pub fn try_into_rust_bool(&self, i: &Interpreter) -> bool {
        let typ = self.sox_type(i);

        let bool_method = typ.methods.get("bool");
        let truth_val = if let Some(meth) = bool_method {
            let call_args = FuncArgs {
                args: vec![self.clone()],
            };
            let truth_val = (meth.func)(i, call_args);
            if let Ok(tv) = truth_val {
                tv.as_bool().map_or(false, |v| v.value)
            } else {
                false
            }
        } else {
            true
        };
        truth_val
    }

    pub fn as_int(&self) -> Option<SoxRef<SoxInt>> {
        match self {
            SoxObject::Int(v) => Some(v.clone()),
            _ => None,
        }
    }

    pub fn as_float(&self) -> Option<SoxRef<SoxFloat>> {
        match self {
            SoxObject::Float(v) => Some(v.clone()),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<SoxRef<SoxBool>> {
        match self {
            SoxObject::Boolean(v) => Some(v.clone()),
            _ => None,
        }
    }

    pub fn as_string(&self) -> Option<SoxRef<SoxString>> {
        match self {
            SoxObject::String(v) => Some(v.clone()),
            _ => None,
        }
    }

    pub fn as_none(&self) -> Option<SoxRef<SoxNone>> {
        match self {
            SoxObject::None(v) => Some(v.clone()),
            _ => None,
        }
    }

    pub fn as_func(&self) -> Option<SoxRef<SoxFunction>> {
        match self {
            SoxObject::Function(v) => Some(v.clone()),
            _ => None,
        }
    }

    pub fn as_exception(&self) -> Option<SoxRef<Exception>> {
        match self {
            SoxObject::Exception(v) => Some(v.clone()),
            _ => None,
        }
    }

    pub fn as_type(&self) -> Option<SoxRef<SoxType>> {
        match self {
            SoxObject::Class(v) => Some(v.clone()),
            _ => None,
        }
    }

    pub fn as_class_instance(&self) -> Option<SoxRef<SoxClassInstance>> {
        match self {
            SoxObject::ClassInstance(v) => Some(v.clone()),
            _ => None,
        }
    }
}

pub type SoxResult<T = SoxObject> = Result<T, SoxObject>;

pub trait SoxNativeFunction {
    fn call(&self, args: i64) -> SoxObject;
}

pub trait SoxClassImpl {
    const METHOD_DEFS: &'static [(&'static str, SoxMethod)];
}

pub trait StaticType {
    const NAME: &'static str;
    fn static_cell() -> &'static OnceCell<SoxType>;
    fn init_builtin_type() -> &'static SoxType
    where
        Self: SoxClassImpl,
    {
        let typ: SoxType = Self::create_static_type();
        let cell = Self::static_cell();
        cell.set(typ)
            .unwrap_or_else(|_| panic!("double initialization of {}", Self::NAME));
        let v = cell.get().unwrap();
        v
    }

    fn create_slots() -> SoxTypeSlot;
    fn create_static_type() -> SoxType
    where
        Self: SoxClassImpl,
    {
        let methods = Self::METHOD_DEFS;
        let slots = Self::create_slots();
        SoxType::new(
            "".to_string(),
            None,
            methods
                .iter()
                .map(move |v| (v.0.to_string(), v.1.clone()))
                .collect::<HashMap<String, SoxMethod>>(),
            slots,
            Default::default(),
        )
    }
}

unsafe impl Send for SoxType {}

unsafe impl Sync for SoxType {}

impl ToSoxResult for SoxObject {
    fn to_sox_result(self, _i: &Interpreter) -> SoxResult {
        Ok(self)
    }
}

#[derive(Debug, PartialEq)]
pub struct SoxRef<T> {
    pub val: Rc<T>,
}

impl<T: SoxObjectPayload> SoxRef<T> {
    pub fn new(obj: T) -> Self {
        Self { val: Rc::new(obj) }
    }

    pub fn to_sox_object(self) -> SoxObject {
        self.val.to_sox_object(self.clone())
    }
}

impl<T: SoxObjectPayload> Deref for SoxRef<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        return &self.val;
    }
}

impl<T> Clone for SoxRef<T> {
    fn clone(&self) -> Self {
        Self {
            val: Rc::clone(&self.val),
        }
    }
}

impl<T: SoxObjectPayload> TryFromSoxObject for SoxRef<T> {
    fn try_from_sox_object(_i: &Interpreter, obj: SoxObject) -> SoxResult<Self> {
        Ok(T::to_sox_type_value(obj))
    }
}

impl<T: SoxObjectPayload> ToSoxResult for SoxRef<T> {
    fn to_sox_result(self, _i: &Interpreter) -> SoxResult {
        Ok(self.to_sox_object())
    }
}

pub trait TryFromSoxObject: Sized {
    fn try_from_sox_object(i: &Interpreter, obj: SoxObject) -> SoxResult<Self>;
}

pub trait ToSoxResult: Sized {
    fn to_sox_result(self, i: &Interpreter) -> SoxResult;
}

impl ToSoxResult for SoxResult {
    fn to_sox_result(self, _i: &Interpreter) -> SoxResult {
        self
    }
}

pub trait SoxObjectPayload: Any + Sized + 'static {
    fn to_sox_type_value(obj: SoxObject) -> SoxRef<Self>;

    fn to_sox_object(&self, ref_type: SoxRef<Self>) -> SoxObject;
    fn as_any(&self) -> &dyn Any;

    fn into_ref(self) -> SoxObject {
        SoxRef::new(self).to_sox_object()
    }

    fn class(&self, i: &Interpreter) -> &'static SoxType;
}

#[cfg(test)]
mod tests {}
