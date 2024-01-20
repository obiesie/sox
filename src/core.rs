use std::fmt::Debug;
use crate::int::SoxIntRef;
use crate::string::SoxStringRef;


#[derive(Clone, Debug)]
pub enum SoxObj {
    Int(SoxIntRef),
    String(SoxStringRef),
}

