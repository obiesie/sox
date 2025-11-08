use crate::builtins::bool::SoxBool;
use crate::builtins::core::{SoxObjectPayload, SoxResult};
use crate::builtins::method::FuncArgs;
use crate::builtins::r#type::SoxType;
use crate::interpreter::Interpreter;
use std::any::TypeId;
use std::borrow::Borrow;
use std::mem::MaybeUninit;
use std::ops::Deref;
use std::ptr;
use std::ptr::NonNull;

#[repr(transparent)]
pub struct SoxObject(SoxObjectInner<()>);

#[derive(Debug, Copy)]
#[repr(transparent)]
pub struct SoxObjectRef {
    pub ptr: NonNull<SoxObject>,
}

impl<T: SoxObjectPayload> Deref for Sox<T> {
    type Target = T;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.0.payload
    }
}

impl SoxObjectRef {
    fn call_method(&self, method_name: &str, interpreter: &Interpreter) -> Option<SoxObjectRef> {
        self.typ().methods.get(method_name).and_then(|method| {
            let call_args = FuncArgs {
                args: vec![self.clone()],
            };
            (method.func)(interpreter, call_args).ok()
        })
    }

    pub fn typ(&self) -> &Sox<SoxType> {
        let obj = unsafe { self.ptr.as_ref() };
        let cls = obj.0.typ.deref();
        cls
    }

    pub fn payload<T: SoxObjectPayload>(&self) -> Option<&T> {
        if self.payload_is::<T>() {
            Some(unsafe { self.payload_unchecked() })
        } else {
            None
        }
    }

    pub fn payload_is<T: SoxObjectPayload>(&self) -> bool {
        unsafe { self.ptr.as_ref().0.type_id == TypeId::of::<T>() }
    }

    pub unsafe fn payload_unchecked<T: SoxObjectPayload>(&self) -> &T {
        let v = self.ptr.as_ref();
        let inner = unsafe { &*(v as *const SoxObject as *const SoxObjectInner<T>) };
        &inner.payload
    }

    pub fn downcast_ref<T: SoxObjectPayload>(&self) -> Option<&Sox<T>> {
        if self.payload_is::<T>() {
            Some(unsafe { self.downcast_unchecked_ref::<T>() })
        } else {
            None
        }
    }

    pub unsafe fn downcast_unchecked_ref<T: SoxObjectPayload>(&self) -> &Sox<T> {
        debug_assert!(self.payload_is::<T>());
        &*(self as *const SoxObjectRef as *const SoxRef<T>)
    }

    // TODO migrate this to a protocol?
    pub fn try_into_rust_bool(&self, i: &Interpreter) -> bool {
        self.call_method("bool", i)
            .and_then(|tv| tv.payload::<SoxBool>().map(|v| v.value))
            .unwrap_or(true)
    }

    pub fn repr(&self, i: &Interpreter) -> SoxResult<String> {
        let typ = self.typ();
        match typ.slots.repr {
            None => Ok("No repr implementation found.".to_string()),
            Some(f) => f(self, i),
        }
    }
}

impl<T: SoxObjectPayload> From<SoxRef<T>> for SoxObjectRef {
    fn from(value: SoxRef<T>) -> Self {
        Self {
            ptr: value.ptr.cast(),
        }
    }
}

impl Clone for SoxObjectRef {
    fn clone(&self) -> Self {
        Self { ptr: self.ptr }
    }
}

#[repr(C)]
pub struct SoxObjectInner<T> {
    pub type_id: TypeId,
    pub typ: SoxRef<SoxType>,
    pub payload: T,
}

impl<T: SoxObjectPayload> SoxObjectInner<T> {
    pub fn new(d: T, typ: SoxRef<SoxType>) -> Box<Self> {
        Box::new(SoxObjectInner {
            type_id: TypeId::of::<T>(),
            typ,
            payload: d,
        })
    }
}

#[repr(transparent)]
pub struct Sox<T: SoxObjectPayload>(SoxObjectInner<T>);

impl<T: SoxObjectPayload> Sox<T> {}

impl<T: SoxObjectPayload> ToOwned for Sox<T> {
    type Owned = SoxRef<T>;

    fn to_owned(&self) -> Self::Owned {
        SoxRef {
            ptr: NonNull::from(self),
        }
    }
}

#[repr(transparent)]
#[derive(Debug)]
pub struct SoxRef<T: SoxObjectPayload> {
    pub ptr: NonNull<Sox<T>>,
}

impl<T: SoxObjectPayload> SoxRef<T> {
    pub unsafe fn from_raw(raw: *const Sox<T>) -> Self {
        Self {
            ptr: NonNull::new_unchecked(raw as *mut _),
        }
    }

    pub fn new_ref(payload: T, typ: SoxRef<SoxType>) -> SoxRef<T> {
        let inner = Box::into_raw(SoxObjectInner::new(payload, typ));
        Self {
            ptr: unsafe { NonNull::new_unchecked(inner.cast::<Sox<T>>()) },
        }
    }
}

unsafe impl<T: SoxObjectPayload> Sync for SoxRef<T> {}

unsafe impl<T: SoxObjectPayload> Send for SoxRef<T> {}

impl<T: SoxObjectPayload> Clone for SoxRef<T> {
    fn clone(&self) -> Self {
        Self { ptr: self.ptr }
    }
}

impl<T> Deref for SoxRef<T>
where
    T: SoxObjectPayload,
{
    type Target = Sox<T>;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        unsafe { self.ptr.as_ref() }
    }
}

impl<T: SoxObjectPayload> Borrow<Sox<T>> for SoxRef<T> {
    fn borrow(&self) -> &Sox<T> {
        todo!()
    }
}

pub fn init_type_type() -> SoxRef<SoxType> {
    let typ = {
        let type_payload = SoxType {
            base: None,
            methods: Default::default(),
            slots: Default::default(),
            attributes: Default::default(),
            name: Some("SoxType".to_owned()),
        };

        let type_type_ptr =
            Box::into_raw(Box::new(MaybeUninit::<SoxObjectInner<SoxType>>::uninit()))
                as *mut SoxObjectInner<SoxType>;
        unsafe {
            ptr::write(&mut (*type_type_ptr).type_id, TypeId::of::<SoxType>());
            ptr::write(&mut (*type_type_ptr).payload, type_payload);

            let type_type = SoxRef::<SoxType>::from_raw(type_type_ptr.cast());
            ptr::write(&mut (*type_type_ptr).typ, type_type.clone());
            type_type
        }
    };
    typ
}

#[cfg(test)]
mod tests {
    use crate::builtins::string::SoxString;
    use crate::interpreter::Interpreter;
    use crate::object::core::{SoxObjectRef, SoxRef};
    use std::any::TypeId;

    #[test]
    fn test_sox_ref() {
        let payload = SoxString {
            value: "hello".to_owned(),
        };
        let i = Interpreter::new();
        let typ = i.types.str_type.to_owned();

        let typ_ref = SoxRef::new_ref(payload, typ);
        let obj = SoxObjectRef::from(typ_ref);

        let t = obj.payload::<SoxString>();
        let a = t.unwrap().value.clone();
        println!("Value is {a}");

        let typ_id = TypeId::of::<SoxString>();
        println!("type of soxstring is {:?}", typ_id);

        let typ = obj.typ();
        println!("type of obj is {:?}", typ.name);
    }
}
