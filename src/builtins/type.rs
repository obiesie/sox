use crate::builtins::core::{SoxClassImpl, SoxObjectPayload, SoxResult, StaticType};
use crate::builtins::exceptions::{Exception, RuntimeError};
use crate::builtins::function::SoxFunction;
use crate::builtins::method::{FuncArgs, SoxMethod};
use crate::interpreter::Interpreter;
use crate::token::Token;
use macros::soxtype;
use once_cell::sync::OnceCell;
use std::any::Any;
use std::borrow::Borrow;
use std::cell::RefCell;
use std::collections::HashMap;
use std::ops::Deref;
use crate::object::core::{Sox, SoxObjectRef, SoxRef};
use crate::slots::call::Callable;
use crate::slots::repr::Representable;

pub type GenericMethod = fn(SoxObjectRef, FuncArgs, &mut Interpreter) -> SoxResult;
pub type ReprMethod = fn(&SoxObjectRef, &Interpreter) -> SoxResult<String>;

#[derive(Clone, Debug, Default)]
pub struct SoxTypeSlot {
    pub call: Option<GenericMethod>,
    pub repr: Option<ReprMethod>,
    pub methods: &'static [(&'static str, SoxMethod)],
}

pub type SoxAttributes = HashMap<String, SoxObjectRef>;

#[derive(Clone)]
pub struct SoxType {
    pub base: Option<SoxRef<SoxType>>,
    pub methods: HashMap<String, SoxMethod>,
    pub slots: SoxTypeSlot,
    pub attributes: SoxAttributes,
    pub name: Option<String>,
}

#[soxtype]
impl SoxType {
    pub fn new_static_type<T: ToString>(
        name: T,
        base: Option<SoxRef<SoxType>>,
        methods: HashMap<String, SoxMethod>,
        slots: SoxTypeSlot,
        attributes: SoxAttributes,
        meta_class: SoxRef<SoxType>,
    ) -> SoxRef<Self> {
        let typ = Self {
            base,
            methods,
            slots,
            attributes,
            name: Some(name.to_string()),
        };
        SoxRef::new_ref(typ, meta_class)
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
        if init_method.is_none() {
            return 0;
        }
        init_method.unwrap().payload::<SoxFunction>().unwrap().arity as i32
    }

    pub fn find_method(&self, name: &str) -> Option<SoxObjectRef> {
        self.attributes
            .get(name)
            .cloned()
            .or_else(|| self.base.as_ref().and_then(|base| base.find_method(name)))
    }

    
}

impl Representable for SoxType {
    fn repr(zelf: &Sox<Self>, _i: &Interpreter) -> String {
        format!("<type '{}'>", zelf.name.as_ref().unwrap().to_string())
    }
}

impl Callable for SoxType {
    fn call(zelf: &Sox<Self>, args: FuncArgs, interpreter: &mut Interpreter) -> SoxResult {
        if args.args.len() != zelf.arity() as usize {
            let error = Exception::Err(RuntimeError {
                msg: format!(
                    "Expected {} arguments but got {}.",
                    zelf.arity(),
                    args.args.len()
                ),
            });
            return Err(SoxObjectRef::from(SoxRef::new_ref(
                error,
                interpreter.types.exception_type.to_owned(),
            )));
        }
        let instance = SoxInstance::new(SoxRef::new_ref(
            zelf.deref().clone(),
            interpreter.types.obj_type.to_owned(),
        ));
        let initializer = zelf.find_method("init".into());
        let instance = SoxObjectRef::from(SoxRef::new_ref(
            instance,
            interpreter.types.obj_type.to_owned(),
        ));
        let ret_val = if let Some(init_func) = initializer {
            let func = init_func
                .payload::<SoxFunction>()
                .expect("init resolved to a non function object");
            // TODO is this round tripping necessary?
            let bound_method = func.bind(instance.clone(), interpreter)?;
            SoxFunction::slot_call(bound_method, args, interpreter)?;
            Ok(instance)
        } else {
            Ok(instance)
        };
        ret_val
    }
}
impl SoxObjectPayload for SoxType {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl StaticType for SoxType {
    const NAME: &'static str = "type";

    fn static_cell() -> &'static OnceCell<SoxRef<SoxType>> {
        static CELL: OnceCell<SoxRef<SoxType>> = OnceCell::new();
        &CELL
    }

    fn create_slots() -> SoxTypeSlot {
        SoxTypeSlot {
            call: Some(Self::slot_call),
            repr: Some(Self::slot_repr),
            methods: Self::METHOD_DEFS,
        }
    }
}

#[derive(Clone)]
pub struct SoxInstance {
    typ: SoxRef<SoxType>,
    fields: RefCell<HashMap<String, SoxObjectRef>>,
}

#[soxtype]
impl SoxInstance {
    pub fn new(class: SoxRef<SoxType>) -> Self {
        let fields = HashMap::new();
        Self {
            typ: class,
            fields: RefCell::new(fields),
        }
    }

    pub fn set(&self, name: Token, value: SoxObjectRef) {
        self.fields.borrow_mut().insert(name.lexeme.into(), value);
    }

    pub fn get(zelf: SoxRef<SoxInstance>, name: Token, interp: &mut Interpreter) -> SoxResult {
        if let Some(field_value) = zelf.fields.borrow().get(name.lexeme.as_str()) {
            return Ok(field_value.clone());
        }

        if let Some(method) = zelf.typ.find_method(name.lexeme.as_str()) {
            if let Some(func) = method.payload::<SoxFunction>() {
                let bound_method = func.bind(SoxObjectRef::from(zelf.clone()), interp);
                return bound_method;
            } else {
                return Err(Interpreter::runtime_error(
                    interp,
                    format!(
                        "Found property with same name, {}, but it is not a function",
                        name.lexeme
                    ),
                ));
            }
        }

        Err(Interpreter::runtime_error(
            interp,
            format!("Undefined property - {}", name.lexeme),
        ))
    }
}

impl StaticType for SoxInstance {
    const NAME: &'static str = "instance";

    fn static_cell() -> &'static OnceCell<SoxRef<SoxType>> {
        static CELL: OnceCell<SoxRef<SoxType>> = OnceCell::new();
        &CELL
    }

    fn create_slots() -> SoxTypeSlot {
        SoxTypeSlot {
            call: None,
            repr: Some(Self::slot_repr),
            methods: Self::METHOD_DEFS,
        }
    }
}

impl Representable for SoxInstance {
    fn repr(zelf: &Sox<Self>, _i: &Interpreter) -> String {
        format!(
            "<{} instance>",
            zelf.typ
                .name
                .as_ref()
                .unwrap_or(&"Unknown type".to_string())
                .to_string()
        )
    }
}
impl SoxObjectPayload for SoxInstance {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl Borrow<SoxType> for SoxRef<SoxType> {
    fn borrow(&self) -> &SoxType {
        todo!()
    }
}
