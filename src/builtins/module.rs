use crate::builtins::bool::SoxBool;
use crate::builtins::chunk::Chunk;
use crate::builtins::core::{
    SoxClassImpl, SoxObjectPayload, SoxResult, StaticType, ToSoxResult, TryFromSoxObject,
};
use crate::builtins::method::{static_func, SoxMethod};
use crate::builtins::r#type::{SoxType, SoxTypeSlot};
use crate::builtins::string;
use crate::builtins::string::SoxString;
use crate::object::core::{Sox, SoxObjectRef, SoxRef};
use crate::object::protocols::repr::Representable;
use crate::runtime::Runtime;
use macros::{soxmethod, soxtype};
use once_cell::sync::OnceCell;
use std::any::Any;

#[derive(Debug, Clone)]
pub struct SoxModule {
    name: String,
    pub co: SoxRef<Chunk>,
}

#[soxtype]
impl SoxModule {
    pub fn new(name: String, co: SoxRef<Chunk>) -> Self {
        Self { name, co }
    }

    #[soxmethod]
    pub fn bool(&self) -> SoxBool {
        SoxBool::new(true)
    }

    fn slot_trace(obj: &SoxObjectRef, trace_fn: &mut dyn FnMut(SoxObjectRef)) {
        if let Some(module) = obj.payload::<SoxModule>() {
            trace_fn(SoxObjectRef::from(module.co.clone()));
        }
    }
}

impl SoxObjectPayload for SoxModule {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl TryFromSoxObject for SoxModule {
    fn try_from_sox_object(i: &mut Runtime, obj: SoxObjectRef) -> SoxResult<Self> {
        if let Some(val) = obj.payload::<SoxModule>() {
            Ok(val.clone())
        } else {
            let err_msg = SoxString {
                value: String::from("failed to get module from provided object."),
            };
            let ob = i.alloc(err_msg, string::SoxString::init_builtin_type().to_owned());
            Err(ob.into())
        }
    }
}

impl ToSoxResult for SoxModule {
    fn to_sox_result(self, i: &mut Runtime) -> SoxResult {
        let obj = i.alloc(self, i.types.mod_type.to_owned());
        Ok(SoxObjectRef::from(obj))
    }
}

impl StaticType for SoxModule {
    const NAME: &'static str = "module";

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

impl SoxModule {
    fn slot_drop(obj: &SoxObjectRef) {
        if obj.payload::<SoxModule>().is_some() {
            unsafe {
                let inner = obj.ptr.as_ptr() as *mut crate::object::core::SoxObjectInner<SoxModule>;
                std::ptr::drop_in_place(&mut (*inner).payload);
            }
        }
    }
}

impl Representable for SoxModule {
    fn repr(zelf: &Sox<Self>, _i: &Runtime) -> String {
        zelf.name.to_string()
    }
}
