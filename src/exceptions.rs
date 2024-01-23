
#[derive(Clone, Debug, Default)]
pub struct RuntimeException {
    pub msg: String,
}



pub enum SoxResult<T, V, E>{
    Ok(T),
    Return(V),
    Err(E)
}