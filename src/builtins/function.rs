use std::any::Any;
use std::iter::zip;
use std::ops::Deref;
use std::rc::Rc;

use once_cell::sync::OnceCell;
use slotmap::DefaultKey;

use crate::builtins::exceptions::{Exception, RuntimeError};
use crate::builtins::method::{FuncArgs, SoxMethod};
use crate::builtins::none::SoxNone;
use crate::builtins::string::SoxString;
use macros::{soxmethod, soxtype};

use crate::core::{
    SoxClassImpl, SoxObject, SoxObjectPayload, SoxRef, SoxResult, SoxType, SoxTypeSlot, StaticType,
    ToSoxResult, TryFromSoxObject,
};
use crate::environment::Namespace;
use crate::interpreter::Interpreter;
use crate::stmt::Stmt;

#[derive(Clone, Debug, PartialEq)]
pub struct SoxFunction {
    pub declaration: Box<Stmt>,
    pub environment_ref: DefaultKey,
}

impl SoxFunction {
    pub fn new(declaration: Stmt, environment_ref: DefaultKey) -> Self {
        Self {
            declaration: Box::new(declaration),
            environment_ref,
        }
    }

    pub fn arity(&self) -> usize {
        if let Stmt::Function { name, params, body } = *self.declaration.clone() {
            params.len()
        } else {
            0
        }
    }

    pub fn call(fo: SoxObject, args: FuncArgs, interpreter: &mut Interpreter) -> SoxResult {
        if let Some(fo) = fo.as_func() {
            let previous_env_ref = interpreter.active_env_ref;

            interpreter.active_env_ref = fo.environment_ref.clone();

            let mut namespace = Namespace::default();
            let mut return_value = Ok(SoxNone {}.into_ref());
            if let Stmt::Function { name, params, body } = *fo.declaration.clone() {
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
            return Err(error.into_ref());
        }
    }
}

impl SoxObjectPayload for SoxFunction {
    fn to_sox_type_value(obj: SoxObject) -> SoxRef<Self> {
        obj.as_func().unwrap()
    }

    fn to_sox_object(&self, ref_type: SoxRef<Self>) -> SoxObject {
        SoxObject::SoxFunction(ref_type)
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
    fn try_from_sox_object(i: &Interpreter, obj: SoxObject) -> SoxResult<Self> {
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
    fn to_sox_result(self, i: &Interpreter) -> SoxResult {
        let obj = self.into_ref();
        Ok(obj)
    }
}
