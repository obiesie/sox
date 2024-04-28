use std::any::Any;
use std::ops::Deref;
use std::rc::Rc;

use once_cell::sync::OnceCell;
use slotmap::DefaultKey;

use macros::{soxmethod, soxtype};
use crate::builtins::method::{FuncArgs, SoxMethod};
use crate::builtins::string::SoxString;

use crate::core::{SoxClassImpl, SoxObject, SoxObjectPayload, SoxRef, SoxResult, SoxType, SoxTypeSlot, StaticType, ToSoxResult, TryFromSoxObject};
use crate::interpreter::Interpreter;
use crate::stmt::Stmt;


pub type SoxFunctionRef = Rc<SoxFunction>;

#[soxtype]
#[derive(Clone, Debug, PartialEq)]
pub struct SoxFunction {
    pub declaration: Box<Stmt>,
    pub environment_ref: DefaultKey,
}


#[soxtype]
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
   

    pub fn call(fo: SoxObject, args: FuncArgs, interpreter: &mut Interpreter ) -> SoxResult {
        todo!()
        // if let Some(fo) = fo.as_func() {
        // 
        // 
        //     let previous_env_ref = interpreter.active_env_ref;
        // 
        //     interpreter.active_env_ref = fo.environment_ref.clone();
        //     
        //     let mut namespace = Namespace::default();
        //     let mut return_value = Ok(SoxRef::new(SoxNone{}));
        //     if let Stmt::Function { name, params, body } = *fo.declaration.clone() {
        //         for (param, arg) in zip(params, args.args.clone()) {
        //             namespace.define(param.lexeme, arg, interpreter)?;
        //         }
        //         let ret = interpreter.execute_block(body.iter().collect(), Some(namespace));
        //     
        //         if ret.is_err() {
        //             let exc = ret.err();
        //             if let Some(Exception::Return(obj)) = exc {
        //                 return_value = Ok(obj);
        //             } else {
        //                 return_value = Err(exc.unwrap());
        //             }
        //         }
        //     }
        //     interpreter.active_env_ref = previous_env_ref;
        //     
        //     return_value
        // } else{
        //     
        // }
    }
}

impl SoxObjectPayload for SoxFunction {
    

    fn to_sox_type_value(obj: SoxObject) -> SoxRef<Self> {
        todo!()
    }

    fn to_sox_object(&self, ref_type: SoxRef<Self>) -> SoxObject {
        todo!()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn into_ref(self) -> SoxObject {
        SoxRef::new(self).to_sox_object()
    }


    fn class(&self, i: &Interpreter) -> &'static SoxType {
        todo!()
    }
}



impl StaticType for SoxFunction {
    const NAME: &'static str = "function";

    fn static_cell() -> &'static OnceCell<SoxType> {
        static CELL: OnceCell<SoxType> = OnceCell::new();
        &CELL
    }

    fn create_slots() -> SoxTypeSlot {
        SoxTypeSlot{
            call: Some(Self::call), 
        }
    }
}


impl TryFromSoxObject for SoxFunction{
    fn try_from_sox_object(i: &Interpreter, obj: SoxObject) -> SoxResult<Self> {
        if let Some(func) = obj.as_func(){
            Ok(func.val.deref().clone())
        } else{
            let err_msg = SoxString {
                value: String::from("failed to get function from supplied object")
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

