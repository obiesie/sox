use crate::builtins::function::SoxFunction;
use crate::object::core::{SoxObjectRef, SoxRef};
use crate::builtins::chunk::Chunk;

#[derive(Debug)]
pub struct CallFrame {
    pub ip: usize,
    pub value_stack_base_addr: usize,
    pub co: Option<SoxRef<Chunk>>,
    pub upvalues: Vec<SoxObjectRef>
}

impl CallFrame {
    pub fn new_frame() -> Self {
        Self {
            ip: 0,
            value_stack_base_addr: 0,
            upvalues: Vec::new(),
            co: None,
        }
    }

    #[inline(always)]
    pub fn relative_slot(&self, slot: usize) -> usize {
        self.value_stack_base_addr + slot
    }
}
