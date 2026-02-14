use crate::builtins::core::{SoxClassImpl, SoxObjectPayload, SoxResult, StaticType};
use crate::builtins::exceptions::{Exception, RuntimeError};
use crate::builtins::function::SoxFn;
use crate::builtins::method::{FuncArgs, SoxMethod};
use crate::object::core::{Sox, SoxObjectRef, SoxRef};
use crate::object::protocols::call::Callable;
use crate::object::protocols::comparable::ComparableMethods;
use crate::object::protocols::number::NumberMethods;
use crate::object::protocols::repr::Representable;
use crate::runtime::Runtime;
use crate::token::Token;
use macros::soxtype;
use once_cell::sync::OnceCell;
use std::any::Any;
use std::borrow::Borrow;
use std::cell::RefCell;
use std::collections::HashMap;
use std::ops::Deref;

pub type GenericMethod = fn(SoxObjectRef, FuncArgs, &mut Runtime) -> SoxResult;
pub type ReprMethod = fn(&SoxObjectRef, &Runtime) -> SoxResult<String>;
/// Type-erased trace function for GC - looks up children of an object
pub type TraceFn = fn(&SoxObjectRef, &mut dyn FnMut(SoxObjectRef));
pub type DropFn = fn(&SoxObjectRef);

#[derive(Clone, Debug, Default)]
pub struct SoxTypeSlot {
    pub call: Option<GenericMethod>,
    pub repr: Option<ReprMethod>,
    pub trace: Option<TraceFn>,
    pub drop: Option<DropFn>,
    pub number: Option<NumberMethods>,
    pub comparable: Option<ComparableMethods>,
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
        init_method.unwrap().payload::<SoxFn>().unwrap().arity as i32
    }

    pub fn find_method(&self, name: &str) -> Option<SoxObjectRef> {
        self.attributes
            .get(name)
            .cloned()
            .or_else(|| self.base.as_ref().and_then(|base| base.find_method(name)))
    }

    fn slot_trace(obj: &SoxObjectRef, trace_fn: &mut dyn FnMut(SoxObjectRef)) {
        if let Some(typ) = obj.payload::<SoxType>() {
            if let Some(base) = &typ.base {
                trace_fn(SoxObjectRef::from(base.clone()));
            }
            for val in typ.attributes.values() {
                trace_fn(val.clone());
            }
        }
    }
}

impl Representable for SoxType {
    fn repr(zelf: &Sox<Self>, _i: &Runtime) -> String {
        format!("<type '{}'>", zelf.name.as_ref().unwrap().to_string())
    }
}

impl Callable for SoxType {
    fn call(zelf: &Sox<Self>, args: FuncArgs, interpreter: &mut Runtime) -> SoxResult {
        if args.args.len() != zelf.arity() as usize {
            let error = Exception::Err(RuntimeError {
                msg: format!(
                    "Expected {} arguments but got {}.",
                    zelf.arity(),
                    args.args.len()
                ),
            });
            return Err(SoxObjectRef::from(
                interpreter.alloc(error, interpreter.types.exception_type.to_owned()),
            ));
        }
        let instance = SoxInstance::new(SoxRef::new_ref(
            zelf.deref().clone(),
            interpreter.types.obj_type.to_owned(),
        ));
        let initializer = zelf.find_method("init".into());
        let instance =
            SoxObjectRef::from(interpreter.alloc(instance, interpreter.types.obj_type.to_owned()));
        let ret_val = if let Some(init_func) = initializer {
            let func = init_func
                .payload::<SoxFn>()
                .expect("init resolved to a non function object");
            // TODO is this round tripping necessary?
            let bound_method = func.bind(instance.clone(), interpreter)?;
            SoxFn::slot_call(bound_method, args, interpreter)?;
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
            trace: Some(Self::slot_trace),
            drop: None,
            number: None,
            comparable: None,
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

    pub fn get(zelf: SoxRef<SoxInstance>, name: Token, interp: &mut Runtime) -> SoxResult {
        if let Some(field_value) = zelf.fields.borrow().get(name.lexeme) {
            return Ok(field_value.clone());
        }

        if let Some(method) = zelf.typ.find_method(name.lexeme) {
            if let Some(func) = method.payload::<SoxFn>() {
                let bound_method = func.bind(SoxObjectRef::from(zelf.clone()), interp);
                return bound_method;
            } else {
                return Err(Runtime::runtime_error(
                    interp,
                    format!(
                        "Found property with same name, {}, but it is not a function",
                        name.lexeme
                    ),
                ));
            }
        }

        Err(Runtime::runtime_error(
            interp,
            format!("Undefined property - {}", name.lexeme),
        ))
    }

    fn slot_trace(obj: &SoxObjectRef, trace_fn: &mut dyn FnMut(SoxObjectRef)) {
        if let Some(ins) = obj.payload::<SoxInstance>() {
            trace_fn(SoxObjectRef::from(ins.typ.clone()));
            for val in ins.fields.borrow().values() {
                trace_fn(val.clone());
            }
        }
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
            trace: Some(Self::slot_trace),
            drop: Some(Self::slot_drop),
            number: None,
            comparable: None,
            methods: Self::METHOD_DEFS,
        }
    }
}

impl SoxInstance {
    fn slot_drop(obj: &SoxObjectRef) {
        if obj.payload::<SoxInstance>().is_some() {
            unsafe {
                let inner =
                    obj.ptr.as_ptr() as *mut crate::object::core::SoxObjectInner<SoxInstance>;
                std::ptr::drop_in_place(&mut (*inner).payload);
            }
        }
    }
}

impl Representable for SoxInstance {
    fn repr(zelf: &Sox<Self>, _i: &Runtime) -> String {
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
