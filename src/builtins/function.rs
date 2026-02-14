use crate::builtins::bool::SoxBool;
use crate::builtins::exceptions::{Exception, RuntimeError};
use crate::builtins::method::{FuncArgs, SoxMethod};
use crate::builtins::r#type::{SoxInstance, SoxType, SoxTypeSlot};
use crate::builtins::string::SoxString;
use macros::soxtype;
use once_cell::sync::OnceCell;
use std::any::Any;
use std::iter::zip;

use crate::builtins::core::{
    SoxClassImpl, SoxObjectPayload, SoxResult, StaticType, ToSoxResult, TryFromSoxObject,
};

use crate::builtins::chunk::Chunk;
use crate::builtins::string;
use crate::environment::EnvRef;
use crate::object::core::{Sox, SoxObjectRef, SoxRef};
use crate::object::protocols::call::Callable;
use crate::object::protocols::comparable::{Comparable, ComparableMethods};
use crate::object::protocols::repr::Representable;
use crate::runtime::Runtime;
use crate::stmt::Stmt;

#[derive(Clone, Debug)]
pub struct SoxFunction {
    pub name: String,
    pub arity: usize,
    pub upvalue_count: usize,
    pub upvalues: Vec<SoxObjectRef>,
    pub chunk: SoxRef<Chunk>,
}

#[soxtype]
impl SoxFunction {
    pub fn new(name: String, arity: usize, upvalue_count: usize, chunk: SoxRef<Chunk>) -> Self {
        Self {
            name,
            arity,
            upvalue_count,
            upvalues: vec![],
            chunk,
        }
    }

    pub fn with_upvalues(&self, upvalues: Vec<SoxObjectRef>) -> Self {
        Self {
            name: self.name.clone(),
            arity: self.arity,
            upvalue_count: self.upvalue_count,
            upvalues,
            chunk: self.chunk.clone(),
        }
    }
}

