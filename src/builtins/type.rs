use std::collections::HashMap;

use once_cell::sync::OnceCell;

use crate::builtins::method::{FuncArgs, SoxMethod};
use crate::core::{SoxClassImpl, SoxObject, SoxRef, SoxResult, StaticType};
use crate::interpreter::Interpreter;

pub type GenericMethod = fn(SoxObject, FuncArgs, &mut Interpreter) -> SoxResult;

#[derive(Clone, Debug)]
pub struct SoxTypeSlot {
    pub call: Option<GenericMethod>,
    //pub methods: &'static [SoxMethod],
}

#[derive(Debug)]
pub struct SoxType {
    pub base: Option<SoxRef<SoxType>>,
    pub methods: HashMap<String, SoxMethod>,
    pub slots: SoxTypeSlot,
}

impl SoxType {
    pub fn new(
        base: Option<SoxRef<SoxType>>,
        methods: HashMap<String, SoxMethod>,
        slots: SoxTypeSlot,
    ) -> Self {
        Self {
            base,
            methods,
            slots,
        }
    }
}

impl StaticType for SoxType {
    const NAME: &'static str = "";

    fn static_cell() -> &'static OnceCell<SoxType> {
        todo!()
    }

    fn init_builtin_type() -> &'static SoxType
    where
        Self: SoxClassImpl,
    {
        todo!()
    }

    fn create_slots() -> SoxTypeSlot {
        todo!()
    }

    fn create_static_type() -> SoxType
    where
        Self: SoxClassImpl,
    {
        todo!()
    }
}

impl SoxClassImpl for SoxType {
    const METHOD_DEFS: &'static [(&'static str, SoxMethod)] = &[];
}

// #[derive(Clone, Debug)]
// pub struct SoxClassInstance {
//     class: SoxRef<SoxType>,
//     fields: HashMap<String, SoxObject>,
// }
//
// impl SoxClass {
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
// impl ClassInstance {
//     pub fn new(class: Rc<Class>) -> Self {
//         let fields = RefCell::new(HashMap::new());
//         Self { class, fields }
//     }
//
//     pub fn set(&self, name: Token, value: Object) {
//         self.fields.borrow_mut().insert(name.lexeme.into(), value);
//     }
//
//     pub fn get_(
//         instance: Rc<ClassInstance>,
//         name: Token,
//         interp: &mut Interpreter,
//     ) -> Result<Object, RuntimeException> {
//         let inst = instance.clone();
//         let val = if inst.fields.borrow().contains_key(name.lexeme.as_str()) {
//             Ok(inst.fields.borrow().get(name.lexeme.as_str()).unwrap().clone())
//         } else if let Some(method) = inst.class.find_method(name.lexeme.as_str()) {
//             let another_instance = instance.clone();
//             let bound_method = method.bind(Object::ClassInstance(another_instance), interp);
//             bound_method
//         } else {
//             Err(RuntimeException::RuntimeError(RuntimeError {
//                 msg: format!("Undefined property - {:?}", name.lexeme),
//             }))
//         };
//         return val;
//     }
// }
