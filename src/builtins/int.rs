use std::any::Any;
use std::rc::Rc;
use once_cell::sync::OnceCell;
use macros::{soxmethod, soxtype};
use crate::builtins::bool::SoxBool;
use crate::builtins::exceptions::{Exception, RuntimeError};
use crate::builtins::{exceptions, int, string};
use crate::builtins::method::{static_func, SoxMethod};
use crate::builtins::r#type::{SoxType, SoxTypeSlot};
use crate::builtins::string::SoxString;
use crate::builtins::core::{SoxClassImpl, SoxObjectPayload, SoxResult, StaticType, ToSoxResult, TryFromSoxObject};
use crate::interpreter::Interpreter;
use crate::object::core::{Sox, SoxObjectRef, SoxRef};
use crate::slots::repr::Representable;

pub type SoxIntRef = Rc<SoxInt>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SoxInt {
    pub value: i64,
}

#[soxtype]
impl SoxInt {
    pub fn new(val: i64) -> Self {
        SoxInt { value: val }
    }

    #[soxmethod]
    pub fn equals(&self, rhs: SoxObjectRef) -> SoxBool {
        if let Some(rhs_int) = rhs.payload::<SoxInt>() {
            SoxBool::new(self.value == rhs_int.value)
        } else {
            SoxBool::new(false)
        }
    }
    
    #[soxmethod]
    pub fn add(&self, rhs: SoxObjectRef) -> SoxObjectRef {
        if let Some(rhs_int) = rhs.payload::<SoxInt>() {
            let new_int = SoxInt::new(self.value + rhs_int.value);
            SoxRef::new_ref(new_int, int::SoxInt::init_builtin_type().to_owned()).into()
        } else {
            let err_msg = "+ operand not supported for both types".to_string(); 
            let runtime_err = RuntimeError{
                msg: err_msg
            };
            let exc: Exception = runtime_err.try_into().unwrap();
            let obj = SoxRef::new_ref(exc, exceptions::Exception::init_builtin_type().to_owned());
            obj.into()
        }
    }
}

impl SoxObjectPayload for SoxInt {
    
    fn as_any(&self) -> &dyn Any {
        self
    }


}

impl StaticType for SoxInt {
    const NAME: &'static str = "int";

    fn static_cell() -> &'static OnceCell<SoxRef<SoxType>> {
        static CELL:OnceCell<SoxRef<SoxType>> = OnceCell::new();
        &CELL
    }

    fn create_slots() -> SoxTypeSlot {
        SoxTypeSlot {
            call: None,
            repr: Some(Self::slot_repr),
            methods: Self::METHOD_DEFS,
        }
    }
}


impl TryFromSoxObject for SoxInt {
    fn try_from_sox_object(_i: &Interpreter, obj: SoxObjectRef) -> SoxResult<Self> {
        if let Some(val) = obj.payload::<SoxInt>() {
            Ok(val.clone())
        } else {
            let err_msg = SoxString {
                value: String::from("failed to get boolean from supplied object"),
            };
            let ob = SoxRef::new_ref(err_msg, string::SoxString::init_builtin_type().to_owned());
            Err(ob.into())
        }
    }
}

impl ToSoxResult for SoxInt {
    fn to_sox_result(self, _i: &Interpreter) -> SoxResult {
        let obj = SoxRef::new_ref(self, Self::init_builtin_type().to_owned());
        Ok(obj.into())
    }
}


impl From<i64> for SoxInt {
    fn from(i: i64) -> Self {
        let v = Self{ value: i};
        v
    }
}

impl Representable for SoxInt {
    fn repr(zelf: &Sox<Self>, _i: &Interpreter) -> String {
        zelf.value.to_string()
    }
}

