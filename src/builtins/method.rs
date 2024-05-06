use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;

use crate::core::{SoxObject, SoxResult, ToSoxResult, TryFromSoxObject};
use crate::interpreter::Interpreter;

pub type SoxNativeFunction = dyn Fn(&Interpreter, FuncArgs) -> SoxResult;

#[derive(Clone)]
pub struct SoxMethod {
    pub func: &'static SoxNativeFunction,
}

impl Debug for SoxMethod {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
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
    fn into_func(self) -> &'static SoxNativeFunction {
        let boxed = Box::new(move |i: &Interpreter, args: FuncArgs| self.call(i, args));
        Box::leak(boxed)
    }

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

pub const fn static_func<Kind, R, F: NativeFn<Kind, R>>(f: F) -> &'static SoxNativeFunction {
    std::mem::forget(f);
    F::STATIC_FUNC
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
        let v = args.args.iter().take(1).next().unwrap().clone();
        T::try_from_sox_object(i, v)
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

pub trait SoxNF<Kind> {
    fn call_(&self, args: Vec<i64>);
}

impl<F, R> NativeFn<(), R> for F
where
    F: Fn() -> R + 'static,
    R: ToSoxResult,
{
    fn call(&self, i: &Interpreter, args: FuncArgs) -> SoxResult {
        (self)().to_sox_result(i)
    }
}

impl<F, T1, R> NativeFn<(T1,), R> for F
where
    F: Fn(T1) -> R + 'static,
    T1: FromArgs,
    R: ToSoxResult,
{
    fn call(&self, i: &Interpreter, mut args: FuncArgs) -> SoxResult {
        let (zelf,) = (args.bind::<(T1,)>(i)).expect("Fail");
        (self)(zelf).to_sox_result(i)
    }
}

pub struct BorrowedParam<T>(PhantomData<T>);

pub struct OwnedParam<T>(PhantomData<T>);

impl<F, S, R> NativeFn<(BorrowedParam<S>,), R> for F
where
    F: Fn(&S) -> R + 'static,
    S: FromArgs,
    R: ToSoxResult,
{
    fn call(&self, i: &Interpreter, mut args: FuncArgs) -> SoxResult {
        let (zelf,) = (args.bind::<(S,)>(i)).expect("Fail");
        (self)(&zelf).to_sox_result(i)
    }
}

impl<F, S, S1, R> NativeFn<(BorrowedParam<S>, S1, &Interpreter), R> for F
where
    F: Fn(&S, S1, &Interpreter) -> R + 'static,
    S: FromArgs,
    S1: FromArgs,
    R: ToSoxResult,
{
    fn call(&self, i: &Interpreter, mut args: FuncArgs) -> SoxResult {
        let (zelf, s1) = (args.bind::<(S, S1)>(i)).expect("Fail");
        (self)(&zelf, s1, i).to_sox_result(i)
    }
}

impl<F, T, R> NativeFn<(OwnedParam<T>,), R> for F
where
    F: Fn(T) -> R + 'static,
    T: FromArgs,
    R: ToSoxResult,
{
    fn call(&self, i: &Interpreter, mut args: FuncArgs) -> SoxResult {
        let (zelf,) = (args.bind::<(T,)>(i)).expect("Fail");
        (self)(zelf).to_sox_result(i)
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
        let (zelf, v1) = (args.bind::<(S, T)>(i)).expect("Fail");
        (self)(&zelf, v1).to_sox_result(i)
    }
}


impl<F, T1, T2, R> NativeFn<(T1, T2), R> for F
where
    F: Fn(T1, T2) -> R + 'static,
    T1: FromArgs,
    T2: FromArgs,
    R: ToSoxResult,
{
    fn call(&self, i: &Interpreter, mut args: FuncArgs) -> SoxResult {
        let (zelf, v1) = (args.bind::<(T1, T2)>(i).expect("Fail"));
        (self)(zelf, v1).to_sox_result(i)
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
        let (zelf, v1, v2) = (args.bind::<(T1, T2, T3)>(i).expect("Fail"));
        (self)(zelf, v1, v2).to_sox_result(i)
    }
}

impl FromArgs for FuncArgs {
    fn from_args(i: &Interpreter, args: &mut FuncArgs) -> SoxResult<Self> {
        return Ok(args.clone());
    }
}
