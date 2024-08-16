use std::any::Any;
use std::iter::zip;
use std::ops::Deref;

use once_cell::sync::OnceCell;
use slotmap::DefaultKey;

use crate::builtins::exceptions::{Exception, RuntimeError};
use crate::builtins::method::{FuncArgs, SoxMethod};
use crate::builtins::none::SoxNone;
use crate::builtins::r#type::{SoxType, SoxTypeSlot};
use crate::builtins::string::SoxString;

use crate::core::{
    SoxClassImpl, SoxObject, SoxObjectPayload, SoxRef, SoxResult, StaticType, ToSoxResult,
    TryFromSoxObject,
};
use crate::environment::Namespace;
use crate::interpreter::Interpreter;
use crate::stmt::Stmt;

#[derive(Clone, Debug, PartialEq)]
pub struct SoxFunction {
    pub declaration: Box<Stmt>,
    pub environment_ref: DefaultKey,
    pub is_initializer: bool,
}

impl SoxFunction {
    pub fn new(declaration: Stmt, environment_ref: DefaultKey) -> Self {
        Self {
            declaration: Box::new(declaration),
            environment_ref,
            is_initializer: false,
        }
    }

    pub fn bind(&self, instance: SoxObject, interp: &mut Interpreter) -> SoxResult {
        if let SoxObject::TypeInstance(_) = instance {
            let environment = interp.referenced_env(self.environment_ref); //ref_env!(interp, self.environment_ref);
            let mut new_env = environment.clone();
            let namespace = Namespace::default();
            new_env
                .push(namespace)
                .expect("Failed to push namespace into env.");
            new_env.define("this", instance);

            let env_ref = interp.envs.insert(new_env);
            let new_func = SoxFunction {
                declaration: self.declaration.clone(),
                environment_ref: env_ref,
                is_initializer: false,
            };
            return Ok(new_func.into_ref());
        } else {
            Err(Interpreter::runtime_error(
                "Could not bind method to instance".to_string(),
            ))
        }
    }

    pub fn call(fo: SoxObject, args: FuncArgs, interpreter: &mut Interpreter) -> SoxResult {
        if let Some(fo) = fo.as_func() {
            let previous_env_ref = interpreter.active_env_ref;

            interpreter.active_env_ref = fo.environment_ref.clone();

            let mut namespace = Namespace::default();
            let mut return_value = Ok(SoxNone {}.into_ref());
            if let Stmt::Function {
                name: _,
                params,
                body,
            } = *fo.declaration.clone()
            {
                for (param, arg) in zip(params, args.args.clone()) {
                    namespace.define(param.lexeme, arg)?;
                }
                let ret = interpreter.execute_block(body.iter().collect(), Some(namespace));

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
            interpreter.active_env_ref = previous_env_ref;

            return_value
        } else {
            let error = Exception::Err(RuntimeError {
                msg: "first argument to this call method should be a function object".to_string(),
            });
            Err(error.into_ref())
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
    const METHOD_DEFS: &'static [(&'static str, SoxMethod)] = &[];
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
