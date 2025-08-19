use crate::builtins::int::SoxInt;
use crate::interpreter::Interpreter;
use crate::object::core::{SoxObjectRef, SoxRef};
use std::fmt::Debug;

#[derive(Clone, Copy, PartialEq, Debug)]
#[repr(u8)]
pub enum OpCode {
    OpConstant,
    OpNone,
    OpTrue,
    OpFalse,
    OpEqual,
    OpGreater,
    OpLess,
    OpAdd,
    OpSubtract,
    OpMultiply,
    OpDivide,
    OpNot,
    OpNegate,
    OpPrint,
    OpPop,
    OpDefineGlobal,
    OpGetGlobal,
    OpSetGlobal,
    OpGetLocal,
    OpSetLocal,
    OpJumpIfFalse,
    OpJump,
    OpLoop,
    OpCall,
    OpReturn,
}

impl TryFrom<u8> for OpCode {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        // Static mapping array for u8 to OpCode
        const OPCODE_MAP: [Option<OpCode>; 25] = [
            Some(OpCode::OpConstant),
            Some(OpCode::OpNone),
            Some(OpCode::OpTrue),
            Some(OpCode::OpFalse),
            Some(OpCode::OpEqual),
            Some(OpCode::OpGreater),
            Some(OpCode::OpLess),
            Some(OpCode::OpAdd),
            Some(OpCode::OpSubtract),
            Some(OpCode::OpMultiply),
            Some(OpCode::OpDivide),
            Some(OpCode::OpNot),
            Some(OpCode::OpNegate),
            Some(OpCode::OpPrint),
            Some(OpCode::OpPop),
            Some(OpCode::OpDefineGlobal),
            Some(OpCode::OpGetGlobal),
            Some(OpCode::OpSetGlobal),
            Some(OpCode::OpGetLocal),
            Some(OpCode::OpSetLocal),
            Some(OpCode::OpJumpIfFalse),
            Some(OpCode::OpJump),
            Some(OpCode::OpLoop),
            Some(OpCode::OpCall),
            Some(OpCode::OpReturn),
        ];

        OPCODE_MAP
            .get(value as usize)
            .and_then(|&opcode| opcode)
            .ok_or(())
    }
}

impl TryFrom<OpCode> for u8 {
    type Error = ();

