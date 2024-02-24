use crate::core::SoxObject;

#[derive(Clone, Debug)]
pub enum Exception {
    Err(RuntimeError),
    Return(SoxObject),
}

impl From<RuntimeError> for Exception {
    fn from(value: RuntimeError) -> Self {
        Exception::Err(value)
    }
}

#[derive(Clone, Debug, Default)]
pub struct RuntimeError {
    pub msg: String,
}

impl From<Exception> for RuntimeError {
    fn from(value: Exception) -> Self {
        if let Exception::Err(v) = value {
            v
        } else {
            RuntimeError { msg: "".into() }
        }
    }
}
