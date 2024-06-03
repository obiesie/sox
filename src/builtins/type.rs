use std::any::Any;
use std::collections::HashMap;

use crate::builtins::exceptions::{Exception, RuntimeError};
use crate::builtins::function::SoxFunction;
use once_cell::sync::OnceCell;

use crate::builtins::method::{FuncArgs, SoxMethod};
use crate::core::{SoxClassImpl, SoxObject, SoxObjectPayload, SoxRef, SoxResult, StaticType};
use crate::interpreter::Interpreter;
use crate::token::Token;

pub type GenericMethod = fn(SoxObject, FuncArgs, &mut Interpreter) -> SoxResult;

#[derive(Clone, Debug, Default)]
pub struct SoxTypeSlot {
    pub call: Option<GenericMethod>,
    //pub methods: &'static [SoxMethod],
}

pub type SoxAttributes = HashMap<String, SoxObject>;

#[derive(Debug)]
pub struct SoxType {
    pub base: Option<SoxRef<SoxType>>,
    pub methods: HashMap<String, SoxMethod>,
    pub slots: SoxTypeSlot,
    pub attributes: SoxAttributes,
}

impl SoxType {
    pub fn new(
        name: String,
        base: Option<SoxRef<SoxType>>,
        methods: HashMap<String, SoxMethod>,
        slots: SoxTypeSlot,
        attributes: SoxAttributes,
    ) -> Self {
        Self {
            base,
            methods,
            slots,
            attributes,
        }
    }

    pub fn find_method(&self, name: &str) -> Option<SoxObject> {
        let method = if let Some(m) = self.attributes.get(name) {
            Some(m.clone())
        } else {
            None
        };
        if method.is_none() && self.base.is_some() {
            self.base.as_ref().unwrap().find_method(name)
        } else {
            method
        }
    }

    pub fn call(fo: SoxObject, args: FuncArgs, interpreter: &mut Interpreter) -> SoxResult {
        if let Some(to) = fo.as_type() {
            let class_instance = SoxClassInstance::new(to.clone());
            let initializer = to.find_method("init".into());
            let instance = class_instance.into_ref(); //SoxObject::ClassInstance(Rc::new(class_instance));
            let ret_val = if let Some(init_func) = initializer {
                let func = init_func.as_func().expect("Non function found as init");
                let bound_method = func.bind(instance.clone(), interpreter)?;
                SoxFunction::call(bound_method, args, interpreter)?;
                Ok(instance)
            } else {
                Ok(instance)
            };
            ret_val
        } else {
            let error = Exception::Err(RuntimeError {
                msg: "first argument to this call method should be a type object".to_string(),
            });
            return Err(error.into_ref());
        }
    }
}
impl SoxObjectPayload for SoxType {
    fn to_sox_type_value(obj: SoxObject) -> SoxRef<Self> {
        todo!()
    }

    fn to_sox_object(&self, ref_type: SoxRef<Self>) -> SoxObject {
        todo!()
    }

    fn as_any(&self) -> &dyn Any {
        todo!()
    }

    fn class(&self, i: &Interpreter) -> &'static SoxType {
        todo!()
    }
}
impl StaticType for SoxType {
    const NAME: &'static str = "type";

    fn static_cell() -> &'static OnceCell<SoxType> {
        static CELL: OnceCell<SoxType> = OnceCell::new();
        &CELL
    }

    fn create_slots() -> SoxTypeSlot {
        SoxTypeSlot { call: None }
    }
}

impl SoxClassImpl for SoxType {
    const METHOD_DEFS: &'static [(&'static str, SoxMethod)] = &[];
}

#[derive(Clone, Debug)]
pub struct SoxClassInstance {
    class: SoxRef<SoxType>,
    fields: HashMap<String, SoxObject>,
}

//impl SoxClass {
//     pub fn new(
//         name: String,
//         methods: HashMap<String, SoxMethod>,
//         superclass: Option<SoxRef<SoxType>>,
//     ) -> Self {
//         Self {
//             name,
//             methods,
//             superclass,
//         }
//     }
//
//     pub fn find_method(&self, name: &str) -> Option<Rc<Function>> {
//         let method = if let Some(m) = self.methods.get(name) {
//             Some(m.clone())
//         } else {
//             None
//         };
//         if method.is_none() && self.superclass.is_some() {
//             self.superclass.as_ref().unwrap().find_method(name)
//         } else {
//             method
//         }
//     }
//
//     pub fn arity(&self) -> usize {
//         let initializer = self.find_method("init".into());
//         let ret_val = if let Some(init_func) = initializer {
//             let func = init_func;
//             func.arity()
//         } else {
//             0
//         };
//         ret_val
//     }
// }
// impl Callable for Class {
//     type T = Result<Object, RuntimeException>;
//
//     fn call(&self, interpreter: &mut Interpreter, args: Vec<Object>) -> Self::T {
//         let class_instance = ClassInstance::new(Rc::new(self.to_owned()));
//         let initializer = self.find_method("init".into());
//         let instance = Object::ClassInstance(Rc::new(class_instance));
//         let ret_val = if let Some(init_func) = initializer {
//             let mut func = init_func;
//             let bound_method = func.bind(instance.clone(), interpreter)?;
//             if let Object::Function(f) = bound_method {
//                 let mut func = f;
//                 func.call(interpreter, args)?;
//                 Ok(instance)
//             } else {
//                 Err(RuntimeException::RuntimeError(RuntimeError {
//                     msg: format!("Initializer found is not a callable function."),
//                 }))
//             }
//         } else {
//             Ok(instance)
//         };
//         ret_val
//     }
// }
//
impl SoxClassInstance {
    pub fn new(class: SoxRef<SoxType>) -> Self {
        let fields = HashMap::new();
        Self { class, fields }
    }

    pub fn set(&mut self, name: Token, value: SoxObject) {
        self.fields.insert(name.lexeme.into(), value);
    }

    pub fn get_(instance: SoxClassInstance, name: Token, interp: &mut Interpreter) -> SoxResult {
        todo!()
        // let inst = instance.clone();
        // let val = if inst.fields.contains_key(name.lexeme.as_str()) {
        //     Ok(inst.fields.get(name.lexeme.as_str()).unwrap().clone())
        // } else if let Some(method) = inst.class.find_method(name.lexeme.as_str()) {
        //     let another_instance = SoxRef::new(instance.clone());
        //     let bound_method = method.bind(SoxObject::ClassInstance(another_instance), interp);
        //     bound_method
        // } else {
        //     Err(RuntimeException::RuntimeError(RuntimeError {
        //         msg: format!("Undefined property - {:?}", name.lexeme),
        //     }))
        // };
        // return val;
    }
}

impl SoxObjectPayload for SoxClassInstance {
    fn to_sox_type_value(obj: SoxObject) -> SoxRef<Self> {
        todo!()
    }

    fn to_sox_object(&self, ref_type: SoxRef<Self>) -> SoxObject {
        todo!()
    }

    fn as_any(&self) -> &dyn Any {
        todo!()
    }

    fn class(&self, i: &Interpreter) -> &'static SoxType {
        todo!()
    }
}
