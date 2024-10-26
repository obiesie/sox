use crate::builtins::r#type::SoxType;
use crate::builtins::{bool, exceptions, float, function, int, none, r#type, string};
use crate::core::StaticType;

#[derive(Debug)]
pub struct TypeLibrary {
    pub bool_type: &'static SoxType,
    pub float_type: &'static SoxType,
    pub int_type: &'static SoxType,
    pub str_type: &'static SoxType,
    pub none_type: &'static SoxType,
    pub exception_type: &'static SoxType,
    pub func_type: &'static SoxType,
    pub type_type: &'static SoxType,
}

impl TypeLibrary {
    pub fn init() -> Self {
        Self {
            bool_type: bool::SoxBool::init_builtin_type(),
            float_type: float::SoxFloat::init_builtin_type(),
            int_type: int::SoxInt::init_builtin_type(),
            str_type: string::SoxString::init_builtin_type(),
            none_type: none::SoxNone::init_builtin_type(),
            exception_type: exceptions::Exception::init_builtin_type(),
            func_type: function::SoxFunction::init_builtin_type(),
            type_type: r#type::SoxType::init_builtin_type(),
        }
    }
}