impl Representable for SoxFunction {
    fn repr(zelf: &Sox<Self>, _i: &Runtime) -> String {
        let func_name = zelf.name.to_string();
        format!("<Function {func_name}>")
    }
}
impl SoxObjectPayload for SoxFunction {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl TryFromSoxObject for SoxFunction {
    fn try_from_sox_object(i: &mut Runtime, obj: SoxObjectRef) -> SoxResult<Self> {
        if let Some(val) = obj.payload::<SoxFunction>() {
            Ok(val.clone())
        } else {
            let err_msg = SoxString {
                value: String::from("failed to get a function from supplied object"),
            };
            let ob = i.alloc(err_msg, string::SoxString::init_builtin_type().to_owned());
            Err(ob.into())
        }
    }
}

impl ToSoxResult for SoxFunction {
    fn to_sox_result(self, i: &mut Runtime) -> SoxResult {
        let obj = i.alloc(self, i.types.function_type.to_owned());
        Ok(SoxObjectRef::from(obj))
    }
}

impl StaticType for SoxFunction {
    const NAME: &'static str = "func";

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

impl SoxFunction {
    fn slot_drop(obj: &SoxObjectRef) {
        if obj.payload::<SoxFunction>().is_some() {
            unsafe {
                let inner =
                    obj.ptr.as_ptr() as *mut crate::object::core::SoxObjectInner<SoxFunction>;
                std::ptr::drop_in_place(&mut (*inner).payload);
            }
        }
    }
}

impl SoxFunction {
    /// GC trace function - reports child references to the collector.
    fn slot_trace(obj: &SoxObjectRef, trace_fn: &mut dyn FnMut(SoxObjectRef)) {
        // Get the SoxFunction payload from the object
        if let Some(func) = obj.payload::<SoxFunction>() {
            // Report the chunk as a child
            trace_fn(SoxObjectRef::from(func.chunk.clone()));

            // Report all upvalues as children
            for upvalue in &func.upvalues {
                trace_fn(upvalue.clone());
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SoxFn {
    pub name: String,
    pub declaration: Box<Stmt>,
    pub environment_ref: EnvRef,
    pub is_initializer: bool,
    pub arity: i8,
}

#[soxtype]
impl SoxFn {
    pub fn new(
        name: String,
        declaration: Stmt,
        environment_ref: EnvRef,
        arity: i8,
        is_initializer: bool,
    ) -> Self {
        Self {
            name,
            declaration: Box::new(declaration),
            environment_ref,
            is_initializer,
            arity,
        }
    }

    pub fn bind(&self, instance: SoxObjectRef, interp: &mut Runtime) -> SoxResult {
        if let Some(_) = instance.payload::<SoxInstance>() {
            let env_ref = interp
                .environment
                .new_local_env_at(self.environment_ref.clone());
            interp
                .environment
                .define_at("this", instance, env_ref.clone());

            let new_func = SoxFn {
                name: self.name.to_string(),
                declaration: self.declaration.clone(),
                environment_ref: env_ref,
                is_initializer: self.is_initializer,
                arity: self.arity,
            };
            Ok(SoxObjectRef::from(
                interp.alloc(new_func, interp.types.func_type.to_owned()),
            ))
        } else {
            Err(Runtime::runtime_error(
                interp,
                "Could not bind method to instance".to_string(),
            ))
        }
    }

    fn equals(lhs: SoxObjectRef, rhs: SoxObjectRef, _i: &mut Runtime) -> SoxResult {
        if let (Some(lhs), Some(rhs)) = (lhs.payload::<SoxFn>(), rhs.payload::<SoxFn>()) {
            SoxBool::new(lhs == rhs).to_sox_result(_i)
        } else {
            SoxBool::new(false).to_sox_result(_i)
        }
    }
}

impl SoxObjectPayload for SoxFn {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl StaticType for SoxFn {
    const NAME: &'static str = "function";

    fn static_cell() -> &'static OnceCell<SoxRef<SoxType>> {
        static CELL: OnceCell<SoxRef<SoxType>> = OnceCell::new();
        &CELL
    }

    fn create_slots() -> SoxTypeSlot {
        SoxTypeSlot {
            call: Some(Self::slot_call),
            repr: Some(Self::slot_repr),
            trace: Some(Self::slot_trace),
            drop: Some(Self::slot_drop),
            number: None,
            comparable: Some(Self::as_comparable()),
            methods: Self::METHOD_DEFS,
        }
    }
}

impl SoxFn {
    fn slot_drop(obj: &SoxObjectRef) {
        if obj.payload::<SoxFn>().is_some() {
            unsafe {
                let inner = obj.ptr.as_ptr() as *mut crate::object::core::SoxObjectInner<SoxFn>;
                std::ptr::drop_in_place(&mut (*inner).payload);
            }
        }
    }
}

impl SoxFn {
    fn slot_trace(_obj: &SoxObjectRef, _trace_fn: &mut dyn FnMut(SoxObjectRef)) {}
}

impl TryFromSoxObject for SoxFn {
    fn try_from_sox_object(i: &mut Runtime, obj: SoxObjectRef) -> SoxResult<Self> {
        if let Some(func) = obj.payload::<SoxFn>() {
            Ok(func.clone())
        } else {
            let err_msg = SoxString {
                value: String::from("failed to get function from supplied object"),
            };
            let ob = i.alloc(err_msg, i.types.str_type.to_owned());
            Err(ob.into())
        }
    }
}

impl ToSoxResult for SoxFn {
    fn to_sox_result(self, i: &mut Runtime) -> SoxResult {
        let obj = SoxObjectRef::from(i.alloc(self, i.types.func_type.to_owned()));
        Ok(obj)
    }
}

impl Representable for SoxFn {
    fn repr(zelf: &Sox<Self>, _i: &Runtime) -> String {
        let func_name = zelf.name.to_string();
        format!("<Function {func_name}>")
    }
}

impl Callable for SoxFn {
    fn call(zelf: &Sox<Self>, args: FuncArgs, i: &mut Runtime) -> SoxResult {
        if args.args.len() != zelf.arity as usize {
            let err = format!("'{}' not callable", zelf.name);
            return Err(SoxObjectRef::from(i.alloc(
                Exception::Err(RuntimeError { msg: err }),
                i.types.exception_type.to_owned(),
            )));
        }
        let previous_env_ref = i.environment.active.clone();
        i.environment.active = zelf.environment_ref.clone();

        let mut return_value = Ok(SoxObjectRef::from(i.none.clone()));
        if let Stmt::Function {
            name: _,
            params,
            body,
        } = *zelf.declaration.clone()
        {
            let exec_ns = i.environment.new_local_env_at(zelf.environment_ref.clone());
            let env = i.environment.envs.get_mut(*exec_ns).unwrap();
            for (param, arg) in zip(params, args.args.clone()) {
                env.define(param.lexeme, arg).expect("TODO: panic message");
            }
            let ret = i.execute_block(body.iter().collect(), Option::from(exec_ns));

            if ret.is_err() {
                let error = ret.unwrap_err();
                let exc = error.payload::<Exception>();
                if let Some(obj) = exc {
                    match obj {
                        Exception::Return(v) => {
                            let val = v.clone();
                            return_value = Ok(val);
                        }
                        Exception::Err(v) => {
                            let rv = Exception::Err(v.clone());
                            return_value = Err(SoxObjectRef::from(
                                i.alloc(rv, i.types.exception_type.to_owned()),
                            ));
                        }
                    }
                }
            }
        }
        if zelf.is_initializer {
            let v = i.environment.find_and_get("this");
            i.environment.active = previous_env_ref;
            return v;
        }
        i.environment.active = previous_env_ref;

        return_value
    }
}

impl Comparable for SoxFn {
    fn as_comparable() -> ComparableMethods {
        ComparableMethods {
            lt: None,
            gt: None,
            eq: Some(Self::equals),
            ne: None,
            ge: None,
            le: None,
        }
    }
}
