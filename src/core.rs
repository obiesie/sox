use std::any::Any;
use std::collections::HashMap;
use std::fmt::Debug;
use std::ops::Deref;
use std::rc::Rc;

pub use once_cell::sync::{Lazy, OnceCell};
use crate::builtins::exceptions::Exception;
use crate::builtins::function::SoxFunction;
use crate::builtins::int::SoxInt;
use crate::builtins::float::SoxFloat;
use crate::builtins::bool_::SoxBool;
use crate::builtins::method::{FuncArgs, SoxMethod};
use crate::builtins::none::SoxNone;
use crate::builtins::string::SoxString;
use crate::interpreter::Interpreter;


pub type SoxResult<T = SoxObject> = Result<T, SoxObject>;

pub trait SoxNativeFunction {
    fn call(&self, args: i64) -> SoxObject;
}

#[derive(Debug)]
pub struct SoxTypeSlot {
    pub call: Option<GenericMethod>,
}

pub trait SoxClassImpl {
    const METHOD_DEFS: &'static [(&'static str, SoxMethod)];
}

pub trait StaticType {
    const NAME: &'static str;
    fn static_cell() -> &'static OnceCell<SoxType>;
    fn init_builtin_type() -> &'static SoxType where Self: SoxClassImpl {
        let typ = Self::create_static_type();
        let cell = Self::static_cell();
        cell.set(typ)
            .unwrap_or_else(|_| panic!("double initialization of {}", Self::NAME));
        let v = cell.get().unwrap();
        v
    }

    fn create_slots() -> SoxTypeSlot;
    fn create_static_type() -> SoxType where Self: SoxClassImpl {
        let methods = Self::METHOD_DEFS;
        let slots = Self::create_slots();
        SoxType::new(None,
                     Default::default(),
                     methods.iter().map(move |v| (v.0.to_string(), v.1.clone())).collect::<HashMap<String, SoxMethod>>(),
                     slots,
        )
    }

    fn static_type() -> &'static SoxType {
        Self::static_cell()
            .get()
            .expect("static type has not been initialized")
    }
}


unsafe impl Send for SoxType {}

unsafe impl Sync for SoxType {}


#[derive(Clone, Debug)]
pub enum SoxObject {
    Int(SoxRef<SoxInt>),
    String(SoxRef<SoxString>),
    Float(SoxRef<SoxFloat>),
    Boolean(SoxRef<SoxBool>),
    SoxFunction(SoxRef<SoxFunction>),
    Exception(SoxRef<Exception>),
    None,
}


impl ToSoxResult for SoxObject {
    fn to_sox_result(self, i: &Interpreter) -> SoxResult {
        Ok(self)
    }
}

#[derive(Debug)]
pub struct SoxRef<T> {
    pub(crate) val: Rc<T>,
}

impl<T: SoxObjectPayload> SoxRef<T> {
    pub fn new(obj: T) -> Self {
        Self {
            val: Rc::new(obj)
        }
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
            val: Rc::clone(&self.val)
        }
    }
}


impl<T: SoxObjectPayload> TryFromSoxObject for SoxRef<T> {
    fn try_from_sox_object(i: &Interpreter, obj: SoxObject) -> SoxResult<Self> {
        Ok(T::to_sox_type_value(obj))
    }
}

impl<T: SoxObjectPayload> ToSoxResult for SoxRef<T> {
    fn to_sox_result(self, i: &Interpreter) -> SoxResult {
        Ok(self.to_sox_object())
    }
}

impl SoxObject {
    pub fn sox_type(&self, i: &Interpreter) -> &'static SoxType {
        let typ = match &self {
            SoxObject::Int(v) => { v.class(i) }
            SoxObject::String(v) => { v.class(i) }
            SoxObject::Float(v) => { v.class(i) }
            SoxObject::Boolean(v) => { v.class(i) }
            SoxObject::SoxFunction(v) => v.class(i),
            SoxObject::Exception(v) => v.class(i),
            SoxObject::None => { i.types.none_type }
        };
        return typ;
    }

    pub fn try_into_rust_bool(&self, i: &Interpreter) -> bool {
        let typ = self.sox_type(i);

        let bool_method = typ.methods.get("bool_");
        let truth_val = if let Some(meth) = bool_method {
            let call_args = FuncArgs { args: vec![self.clone()] };
            let truth_val = (meth.func)(i, call_args).unwrap();
            truth_val.as_bool().unwrap().value
        } else {
            true
        };
        truth_val
    }

    pub fn as_int(&self) -> Option<SoxRef<SoxInt>> {
        match self {
            SoxObject::Int(v) => {
                Some(v.clone())
            }
            _ => None
        }
    }

    pub fn as_float(&self) -> Option<SoxRef<SoxFloat>> {
        match self {
            SoxObject::Float(v) => {
                Some(v.clone())
            }
            _ => None
        }
    }

    pub fn as_bool(&self) -> Option<SoxRef<SoxBool>> {
        // TODO implement bool implementation for other types here too

        match self {
            SoxObject::Boolean(v) => {
                Some(v.clone())
            }
            _ => None
        }
    }

    pub fn as_string(&self) -> Option<SoxRef<SoxString>> {
        match self {
            SoxObject::String(v) => {
                Some(v.clone())
            }
            _ => None
        }
    }

    pub fn as_none(&self) -> Option<SoxRef<SoxNone>> {
        match self {
            SoxObject::None => {
                Some(SoxRef::new(SoxNone {}))
            }
            _ => None
        }
    }

    pub fn as_func(&self) -> Option<SoxRef<SoxFunction>> {
        match self {
            SoxObject::SoxFunction(v) => {
                Some(v.clone())
            }
            _ => None
        }
    }

    pub fn as_exception(&self) -> Option<SoxRef<Exception>> {
        match self {
            SoxObject::Exception(v) => {
                Some(v.clone())
            }
            _ => None
        }
    }
}

pub trait TryFromSoxObject: Sized {
    /// Attempt to convert a Sox object to a value of this type.
    fn try_from_sox_object(i: &Interpreter, obj: SoxObject) -> SoxResult<Self>;
}


pub trait ToSoxResult: Sized {
    fn to_sox_result(self, i: &Interpreter) -> SoxResult;
}


impl ToSoxResult for SoxResult {
    fn to_sox_result(self, i: &Interpreter) -> SoxResult {
        self
    }
}

pub trait Callable {
    type T;
    fn call(&self, interpreter: &mut Interpreter, args: Vec<SoxObject>) -> Self::T;
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


pub type SoxAttributes = HashMap<String, SoxObject>;
pub(crate) type GenericMethod = fn(SoxObject, FuncArgs, &mut Interpreter) -> SoxResult;

#[derive(Debug)]
pub struct SoxType {
    pub base: Option<SoxTypeRef>,
    pub attributes: SoxAttributes,
    pub methods: HashMap<String, SoxMethod>,
    pub slots: SoxTypeSlot,
}

impl SoxType {
    pub fn new(base: Option<SoxTypeRef>, attributes: SoxAttributes,
               methods: HashMap<String, SoxMethod>, slots: SoxTypeSlot) -> Self {
        Self {
            base,
            attributes,
            methods,
            slots,
        }
    }
}


pub type SoxTypeRef = Rc<SoxType>;

#[cfg(test)]
mod tests {}

