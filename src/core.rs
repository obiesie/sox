use std::fmt::Debug;
use crate::int::SoxIntRef;
use crate::string::SoxStringRef;

#[macro_export]
macro_rules! payload {
    ($e:expr, $p:path) => {
        match $e {
            $p(v) => Some(v),
            _ => None
        }
    };
}

#[derive(Clone, Debug)]
pub enum SoxObject {
    Int(SoxIntRef),
    String(SoxStringRef),
}

