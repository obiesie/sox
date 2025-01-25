use once_cell::sync::OnceCell;
use std::any::Any;
use std::iter::zip;
use crate::builtins::bool::SoxBool;
use crate::builtins::exceptions::{Exception, RuntimeError};
use crate::builtins::method::{static_func, FuncArgs, SoxMethod};
use crate::builtins::r#type::{SoxInstance, SoxType, SoxTypeSlot};
use crate::builtins::string::SoxString;

use crate::builtins::core::{
    SoxClassImpl, SoxObjectPayload, SoxResult, StaticType,
    ToSoxResult, TryFromSoxObject,
};
use crate::environment::EnvRef;
use crate::interpreter::Interpreter;
use crate::object::core::{Sox, SoxObjectRef, SoxRef};
use crate::object::protocols::call::Callable;
use crate::object::protocols::repr::Representable;
use crate::stmt::Stmt;

#[derive(Clone, Debug, PartialEq)]
pub struct SoxFunction {
    pub name: String,
    pub declaration: Box<Stmt>,
    pub environment_ref: EnvRef,
    pub is_initializer: bool,
    pub arity: i8,
}

impl SoxFunction {
    pub fn new(name: String, declaration: Stmt, environment_ref: EnvRef, arity: i8, is_initializer: bool) -> Self {
        Self {
            name,
            declaration: Box::new(declaration),
            environment_ref,
            is_initializer,
            arity,
        }
    }

    pub fn bind(&self, instance: SoxObjectRef, interp: &mut Interpreter) -> SoxResult {
        if let Some(_) = instance.payload::<SoxInstance>() {

            let env_ref = interp
                .environment
                .new_local_env_at(self.environment_ref.clone());
            interp
                .environment
                .define_at("this", instance, env_ref.clone());

            let new_func = SoxFunction {
                name: self.name.to_string(),
                declaration: self.declaration.clone(),
                environment_ref: env_ref,
                is_initializer: self.is_initializer,
                arity: self.arity,
            };
            Ok(SoxObjectRef::from(SoxRef::new_ref(new_func, interp.types.func_type.to_owned())))
        } else {
            Err(Interpreter::runtime_error(interp,
                "Could not bind method to instance".to_string(),
            ))
        }
    }



    pub fn equals(&self, other: &SoxObjectRef) -> SoxBool {
        if let Some(other_func) = other.payload::<SoxFunction>() {
            SoxBool::from(self.name == other_func.name
                && self.declaration == other_func.declaration
                && self.environment_ref == other_func.environment_ref
                && self.is_initializer == other_func.is_initializer
                && self.arity == other_func.arity)
        } else {
            SoxBool::from(false)
        }
    }
}

impl SoxObjectPayload for SoxFunction {
    
    fn as_any(&self) -> &dyn Any {
        self
    }


}

impl SoxClassImpl for SoxFunction {
    const METHOD_DEFS: &'static [(&'static str, SoxMethod)] = &[  (
        "equals",
        SoxMethod {
            func: static_func(SoxBool::equals),
        },
    )];
}

impl StaticType for SoxFunction {
    const NAME: &'static str = "function";

    fn static_cell() -> &'static OnceCell<SoxRef<SoxType>> {
        static CELL: OnceCell<SoxRef<SoxType>> = OnceCell::new();
        &CELL
    }

    fn create_slots() -> SoxTypeSlot {
        SoxTypeSlot {
            call: Some(Self::slot_call),
            repr: Some(Self::slot_repr),
            number: None,
            comparable: None,
            methods: Self::METHOD_DEFS,

        }
    }
}

impl TryFromSoxObject for SoxFunction {
    fn try_from_sox_object(i: &Interpreter, obj: SoxObjectRef) -> SoxResult<Self> {
        if let Some(func) = obj.payload::<SoxFunction>() {
            Ok(func.clone())
        } else {
            let err_msg = SoxString {
                value: String::from("failed to get function from supplied object"),
            };
            let ob = SoxRef::new_ref(err_msg, i.types.str_type.to_owned());
            Err(SoxObjectRef::from(ob))
        }
    }
}

impl ToSoxResult for SoxFunction {
    fn to_sox_result(self, i: &Interpreter) -> SoxResult {
        let obj = SoxObjectRef::from(SoxRef::new_ref(self, i.types.func_type.to_owned()));
        Ok(obj)
    }
}

impl Representable for SoxFunction {
    fn repr(zelf: &Sox<Self>, _i: &Interpreter) -> String {
        let func_name = zelf.name.to_string();
        format!("<Function {func_name}>")
    }
}

impl Callable for SoxFunction {
    fn call(zelf: &Sox<Self>, args: FuncArgs, i: &mut Interpreter) -> SoxResult {
        if args.args.len() != zelf.arity as usize {
            let error = Exception::Err(RuntimeError {
                msg: format!(
                    "Expected {} arguments but got {}.",
                    zelf.arity,
                    args.args.len()
                ),
            });

            return Err(SoxObjectRef::from(SoxRef::new_ref(error, i.types.exception_type.to_owned())));
        }
        let previous_env_ref = i.environment.active.clone();
        i.environment.active = zelf.environment_ref.clone();

        let mut return_value = Ok(SoxObjectRef::from(i.none.clone()));
        if let Stmt::Function {
            name: _,
            params,
            body,
        } = *zelf.declaration.clone()
        {
            let exec_ns = i
                .environment
                .new_local_env_at(zelf.environment_ref.clone());
            let env = i.environment.envs.get_mut(*exec_ns).unwrap();
            for (param, arg) in zip(params, args.args.clone()) {
                env.define(param.lexeme, arg).expect("TODO: panic message");
            }
            let ret = i.execute_block(body.iter().collect(), Option::from(exec_ns));

            if ret.is_err() {
                let error = ret.unwrap_err();
                let exc = error.payload::<Exception>();
                if let Some(obj) = exc {
                    match obj {
                        Exception::Return(v) => {
                            let val = v.clone();
                            return_value = Ok(val);
                        }
                        Exception::Err(v) => {
                            let rv = Exception::Err(v.clone());
                            return_value = Err(SoxObjectRef::from(SoxRef::new_ref(rv, i.types.exception_type.to_owned())));
                        }
                    }
                }
            }
        }
        if zelf.is_initializer {

            let v = i.environment.find_and_get( "this");
            i.environment.active = previous_env_ref;
            return v;

        }
        i.environment.active = previous_env_ref;

        return_value
    }
}