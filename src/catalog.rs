use crate::builtins::core::StaticType;
use crate::builtins::r#type::{SoxInstance, SoxType};
use crate::builtins::{bool, exceptions, float, function, int, module, none, r#type, string};
use crate::object::core::{init_type_type, Sox, SoxObjectRef};

pub struct TypeLibrary {
    pub bool_type: &'static Sox<SoxType>,
    pub float_type: &'static Sox<SoxType>,
    pub int_type: &'static Sox<SoxType>,
    pub str_type: &'static Sox<SoxType>,
    pub none_type: &'static Sox<SoxType>,
    pub exception_type: &'static Sox<SoxType>,
    pub func_type: &'static Sox<SoxType>,
    pub function_type: &'static Sox<SoxType>,
    pub type_type: &'static Sox<SoxType>,
    pub obj_type: &'static Sox<SoxType>,
    pub mod_type: &'static Sox<SoxType>,
}

impl TypeLibrary {
    pub fn init() -> Self {
        let type_type = init_type_type();
        Self {
            type_type: r#type::SoxType::init_manually(type_type),
            obj_type: SoxInstance::init_builtin_type(),
            bool_type: bool::SoxBool::init_builtin_type(),
            float_type: float::SoxFloat::init_builtin_type(),
            int_type: int::SoxInt::init_builtin_type(),
            str_type: string::SoxString::init_builtin_type(),
            none_type: none::SoxNone::init_builtin_type(),
            exception_type: exceptions::Exception::init_builtin_type(),
            func_type: function::SoxFunction::init_builtin_type(),
            function_type: function::SoxFunc::init_builtin_type(),
            mod_type: module::SoxModule::init_builtin_type(),
        }
    }
}

pub struct ExceptionLibrary {
    pub visit_block_stmt_error: SoxObjectRef,
}
