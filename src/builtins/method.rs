use crate::builtins::exceptions::{Exception, RuntimeError};
use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;

use crate::core::{Representable, SoxObject, SoxObjectPayload, SoxResult, ToSoxResult, TryFromSoxObject};
use crate::interpreter::Interpreter;

pub type SoxNativeFunction = dyn Fn(&Interpreter, FuncArgs) -> SoxResult;

#[derive(Clone)]
pub struct SoxMethod {
    pub func: &'static SoxNativeFunction,
}

impl Debug for SoxMethod {
    fn fmt(&self, _f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl SoxMethod {
    pub const fn new<Kind, R>(f: impl NativeFn<Kind, R>) -> Self {
        Self {
            func: static_func(f),
        }
    }
}

pub trait NativeFn<K, R>: Sized + 'static {
    fn call(&self, i: &Interpreter, arg: FuncArgs) -> SoxResult;

    const STATIC_FUNC: &'static SoxNativeFunction = {
        if std::mem::size_of::<Self>() == 0 {
            &|i, args| {
                let f = unsafe { std::mem::MaybeUninit::<Self>::uninit().assume_init() };
                f.call(i, args)
            }
        } else {
            panic!("function must be zero-sized to access STATIC_FUNC")
        }
    };
}

pub const fn static_func<K, R, F: NativeFn<K, R>>(f: F) -> &'static SoxNativeFunction {
    std::mem::forget(f);
    let v = F::STATIC_FUNC;
    v
}

#[derive(Clone, Debug)]
pub struct FuncArgs {
    pub args: Vec<SoxObject>,
}

impl FuncArgs {
    pub fn new(args: Vec<SoxObject>) -> Self {
        Self { args }
    }

    fn bind<T: FromArgs>(&mut self, i: &Interpreter) -> SoxResult<T> {
        let bound = T::from_args(i, self);
        bound
    }
}

pub trait FromArgs: Sized {
    fn from_args(i: &Interpreter, args: &mut FuncArgs) -> SoxResult<Self>;
}

#[derive(Clone, Debug)]
pub struct ArgumentError;

impl<T: TryFromSoxObject> FromArgs for T {
    fn from_args(i: &Interpreter, args: &mut FuncArgs) -> SoxResult<Self> {
        let val = if let Some(v) = args.args.iter().take(1).next() {
            T::try_from_sox_object(i, v.clone())
        } else {
            Err(Exception::Err(RuntimeError {
                msg: "Too few argument supplied to function".into(),
            })
            .into_ref())
        };
        val
    }
}

impl<A: FromArgs> FromArgs for (A,) {
    fn from_args(i: &Interpreter, args: &mut FuncArgs) -> SoxResult<Self> {
        Ok((A::from_args(i, args)?,))
    }
}

impl<A: FromArgs, B: FromArgs> FromArgs for (A, B) {
    fn from_args(i: &Interpreter, args: &mut FuncArgs) -> SoxResult<Self> {
        Ok((A::from_args(i, args)?, B::from_args(i, args)?))
    }
}

impl<A: FromArgs, B: FromArgs, C: FromArgs> FromArgs for (A, B, C) {
    fn from_args(i: &Interpreter, args: &mut FuncArgs) -> SoxResult<Self> {
        Ok((
            A::from_args(i, args)?,
            B::from_args(i, args)?,
            C::from_args(i, args)?,
        ))
    }
}
pub struct BorrowedParam<T>(PhantomData<T>);

pub struct OwnedParam<T>(PhantomData<T>);

impl<F, R> NativeFn<(), R> for F
where
    F: Fn() -> R + 'static,
    R: ToSoxResult,
{
    fn call(&self, i: &Interpreter, _args: FuncArgs) -> SoxResult {
        (self)().to_sox_result(i)
    }
}

impl<F, T1, R> NativeFn<(OwnedParam<T1>,), R> for F
where
    F: Fn(T1) -> R + 'static,
    T1: FromArgs,
    R: ToSoxResult,
{
    fn call(&self, i: &Interpreter, mut args: FuncArgs) -> SoxResult {
        let (zelf,) = (args.bind::<(T1,)>(i)).expect("Failed to bind function arguments.");
        (self)(zelf).to_sox_result(i)
    }
}

impl<F, S, R> NativeFn<(BorrowedParam<S>,), R> for F
where
    F: Fn(&S) -> R + 'static,
    S: FromArgs,
    R: ToSoxResult,
{
    fn call(&self, i: &Interpreter, mut args: FuncArgs) -> SoxResult {
        let (zelf,) = (args.bind::<(S,)>(i)).expect("Failed to bind function arguments.");
        (self)(&zelf).to_sox_result(i)
    }
}

impl<F, S, S1, R> NativeFn<(S, S1), R> for F
where
    F: Fn(&S, S1) -> R + 'static,
    S: FromArgs,
    S1: FromArgs,
    R: ToSoxResult,
{
    fn call(&self, i: &Interpreter, mut args: FuncArgs) -> SoxResult {
        let (zelf, s1) = (args.bind::<(S, S1)>(i)).expect("Failed to bind function arguments.");
        (self)(&zelf, s1).to_sox_result(i)
    }
}

impl<F, S, T, R> NativeFn<(BorrowedParam<S>, OwnedParam<T>), R> for F
where
    F: Fn(&S, T) -> R + 'static,
    S: FromArgs,
    T: FromArgs,
    R: ToSoxResult,
{
    fn call(&self, i: &Interpreter, mut args: FuncArgs) -> SoxResult {
        let (zelf, v1) = (args.bind::<(S, T)>(i)).expect("Failed to bind function arguments.");
        (self)(&zelf, v1).to_sox_result(i)
    }
}

impl<F, T1, T2, T3, R> NativeFn<(T1, T2, T3), R> for F
where
    F: Fn(T1, T2, T3) -> R + 'static,
    T1: FromArgs,
    T2: FromArgs,
    T3: FromArgs,
    R: ToSoxResult,
{
    fn call(&self, i: &Interpreter, mut args: FuncArgs) -> SoxResult {
        let (zelf, v1, v2) = args
            .bind::<(T1, T2, T3)>(i)
            .expect("Failed to bind function arguments.");
        (self)(zelf, v1, v2).to_sox_result(i)
    }
}

impl FromArgs for FuncArgs {
    fn from_args(_i: &Interpreter, args: &mut FuncArgs) -> SoxResult<Self> {
        Ok(args.clone())
    }
}
