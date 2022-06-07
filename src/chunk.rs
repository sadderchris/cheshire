use gc_arena_derive::Collect;
use num_enum::{IntoPrimitive, TryFromPrimitive};

use crate::value::Value;

/// Represents an opcode that runs on our virtual machine.
/// Opcodes are 1 byte in length (for now) and represent the
/// simplest operations our VM can perform (arithmetic, control flow, etc.).
#[derive(Debug, Copy, Clone, IntoPrimitive, TryFromPrimitive, PartialEq, Eq)]
#[repr(u8)]
pub enum OpCode {
    ConstantLong,
    Constant,
    DefineGlobal,
    GetGlobal,
    SetGlobal,
    GetLocal,
    SetLocal,
    GetUpvalue,
    SetUpvalue,
    JumpIfFalse,
    Jump,
    Call,
    TailCall,
    Closure,
    Pop,
    Void,
    Null,
    True,
    False,
    Return,
}

/// Represents a series of instructions that correspond to some piece of high-level code.
#[derive(Debug, Default, Clone, Collect)]
#[collect(no_drop)]
pub struct Chunk<'gc> {
    code: Vec<u8>,
    lines: Vec<(isize, usize)>,
    constants: Vec<Value<'gc>>,
}

impl Chunk<'_> {
    pub fn new() -> Self {
        Self::default()
    }

    /// Write a single byte of data into this chunk
    pub fn write(&mut self, byte: u8, line: usize) {
        self.code.push(byte);
        if self.lines.is_empty() {
            self.lines.push((1, line));
            return;
        }

        let end = self.lines.len() - 1;
        let (times, current_line) = self.lines[end];
        if line == current_line {
            self.lines[end] = (times + 1, current_line);
        } else {
            self.lines.push((1, line));
        }
    }

    #[inline(always)]
    pub fn read(&self, offset: usize) -> u8 {
        self.code[offset]
    }

    /// Disassemble this chunk
    pub fn disassemble(&self, name: &str) {
        println!("== {} ==", name);

        let mut offset = 0;
        while offset < self.code.len() {
            offset = self.disassemble_instruction(offset);
        }
    }

    pub fn get_line(&self, offset: usize) -> usize {
        let mut current_offset = offset as isize;
        let mut i = 0;
        let mut current_line = 0;
        while current_offset >= 0 {
            let (times, line) = self.lines[i];
            current_offset -= times;
            current_line = line;
            i += 1;
        }

        current_line
    }

    /// Try to disassemble the instruction at the offset in this chunk
    pub fn disassemble_instruction(&self, offset: usize) -> usize {
        print!("{:04} ", offset);

        if offset > 0 && self.get_line(offset) == self.get_line(offset - 1) {
            print!("   | ");
        } else {
            print!("{:4} ", self.get_line(offset))
        }

        let instruction = OpCode::try_from(self.code[offset]);
        if instruction.is_err() {
            println!("Unknown opcode {}", self.code[offset]);
            return offset + 1;
        }

        match instruction.unwrap() {
            OpCode::ConstantLong => self.constant_long_instruction("CONSTANT_LONG", offset),
            OpCode::Constant => self.constant_instruction("CONSTANT", offset),
            OpCode::DefineGlobal => self.constant_instruction("DEFINE_GLOBAL", offset),
            OpCode::GetGlobal => self.constant_instruction("GET_GLOBAL", offset),
            OpCode::SetGlobal => self.constant_instruction("SET_GLOBAL", offset),
            OpCode::GetLocal => self.byte_instruction("GET_LOCAL", offset),
            OpCode::SetLocal => self.byte_instruction("SET_LOCAL", offset),
            OpCode::JumpIfFalse => self.jump_instruction("JUMP_IF_FALSE", 1, offset),
            OpCode::Jump => self.jump_instruction("JUMP", 1, offset),
            OpCode::Call => self.byte_instruction("CALL", offset),
            OpCode::TailCall => self.byte_instruction("TAIL_CALL", offset),
            OpCode::Closure => {
                let mut offset = offset + 1;
                let constant = self.read(offset);
                offset += 1;
                println!(
                    "{:-16} {:4} {}",
                    "CLOSURE",
                    constant,
                    self.read_constant(constant as usize)
                );

                let function = self.read_constant(constant as usize);
                let function = function.as_object().unwrap();
                let function = function.read();
                let function = function.as_function().unwrap();
                for _ in 0..function.upvalues().len() {
                    let is_local = self.read(offset);
                    offset += 1;
                    let index = self.read(offset);
                    offset += 1;
                    let is_local = if is_local > 0 { "local" } else { "upvalue" };
                    println!(
                        "{:04}    |                      {} {}",
                        offset - 2,
                        is_local,
                        index
                    );
                }

                offset
            }
            OpCode::GetUpvalue => self.byte_instruction("GET_UPVALUE", offset),
            OpCode::SetUpvalue => self.byte_instruction("SET_UPVALUE", offset),
            OpCode::Pop => simple_instruction("POP", offset),
            OpCode::Void => simple_instruction("VOID", offset),
            OpCode::Null => simple_instruction("NULL", offset),
            OpCode::True => simple_instruction("TRUE", offset),
            OpCode::False => simple_instruction("FALSE", offset),
            OpCode::Return => simple_instruction("RETURN", offset),
        }
    }

    pub fn emit_jump(&mut self, opcode: OpCode, line: usize) -> usize {
        self.write(opcode.into(), line);
        self.write(0xff, line);
        self.write(0xff, line);
        self.code.len() - 2
    }

    pub fn patch_jump(&mut self, offset: usize) {
        let jump = self.code.len() - offset - 2;
        if jump > u16::MAX as usize {
            // TODO: return an err instead
            panic!("Too much code to jump over");
        }

        self.code[offset] = ((jump >> 8) & 0xff) as u8;
        self.code[offset + 1] = (jump & 0xff) as u8;
    }

    /// Print a constant instruction
    fn constant_instruction(&self, name: &str, offset: usize) -> usize {
        let constant = self.read(offset + 1);
        println!(
            "{} {:4} '{}'",
            name,
            constant,
            self.read_constant(constant as usize)
        );
        offset + 2
    }

    /// Print a constant long instruction
    fn constant_long_instruction(&self, name: &str, offset: usize) -> usize {
        let constant_bits = &self.code[(offset + 1)..(offset + 4)];
        let mut constant: usize = 0;
        for (i, item) in constant_bits.iter().enumerate().take(3) {
            constant |= (*item as usize) << (8 * i);
        }
        println!("{} {:8} '{}'", name, constant, self.read_constant(constant));
        offset + 4
    }

    fn byte_instruction(&self, name: &str, offset: usize) -> usize {
        let slot = self.read(offset + 1);
        println!("{:16} {:4}", name, slot);
        offset + 2
    }

    fn jump_instruction(&self, name: &str, sign: isize, offset: usize) -> usize {
        let jump = ((self.read(offset + 1) as u16) << 8) | (self.read(offset + 2) as u16);
        println!(
            "{:-16} {:4} -> {}",
            name,
            offset,
            ((offset + 3) as isize) + sign * (jump as isize)
        );
        offset + 3
    }
}

impl<'gc> Chunk<'gc> {
    #[inline(always)]
    pub fn read_constant(&self, offset: usize) -> Value<'gc> {
        self.constants[offset]
    }

    /// Add a constant to this chunk's constant pool
    pub fn add_constant(&mut self, value: Value<'gc>) -> usize {
        self.constants.push(value);
        self.constants.len() - 1
    }

    /// Write a constant to the chunk's constant pool and emit the appropriate instruction to fetch
    /// it
    pub fn write_constant(&mut self, value: Value<'gc>, line: usize) {
        let offset = self.add_constant(value);
        if offset > 255 {
            self.write(OpCode::ConstantLong.into(), line);
            for byte in offset.to_le_bytes()[0..3].iter() {
                self.write(*byte, line);
            }
        } else {
            self.write(OpCode::Constant.into(), line);
            self.write(offset as u8, line);
        }
    }
}

/// Print a simple instruction with no operands
fn simple_instruction(name: &str, offset: usize) -> usize {
    println!("{}", name);
    offset + 1
}