    fn try_from(op_code: OpCode) -> Result<Self, Self::Error> {
        match op_code {
            OpCode::OpConstant => Ok(0),
            OpCode::OpNone => Ok(1),
            OpCode::OpTrue => Ok(2),
            OpCode::OpFalse => Ok(3),
            OpCode::OpEqual => Ok(4),
            OpCode::OpGreater => Ok(5),
            OpCode::OpLess => Ok(6),
            OpCode::OpAdd => Ok(7),
            OpCode::OpSubtract => Ok(8),
            OpCode::OpMultiply => Ok(9),
            OpCode::OpDivide => Ok(10),
            OpCode::OpNot => Ok(11),
            OpCode::OpNegate => Ok(12),
            OpCode::OpPrint => Ok(13),
            OpCode::OpPop => Ok(14),
            OpCode::OpDefineGlobal => Ok(15),
            OpCode::OpGetGlobal => Ok(16),
            OpCode::OpSetGlobal => Ok(17),
            OpCode::OpGetLocal => Ok(18),
            OpCode::OpSetLocal => Ok(19),
            OpCode::OpJumpIfFalse => Ok(20),
            OpCode::OpJump => Ok(21),
            OpCode::OpLoop => Ok(22),
            OpCode::OpCall => Ok(23),
            OpCode::OpReturn => Ok(24),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Chunk {
    pub name: String,
    pub code: Vec<u8>,
    pub constants: Vec<SoxObjectRef>,
    pub lines: Vec<usize>,
}

impl Default for Chunk {
    fn default() -> Self {
        Self::new()
    }
}

impl Chunk {
    pub fn test_chunk(i: &Interpreter) -> Chunk {
        let mut chunk = Chunk::new();
        let value = SoxInt { value: 42 };
        let val_ref = SoxRef::new_ref(value, i.types.int_type.to_owned());
        let obj_ref = SoxObjectRef::from(val_ref);
        let const_idx = chunk.add_constant(obj_ref);

        chunk.write_chunk(OpCode::OpConstant, 1);
        chunk.write_chunk(const_idx as u8, 1);
        chunk.write_chunk(OpCode::OpNegate, 1);
        chunk.write_chunk(OpCode::OpReturn, 1);
        chunk
    }
}

impl Chunk {
    pub fn new() -> Chunk {
        Chunk {
            name: String::from("<module>"),
            code: Vec::new(),
            constants: vec![],
            lines: vec![],
        }
    }

    pub fn get_line(&self, offset: usize) -> usize {
        self.lines[offset]
    }

    pub fn write_chunk<T: TryInto<u8>>(&mut self, data: T, line: usize) {
        if let Ok(data) = data.try_into() {
            self.code.push(data);
            self.lines.push(line);
        }
    }

    pub fn disassemble(&self, name: &str) {
        println!("== {} ==", name);
        let mut offset = 0;
        while offset < self.code.len() {
            offset = self.disassemble_instruction(offset);
            println!("{:?}", offset);
        }
    }

    pub fn disassemble_instruction(&self, offset: usize) -> usize {
        print!("{:04} ", offset);
        if offset > 0 && self.lines[offset] == self.lines[offset - 1] {
            print!("   | ");
        } else {
            print!("{:4} ", self.lines[offset]);
        }
        let opcode = self.code[offset].try_into().unwrap();

        match opcode {
            OpCode::OpReturn => self.simple_instruction("OpReturn", offset),
            OpCode::OpConstant => self.constant_instruction("OpConstant", offset),
            OpCode::OpNegate => self.simple_instruction("OpNegate", offset),
            OpCode::OpAdd => self.simple_instruction("OpAdd", offset),
            OpCode::OpSubtract => self.simple_instruction("OpSubtract", offset),
            OpCode::OpMultiply => self.simple_instruction("OpMultiply", offset),
            OpCode::OpDivide => self.simple_instruction("OpDivide", offset),
            OpCode::OpNot => self.simple_instruction("OpNot", offset),
            OpCode::OpNone => self.simple_instruction("OpNone", offset),
            OpCode::OpTrue => self.simple_instruction("OpTrue", offset),
            OpCode::OpFalse => self.simple_instruction("OpFalse", offset),
            OpCode::OpEqual => self.simple_instruction("OpEqual", offset),
            OpCode::OpGreater => self.simple_instruction("OpGreater", offset),
            OpCode::OpLess => self.simple_instruction("OpLess", offset),
            OpCode::OpPrint => self.simple_instruction("OpPrint", offset),
            OpCode::OpDefineGlobal => self.simple_instruction("OpDefineGlobal", offset),
            OpCode::OpPop => self.simple_instruction("OpPop", offset),
            OpCode::OpGetGlobal => self.simple_instruction("OpGetGlobal", offset),
            OpCode::OpSetGlobal => self.simple_instruction("OpSetGlobal", offset),
            OpCode::OpGetLocal => self.byte_instruction("OpGetLocal", offset),
            OpCode::OpSetLocal => self.byte_instruction("OpSetLocal", offset),
            OpCode::OpJumpIfFalse => self.jump_instruction("OP_JUMP_IF_FALSE", 1, offset),
            OpCode::OpJump => self.jump_instruction("OP_JUMP", 1, offset),
            OpCode::OpLoop => self.jump_instruction("OP_LOOP", -1, offset),
            OpCode::OpCall => self.simple_instruction("OP_CALL", offset),
        }
    }

    fn jump_instruction(&self, op_code: &str, sign: isize, offset: usize) -> usize {
        let mut jump = (self.code[offset + 1] as u16) << 8;
        jump |= self.code[offset + 2] as u16;
        print!(
            "{} {:#04x} -> {}",
            op_code,
            jump,
            (offset as isize) + (3 as isize) + sign * (jump as isize)
        );
        offset + 3
    }

    fn byte_instruction(&self, opcode: &str, offset: usize) -> usize {
        let byte = self.code[offset + 1];
        print!("{} {:#04x}", opcode, byte);
        offset + 2
    }

    fn simple_instruction(&self, opcode: &str, offset: usize) -> usize {
        println!("{}", opcode);
        offset + 1
    }

    pub fn add_constant(&mut self, constant: SoxObjectRef) -> usize {
        self.constants.push(constant);
        self.constants.len() - 1
    }

    fn constant_instruction(&self, opcode: &str, offset: usize) -> usize {
        let constant_idx = self.code[offset + 1] as usize;
        // print!("{} ({:?})", opcode, constant_idx);
        // println!("{:?}", self.constants[constant_idx]);
        offset + 2
    }
}
