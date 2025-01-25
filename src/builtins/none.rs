use std::any::Any;
use crate::builtins::bool::SoxBool;
use crate::builtins::method::{static_func, SoxMethod};
use crate::builtins::r#type::{SoxType, SoxTypeSlot};
use crate::builtins::core::{SoxClassImpl, SoxObjectPayload, SoxResult, StaticType, ToSoxResult, TryFromSoxObject};
use crate::interpreter::Interpreter;
use once_cell::sync::OnceCell;
use macros::{soxmethod, soxtype};
use crate::builtins::string::SoxString;
use crate::object::core::{Sox, SoxObjectRef, SoxRef};
use crate::object::protocols::repr::Representable;

#[derive(Debug, Clone, Copy)]
pub struct SoxNone;

#[soxtype]
impl SoxNone {
    
    #[soxmethod]
    pub fn bool(&self) -> SoxBool {
        SoxBool::new(false)
    }

    #[soxmethod]
    pub fn equals(&self, rhs: SoxObjectRef) -> SoxBool {
        let other = rhs.payload::<SoxNone>();
        match other {
            Some(_) => SoxBool::new(true),
            None => SoxBool::new(false),
        }
    }
}


impl SoxObjectPayload for SoxNone {

   
    fn as_any(&self) -> &dyn Any {
        self
    }
    
}

impl StaticType for SoxNone {
    const NAME: &'static str = "none";

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


impl TryFromSoxObject for SoxNone {
    fn try_from_sox_object(i: &Interpreter, obj: SoxObjectRef) -> SoxResult<Self> {
        if let Some(val) = obj.payload::<SoxNone>() {
            Ok(val.clone())
        } else {
            let err_msg = SoxString {
                value: String::from("failed to get boolean from supplied object"),
            };
            let ob = SoxRef::new_ref(err_msg, i.types.none_type.to_owned());
            Err(ob.into())
        }
    }
}

impl ToSoxResult for SoxNone {
    fn to_sox_result(self, i: &Interpreter) -> SoxResult {
        let obj = SoxRef::new_ref(self, i.types.none_type.to_owned());
        Ok(obj.into())
    }
}



impl Representable for SoxNone {
    fn repr(zelf: &Sox<Self>, _i: &Interpreter) -> String {
        "None".to_string()
    }
}
