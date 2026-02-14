use crate::builtins::method::SoxMethod;
use crate::builtins::r#type::{SoxType, SoxTypeSlot};
use crate::builtins::string::SoxString;
use macros::soxtype;
use once_cell::sync::OnceCell;
use std::any::Any;

use crate::builtins::core::{
    SoxClassImpl, SoxObjectPayload, SoxResult, StaticType, ToSoxResult, TryFromSoxObject,
};
use crate::object::core::SoxObjectRef;
use crate::object::core::{Sox, SoxRef};
use crate::object::protocols::repr::Representable;
use crate::runtime::Runtime;

#[derive(Clone, Debug)]
pub struct SoxUpvalue {
    pub location: usize,
    pub closed: Option<Box<SoxObjectRef>>,
}

#[soxtype]
impl SoxUpvalue {
    pub fn new(location: usize) -> Self {
        Self {
            location,
            closed: None,
        }
    }

    fn slot_trace(obj: &SoxObjectRef, trace_fn: &mut dyn FnMut(SoxObjectRef)) {
        if let Some(upvalue) = obj.payload::<SoxUpvalue>() {
            if let Some(closed) = &upvalue.closed {
                trace_fn(*closed.clone());
            }
        }
    }
}

impl Representable for SoxUpvalue {
    fn repr(zelf: &Sox<Self>, _i: &Runtime) -> String {
        let value_repr = if let Some(closed) = &zelf.closed {
            closed.repr(_i).unwrap_or_default()
        } else {
            format!("open at {}", zelf.location)
        };
        format!("<Upvalue {value_repr}>")
    }
}
impl SoxObjectPayload for SoxUpvalue {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl StaticType for SoxUpvalue {
    const NAME: &'static str = "upvalue";

    fn static_cell() -> &'static OnceCell<SoxRef<SoxType>> {
        static CELL: OnceCell<SoxRef<SoxType>> = OnceCell::new();
        &CELL
    }

    fn create_slots() -> SoxTypeSlot {
        SoxTypeSlot {
            call: None,
            repr: Some(Self::slot_repr),
            trace: Some(Self::slot_trace),
            drop: Some(Self::slot_drop),
            number: None,
            comparable: None,
            methods: Self::METHOD_DEFS,
        }
    }
}

impl SoxUpvalue {
    fn slot_drop(obj: &SoxObjectRef) {
        if obj.payload::<SoxUpvalue>().is_some() {
            unsafe {
                let inner =
                    obj.ptr.as_ptr() as *mut crate::object::core::SoxObjectInner<SoxUpvalue>;
                std::ptr::drop_in_place(&mut (*inner).payload);
            }
        }
    }
}

impl TryFromSoxObject for SoxUpvalue {
    fn try_from_sox_object(i: &mut Runtime, obj: SoxObjectRef) -> SoxResult<Self> {
        if let Some(val) = obj.payload::<SoxUpvalue>() {
            Ok(val.clone())
        } else {
            let err_msg = SoxString {
                value: String::from("failed to get an upvalue from supplied object"),
            };
            let ob = i.alloc(
                err_msg,
                crate::builtins::string::SoxString::init_builtin_type().to_owned(),
            );
            Err(ob.into())
        }
    }
}

impl ToSoxResult for SoxUpvalue {
    fn to_sox_result(self, i: &mut Runtime) -> SoxResult {
        let obj = i.alloc(self, i.types.upvalue_type.to_owned());
        Ok(SoxObjectRef::from(obj))
    }
}
