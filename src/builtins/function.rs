use once_cell::sync::OnceCell;
use std::any::Any;
use std::iter::zip;
use std::ops::Deref;
use crate::builtins::bool::SoxBool;
use crate::builtins::exceptions::{Exception, RuntimeError};
use crate::builtins::method::{static_func, FuncArgs, SoxMethod};
use crate::builtins::none::SoxNone;
use crate::builtins::r#type::{SoxType, SoxTypeSlot};
use crate::builtins::string::SoxString;

use crate::core::{
    Representable, SoxClassImpl, SoxObject, SoxObjectPayload, SoxRef, SoxResult, StaticType,
    ToSoxResult, TryFromSoxObject,
};
use crate::environment::EnvRef;
use crate::interpreter::Interpreter;
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

    pub fn bind(&self, instance: SoxObject, interp: &mut Interpreter) -> SoxResult {
        if let SoxObject::TypeInstance(_) = instance {
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
            Ok(new_func.into_ref())
        } else {
            Err(Interpreter::runtime_error(
                "Could not bind method to instance".to_string(),
            ))
        }
    }

    pub fn call(fo: SoxObject, args: FuncArgs, interpreter: &mut Interpreter) -> SoxResult {
        if let Some(fo) = fo.as_func() {
            if args.args.len() != fo.arity as usize {
                let error = Exception::Err(RuntimeError {
                    msg: format!(
                        "Expected {} arguments but got {}.",
                        fo.arity,
                        args.args.len()
                    ),
                });
                return Err(error.into_ref());
            }
            let previous_env_ref = interpreter.environment.active.clone();

            interpreter.environment.active = fo.environment_ref.clone();
            let mut return_value = Ok(SoxNone {}.into_ref());
            if let Stmt::Function {
                name: _,
                params,
                body,
            } = *fo.declaration.clone()
            {
                let exec_ns = interpreter
                    .environment
                    .new_local_env_at(fo.environment_ref.clone());
                let env = interpreter.environment.envs.get_mut(*exec_ns).unwrap();
                for (param, arg) in zip(params, args.args.clone()) {
                    env.define(param.lexeme, arg).expect("TODO: panic message");
                }
                let ret = interpreter.execute_block(body.iter().collect(), Option::from(exec_ns));

                if ret.is_err() {
                    let exc = ret.err().unwrap().as_exception();
                    if let Some(obj) = exc {
                        match obj.deref() {
                            Exception::Return(v) => {
                                return_value = Ok(v.clone());
                            }
                            Exception::Err(v) => {
                                let rv = Exception::Err(v.clone());
                                return_value = Err(rv.into_ref());
                            }
                        }
                    }
                }
            }
            if fo.is_initializer {

                let v = interpreter.environment.find_and_get( "this");
                interpreter.environment.active = previous_env_ref;
                return v;

            }
            interpreter.environment.active = previous_env_ref;
           
            return_value
        } else {
            let error = Exception::Err(RuntimeError {
                msg: "first argument to this call method should be a function object".to_string(),
            });
            Err(error.into_ref())
        }
    }

    pub fn equals(&self, other: &SoxObject) -> SoxBool {
        if let Some(other_func) = other.as_func() {
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
    fn to_sox_type_value(obj: SoxObject) -> SoxRef<Self> {
        obj.as_func().unwrap()
    }

    fn to_sox_object(&self, ref_type: SoxRef<Self>) -> SoxObject {
        SoxObject::Function(ref_type)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn into_ref(self) -> SoxObject {
        SoxRef::new(self).to_sox_object()
    }

    fn class(&self, i: &Interpreter) -> &'static SoxType {
        i.types.func_type
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

    fn static_cell() -> &'static OnceCell<SoxType> {
        static CELL: OnceCell<SoxType> = OnceCell::new();
        &CELL
    }

    fn create_slots() -> SoxTypeSlot {
        SoxTypeSlot {
            call: Some(Self::call),
            //eq: None
            methods: Self::METHOD_DEFS,

        }
    }
}

impl TryFromSoxObject for SoxFunction {
    fn try_from_sox_object(_i: &Interpreter, obj: SoxObject) -> SoxResult<Self> {
        if let Some(func) = obj.as_func() {
            Ok(func.val.deref().clone())
        } else {
            let err_msg = SoxString {
                value: String::from("failed to get function from supplied object"),
            };
            let ob = SoxRef::new(err_msg);
            Err(SoxObject::String(ob))
        }
    }
}

impl ToSoxResult for SoxFunction {
    fn to_sox_result(self, _i: &Interpreter) -> SoxResult {
        let obj = self.into_ref();
        Ok(obj)
    }
}

impl Representable for SoxFunction {
    fn repr(&self, _i: &Interpreter) -> String {
        let func_name = self.name.to_string();
        format!("<Function {func_name}>")
    }
}
