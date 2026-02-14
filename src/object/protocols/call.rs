use crate::builtins::core::{SoxObjectPayload, SoxResult};
use crate::builtins::method::FuncArgs;
use crate::runtime::Runtime;
use crate::object::core::{Sox, SoxObjectRef};

pub trait Callable {
    fn slot_call(zelf: SoxObjectRef, mut args: FuncArgs, i: &mut Runtime) -> SoxResult
    where
        Self: SoxObjectPayload,
    {
        let zelf = zelf.downcast_ref().ok_or_else(|| {
            let repr = zelf.repr(i);
            let help = if let Ok(repr) = repr.as_ref() {
                repr.as_str().to_owned()
            } else {
                zelf.typ().name.clone().unwrap().to_owned()
            };
            i.runtime_error(format!("{} is not callable", help))
        })?;
        let args = args.bind(i)?;
        Self::call(zelf, args, i)
    }

    fn call(zelf: &Sox<Self>, args: FuncArgs, i: &mut Runtime) -> SoxResult
    where
        Self: SoxObjectPayload;
}
