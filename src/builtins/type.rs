use std::any::Any;
use std::cell::RefCell;
use std::collections::HashMap;

use once_cell::sync::OnceCell;

use crate::builtins::exceptions::{Exception, RuntimeError};
use crate::builtins::function::SoxFunction;
use crate::builtins::method::{FuncArgs, SoxMethod};
use crate::core::{
    Representable, SoxClassImpl, SoxObject, SoxObjectPayload, SoxRef, SoxResult, StaticType,
};
use crate::interpreter::Interpreter;
use crate::token::Token;

pub type GenericMethod = fn(SoxObject, FuncArgs, &mut Interpreter) -> SoxResult;

#[derive(Clone, Debug, Default)]
pub struct SoxTypeSlot {
    pub call: Option<GenericMethod>,
    pub methods: &'static [(&'static str, SoxMethod)],

    //pub eq: Option<GenericMethod>
}

pub type SoxAttributes = HashMap<String, SoxObject>;

#[derive(Debug)]
pub struct SoxType {
    pub base: Option<SoxRef<SoxType>>,
    pub methods: HashMap<String, SoxMethod>,
    pub slots: SoxTypeSlot,
    pub attributes: SoxAttributes,
    pub name: Option<String>,
}


impl SoxType {
    pub fn new_static_type<T: ToString>(
        name: T,
        base: Option<SoxRef<SoxType>>,
        methods: HashMap<String, SoxMethod>,
        slots: SoxTypeSlot,
        attributes: SoxAttributes,
    ) -> Self {
        let typ = Self {
            base,
            methods,
            slots,
            attributes,
            name: Some(name.to_string()),
        };
        typ
    }

    pub fn new<T: ToString>(
        name: T,
        base: Option<SoxRef<SoxType>>,
        methods: HashMap<String, SoxMethod>,
        slots: SoxTypeSlot,
        attributes: SoxAttributes,
    ) -> Self {
        let typ = Self {
            base,
            methods,
            slots,
            attributes,
            name: Some(name.to_string()),
        };
        typ
    }

    pub fn arity(&self) -> i32 {
        let init_method = self.find_method("init".into());
        if init_method.is_none(){
            return 0;
        }
        return init_method.unwrap().as_func().unwrap().arity as i32;
    }

    pub fn find_method(&self, name: &str) -> Option<SoxObject> {
        self.attributes
            .get(name)
            .cloned()
            .or_else(|| self.base.as_ref().and_then(|base| base.find_method(name)))
    }

    pub fn call(fo: SoxObject, args: FuncArgs, interpreter: &mut Interpreter) -> SoxResult {
        
        if let Some(to) = fo.as_type() {
            if (args.args.len() != to.arity() as usize) {
                let error = Exception::Err(RuntimeError {
                    msg: format!(
                        "Expected {} arguments but got {}.",
                        to.arity(),
                        args.args.len()
                    ),
                });
                return Err(error.into_ref());
            }
            let instance = SoxInstance::new(to.clone());
            let initializer = to.find_method("init".into());
            let instance = instance.into_ref();
            let ret_val = if let Some(init_func) = initializer {
                let func = init_func
                    .as_func()
                    .expect("init resolved to a non function object");
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
            Err(error.into_ref())
        }
    }
}

impl Representable for SoxType {
    fn repr(&self, i: &Interpreter) -> String {
        format!("<type '{}'>", self.name.as_ref().unwrap().to_string())
    }
}
impl SoxObjectPayload for SoxType {
    fn to_sox_type_value(obj: SoxObject) -> SoxRef<Self> {
        obj.as_type().unwrap()
    }

    fn to_sox_object(&self, ref_type: SoxRef<Self>) -> SoxObject {
        SoxObject::Type(ref_type)
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
        SoxTypeSlot {
            call: Some(Self::call),
            methods: Self::METHOD_DEFS,
        }
    }
}

impl SoxClassImpl for SoxType {
    const METHOD_DEFS: &'static [(&'static str, SoxMethod)] = &[];
}

#[derive(Clone, Debug)]
pub struct SoxInstance {
    typ: SoxRef<SoxType>,
    fields: RefCell<HashMap<String, SoxObject>>,
}

impl SoxInstance {
    pub fn new(class: SoxRef<SoxType>) -> Self {
        let fields = HashMap::new();
        Self {
            typ: class,
            fields: RefCell::new(fields),
        }
    }

    pub fn set(&self, name: Token, value: SoxObject) {
        self.fields.borrow_mut().insert(name.lexeme.into(), value);
    }


    pub fn get(inst: SoxRef<SoxInstance>, name: Token, interp: &mut Interpreter) -> SoxResult {
        if let Some(field_value) = inst.fields.borrow().get(name.lexeme.as_str()) {
            return Ok(field_value.clone());
        }

        if let Some(method) = inst.typ.find_method(name.lexeme.as_str()) {
            if let Some(func) = method.as_func() {
                let bound_method = func.bind(SoxObject::TypeInstance(inst.clone()), interp);
                return bound_method;
            } else {
                return Err(Interpreter::runtime_error(format!(
                    "Found property with same name, {}, but it is not a function",
                    name.lexeme
                )));
            }
        }

        Err(Interpreter::runtime_error(format!(
            "Undefined property - {}",
            name.lexeme
        )))
    }
}

impl Representable for SoxInstance {
    fn repr(&self, i: &Interpreter) -> String {
        format!(
            "<{} instance>",
            self.typ
                .name
                .as_ref()
                .unwrap_or(&"Unknown type".to_string())
                .to_string()
        )
    }
}
impl SoxObjectPayload for SoxInstance {
    fn to_sox_type_value(obj: SoxObject) -> SoxRef<Self> {
        obj.as_class_instance().unwrap()
    }

    fn to_sox_object(&self, ref_type: SoxRef<Self>) -> SoxObject {
        SoxObject::TypeInstance(ref_type)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn into_ref(self) -> SoxObject {
        SoxRef::new(self).to_sox_object()
    }

    fn class(&self, i: &Interpreter) -> &SoxType {
        self.typ.val.as_ref()
    }
}

