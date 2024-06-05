use std::any::Any;
use std::collections::HashMap;

use once_cell::sync::OnceCell;

use crate::builtins::exceptions::{Exception, RuntimeError};
use crate::builtins::function::SoxFunction;
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
        self.attributes.get(name)
            .cloned()
            .or_else(|| {
                self.base.as_ref().and_then(|base| base.find_method(name))
            })
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
        obj.as_type().unwrap()
    }

    fn to_sox_object(&self, ref_type: SoxRef<Self>) -> SoxObject {
        SoxObject::Class(ref_type)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn into_ref(self) -> SoxObject {
        SoxRef::new(self).to_sox_object()
    }

    fn class(&self, i: &Interpreter) -> &'static SoxType {
        i.types.type_type
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


impl SoxClassInstance {
    pub fn new(class: SoxRef<SoxType>) -> Self {
        let fields = HashMap::new();
        Self { class, fields }
    }

    pub fn set(&mut self, name: Token, value: SoxObject) {
        self.fields.insert(name.lexeme.into(), value);
    }

    pub fn get(inst: SoxRef<SoxClassInstance>, name: Token, interp: &mut Interpreter) -> SoxResult {
        if let Some(field_value) = inst.fields.get(name.lexeme.as_str()) {
            return Ok(field_value.clone());
        }

        if let Some(method) = inst.class.find_method(name.lexeme.as_str()) {
            if let Some(func) = method.as_func() {
                let bound_method = func.bind(SoxObject::ClassInstance(inst.clone()), interp);
                return bound_method;
            } else {
                return Err(Interpreter::runtime_error(
                    format!("Found property with same name, {}, but it is not a function", name.lexeme),
                ));
            }
        }

        Err(Interpreter::runtime_error(
            format!("Undefined property - {}", name.lexeme),
        ))
    }
}

impl SoxObjectPayload for SoxClassInstance {
    fn to_sox_type_value(obj: SoxObject) -> SoxRef<Self> {
        obj.as_class_instance().unwrap()
    }

    fn to_sox_object(&self, ref_type: SoxRef<Self>) -> SoxObject {
        SoxObject::ClassInstance(ref_type)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn into_ref(self) -> SoxObject {
        SoxRef::new(self).to_sox_object()
    }

    fn class(&self, i: &Interpreter) -> &'static SoxType {
        i.types.type_type
    }
}
