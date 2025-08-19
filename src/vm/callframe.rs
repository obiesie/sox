use crate::builtins::function::SoxFunc;
use crate::vm::chunk::Chunk;

pub struct CallFrame {
    pub ip: usize,
    pub value_stack_base_addr: usize,
    pub co: Option<Chunk>,
}

impl CallFrame {
    pub fn new_frame() -> Self {
        Self {
            co: None,
            ip: 0,
            value_stack_base_addr: 0,
        }
    }

    #[inline(always)]
    pub fn relative_slot(&self, slot: usize) -> usize {
        self.value_stack_base_addr + slot
    }
}
