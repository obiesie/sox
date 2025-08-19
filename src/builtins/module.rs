use crate::builtins::bool::SoxBool;
use crate::builtins::core::{
    SoxClassImpl, SoxObjectPayload, SoxResult, StaticType, ToSoxResult, TryFromSoxObject,
};
use crate::builtins::method::{static_func, SoxMethod};
use crate::builtins::r#type::{SoxType, SoxTypeSlot};
use crate::builtins::string;
use crate::builtins::string::SoxString;
use crate::interpreter::Interpreter;
use crate::object::core::{Sox, SoxObjectRef, SoxRef};
use crate::object::protocols::repr::Representable;
use crate::vm::chunk::Chunk;
use crate::vm::compiler::Compilable;
use macros::{soxmethod, soxtype};
use once_cell::sync::OnceCell;
use std::any::Any;

#[derive(Debug, Clone)]
pub struct SoxModule {
    name: String,
    pub co: Chunk,
}

#[soxtype]
impl SoxModule {
    pub fn new(name: String, co: Option<Chunk>) -> Self {
        if let Some(co) = co {
            Self { name, co }
        } else {
            Self {
                name,
                co: Chunk::new(),
            }
        }
    }

    #[soxmethod]
    pub fn bool(&self) -> SoxBool {
        SoxBool::new(true)
    }
}

impl Compilable for SoxModule {
    fn new_empty(name: String) -> Self {
        Self::new(name, None)
    }

    fn new(name: String, co: Chunk) -> Self {
        Self::new(name, Some(co))
    }
}
impl SoxObjectPayload for SoxModule {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl TryFromSoxObject for SoxModule {
    fn try_from_sox_object(_i: &Interpreter, obj: SoxObjectRef) -> SoxResult<Self> {
        if let Some(val) = obj.payload::<SoxModule>() {
            Ok(val.clone())
        } else {
            let err_msg = SoxString {
                value: String::from("failed to get module from provided object."),
            };
            let ob = SoxRef::new_ref(err_msg, string::SoxString::init_builtin_type().to_owned());
            Err(ob.into())
        }
    }
}

impl ToSoxResult for SoxModule {
    fn to_sox_result(self, i: &Interpreter) -> SoxResult {
        let obj = SoxRef::new_ref(self, i.types.mod_type.to_owned());
        Ok(obj.into())
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
            number: None,
            comparable: None,
            methods: Self::METHOD_DEFS,
        }
    }
}

impl Representable for SoxModule {
    fn repr(zelf: &Sox<Self>, _i: &Interpreter) -> String {
        zelf.name.to_string()
    }
}
