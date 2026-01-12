use crate::builtins::method::SoxMethod;
use crate::builtins::r#type::{SoxType, SoxTypeSlot};
use crate::builtins::string::SoxString;
use macros::soxtype;
use once_cell::sync::OnceCell;
use std::any::Any;

use crate::builtins::core::{
    SoxClassImpl, SoxObjectPayload, SoxResult, StaticType, ToSoxResult, TryFromSoxObject,
};
use crate::interpreter::Interpreter;
use crate::object::core::SoxObjectRef;
use crate::object::core::{Sox, SoxRef};
use crate::object::protocols::repr::Representable;

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
}

impl Representable for SoxUpvalue {
    fn repr(zelf: &Sox<Self>, _i: &Interpreter) -> String {
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
            number: None,
            comparable: None,
            methods: Self::METHOD_DEFS,
        }
    }
}

impl TryFromSoxObject for SoxUpvalue {
    fn try_from_sox_object(_i: &Interpreter, obj: SoxObjectRef) -> SoxResult<Self> {
        if let Some(val) = obj.payload::<SoxUpvalue>() {
            Ok(val.clone())
        } else {
            let err_msg = SoxString {
                value: String::from("failed to get an upvalue from supplied object"),
            };
            let ob = SoxRef::new_ref(err_msg, SoxString::init_builtin_type().to_owned());
            Err(ob.into())
        }
    }
}

impl ToSoxResult for SoxUpvalue {
    fn to_sox_result(self, _i: &Interpreter) -> SoxResult {
        let obj = SoxRef::new_ref(self, Self::static_type().to_owned());
        Ok(obj.into())
    }
}
