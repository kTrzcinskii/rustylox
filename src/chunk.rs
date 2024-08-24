use crate::value::{Value, ValueContainer};

const LONGEST_INSTRUCTION_LENGTH: usize = 3;

pub const JUMP_INSTRUCTION_ARGUMENT_LENGTH: usize = 2;

pub enum OperationCode {
    Return,
    /// Load constant operation, arguments: (constant index in `ValueContainer`)
    Constant(u8),
    Nil,
    True,
    False,
    Not,
    Negate,
    Add,
    Substract,
    Multiply,
    Divide,
    Equal,
    Greater,
    Less,
    Print,
    PopStack,
    /// Define global variable, arguments: (variable index in `ValueContainer`)
    DefineGlobal(u8),
    /// Get global variable, arguments: (variable index in `ValueContainer`)
    GetGlobal(u8),
    /// Set global variable, arguments: (variable index in `ValueContainer`)
    SetGlobal(u8),
    /// Get local variable, arguments: (variable index in `ValueContainer`)
    GetLocal(u8),
    /// Set local variable, arguments: (variable index in `ValueContainer`)
    SetLocal(u8),
    /// Jump if stack top is false, arguments: (number of bytes to skip)
    JumpIfFalse(u16),
    /// Jump unconditionally, arguments: (number of bytes to skip)
    Jump(u16),
    /// Jump if stack top is true, arguments: (number of bytes to skip)
    JumpIfTrue(u16),
    /// Jump back unconditionally, arguments: (number of bytes to skip)
    JumpBack(u16),
    /// Call function/method, arguments: (number of call arguments)
    Call(u8),
    /// Create closure from function, arguments: (function index in `ValueContainer`)
    Closure(u8),
}

impl OperationCode {
    pub fn get_instruction_bytes_length(code: &Self) -> usize {
        match code {
            OperationCode::Return => 1,
            OperationCode::Constant(_) => 2,
            OperationCode::Negate => 1,
            OperationCode::Add => 1,
            OperationCode::Substract => 1,
            OperationCode::Multiply => 1,
            OperationCode::Divide => 1,
            OperationCode::Nil => 1,
            OperationCode::True => 1,
            OperationCode::False => 1,
            OperationCode::Not => 1,
            OperationCode::Equal => 1,
            OperationCode::Greater => 1,
            OperationCode::Less => 1,
            OperationCode::Print => 1,
            OperationCode::PopStack => 1,
            OperationCode::DefineGlobal(_) => 2,
            OperationCode::GetGlobal(_) => 2,
            OperationCode::SetGlobal(_) => 2,
            OperationCode::GetLocal(_) => 2,
            OperationCode::SetLocal(_) => 2,
            OperationCode::JumpIfFalse(_) => 3,
            OperationCode::Jump(_) => 3,
            OperationCode::JumpIfTrue(_) => 3,
            OperationCode::JumpBack(_) => 3,
            OperationCode::Call(_) => 2,
            OperationCode::Closure(_) => 2,
        }
    }
}

impl From<OperationCode> for u8 {
    fn from(value: OperationCode) -> Self {
        match value {
            OperationCode::Return => 0,
            OperationCode::Constant(_) => 1,
            OperationCode::Negate => 2,
            OperationCode::Add => 3,
            OperationCode::Substract => 4,
            OperationCode::Multiply => 5,
            OperationCode::Divide => 6,
            OperationCode::Nil => 7,
            OperationCode::True => 8,
            OperationCode::False => 9,
            OperationCode::Not => 10,
            OperationCode::Equal => 11,
            OperationCode::Greater => 12,
            OperationCode::Less => 13,
            OperationCode::Print => 14,
            OperationCode::PopStack => 15,
            OperationCode::DefineGlobal(_) => 16,
            OperationCode::GetGlobal(_) => 17,
            OperationCode::SetGlobal(_) => 18,
            OperationCode::GetLocal(_) => 19,
            OperationCode::SetLocal(_) => 20,
            OperationCode::JumpIfFalse(_) => 21,
            OperationCode::Jump(_) => 22,
            OperationCode::JumpIfTrue(_) => 23,
            OperationCode::JumpBack(_) => 24,
            OperationCode::Call(_) => 25,
            OperationCode::Closure(_) => 26,
        }
    }
}

impl From<OperationCode> for Vec<u8> {
    fn from(value: OperationCode) -> Self {
        match value {
            OperationCode::Return => vec![u8::from(OperationCode::Return)],
            OperationCode::Constant(constant) => {
                vec![u8::from(OperationCode::Constant(constant)), constant]
            }
            OperationCode::Negate => vec![u8::from(OperationCode::Negate)],
            OperationCode::Add => vec![u8::from(OperationCode::Add)],
            OperationCode::Substract => vec![u8::from(OperationCode::Substract)],
            OperationCode::Multiply => vec![u8::from(OperationCode::Multiply)],
            OperationCode::Divide => vec![u8::from(OperationCode::Divide)],
            OperationCode::Nil => vec![u8::from(OperationCode::Nil)],
            OperationCode::True => vec![u8::from(OperationCode::True)],
            OperationCode::False => vec![u8::from(OperationCode::False)],
            OperationCode::Not => vec![u8::from(OperationCode::Not)],
            OperationCode::Equal => vec![u8::from(OperationCode::Equal)],
            OperationCode::Greater => vec![u8::from(OperationCode::Greater)],
            OperationCode::Less => vec![u8::from(OperationCode::Less)],
            OperationCode::Print => vec![u8::from(OperationCode::Print)],
            OperationCode::PopStack => vec![u8::from(OperationCode::PopStack)],
            OperationCode::DefineGlobal(global_var) => {
                vec![
                    u8::from(OperationCode::DefineGlobal(global_var)),
                    global_var,
                ]
            }
            OperationCode::GetGlobal(global_var) => {
                vec![u8::from(OperationCode::GetGlobal(global_var)), global_var]
            }
            OperationCode::SetGlobal(global_var) => {
                vec![u8::from(OperationCode::SetGlobal(global_var)), global_var]
            }
            OperationCode::GetLocal(local_var) => {
                vec![u8::from(OperationCode::GetLocal(local_var)), local_var]
            }
            OperationCode::SetLocal(local_var) => {
                vec![u8::from(OperationCode::SetLocal(local_var)), local_var]
            }
            OperationCode::JumpIfFalse(bytes_to_skip) => {
                let number_in_bytes = bytes_to_skip.to_ne_bytes();
                vec![
                    u8::from(OperationCode::JumpIfFalse(bytes_to_skip)),
                    number_in_bytes[0],
                    number_in_bytes[1],
                ]
            }
            OperationCode::Jump(bytes_to_skip) => {
                let number_in_bytes = bytes_to_skip.to_ne_bytes();
                vec![
                    u8::from(OperationCode::Jump(bytes_to_skip)),
                    number_in_bytes[0],
                    number_in_bytes[1],
                ]
            }
            OperationCode::JumpIfTrue(bytes_to_skip) => {
                let number_in_bytes = bytes_to_skip.to_ne_bytes();
                vec![
                    u8::from(OperationCode::JumpIfTrue(bytes_to_skip)),
                    number_in_bytes[0],
                    number_in_bytes[1],
                ]
            }
            OperationCode::JumpBack(bytes_to_skip) => {
                let number_in_bytes = bytes_to_skip.to_ne_bytes();
                vec![
                    u8::from(OperationCode::JumpBack(bytes_to_skip)),
                    number_in_bytes[0],
                    number_in_bytes[1],
                ]
            }
            OperationCode::Call(arguments_count) => {
                vec![
                    u8::from(OperationCode::Call(arguments_count)),
                    arguments_count,
                ]
            }
            OperationCode::Closure(function_index) => {
                vec![
                    u8::from(OperationCode::Closure(function_index)),
                    function_index,
                ]
            }
        }
    }
}

#[derive(Debug)]
pub enum OperationCodeConversionError {
    InvalidValue(u8),
    EmptyInput,
    InvalidFormat,
}

impl TryFrom<&[u8]> for OperationCode {
    type Error = OperationCodeConversionError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        if value.is_empty() {
            return Err(OperationCodeConversionError::EmptyInput);
        }
        match value[0] {
            0 => Ok(OperationCode::Return),
            1 => {
                if value.len()
                    < OperationCode::get_instruction_bytes_length(&OperationCode::Constant(u8::MIN))
                {
                    return Err(OperationCodeConversionError::InvalidFormat);
                }
                Ok(OperationCode::Constant(value[1]))
            }
            2 => Ok(OperationCode::Negate),
            3 => Ok(OperationCode::Add),
            4 => Ok(OperationCode::Substract),
            5 => Ok(OperationCode::Multiply),
            6 => Ok(OperationCode::Divide),
            7 => Ok(OperationCode::Nil),
            8 => Ok(OperationCode::True),
            9 => Ok(OperationCode::False),
            10 => Ok(OperationCode::Not),
            11 => Ok(OperationCode::Equal),
            12 => Ok(OperationCode::Greater),
            13 => Ok(OperationCode::Less),
            14 => Ok(OperationCode::Print),
            15 => Ok(OperationCode::PopStack),
            16 => {
                if value.len()
                    < OperationCode::get_instruction_bytes_length(&OperationCode::DefineGlobal(
                        u8::MIN,
                    ))
                {
                    return Err(OperationCodeConversionError::InvalidFormat);
                }
                Ok(OperationCode::DefineGlobal(value[1]))
            }
            17 => {
                if value.len()
                    < OperationCode::get_instruction_bytes_length(&OperationCode::GetGlobal(
                        u8::MIN,
                    ))
                {
                    return Err(OperationCodeConversionError::InvalidFormat);
                }
                Ok(OperationCode::GetGlobal(value[1]))
            }
            18 => {
                if value.len()
                    < OperationCode::get_instruction_bytes_length(&OperationCode::SetGlobal(
                        u8::MIN,
                    ))
                {
                    return Err(OperationCodeConversionError::InvalidFormat);
                }
                Ok(OperationCode::SetGlobal(value[1]))
            }
            19 => {
                if value.len()
                    < OperationCode::get_instruction_bytes_length(&OperationCode::GetLocal(u8::MIN))
                {
                    return Err(OperationCodeConversionError::InvalidFormat);
                }
                Ok(OperationCode::GetLocal(value[1]))
            }
            20 => {
                if value.len()
                    < OperationCode::get_instruction_bytes_length(&OperationCode::SetLocal(u8::MIN))
                {
                    return Err(OperationCodeConversionError::InvalidFormat);
                }
                Ok(OperationCode::SetLocal(value[1]))
            }
            21 => {
                if value.len()
                    < OperationCode::get_instruction_bytes_length(&OperationCode::JumpIfFalse(
                        u16::MIN,
                    ))
                {
                    return Err(OperationCodeConversionError::InvalidFormat);
                }
                let bytes_to_skip = u16::from_ne_bytes([value[1], value[2]]);
                Ok(OperationCode::JumpIfFalse(bytes_to_skip))
            }
            22 => {
                if value.len()
                    < OperationCode::get_instruction_bytes_length(&OperationCode::Jump(u16::MIN))
                {
                    return Err(OperationCodeConversionError::InvalidFormat);
                }
                let bytes_to_skip = u16::from_ne_bytes([value[1], value[2]]);
                Ok(OperationCode::Jump(bytes_to_skip))
            }
            23 => {
                if value.len()
                    < OperationCode::get_instruction_bytes_length(&OperationCode::JumpIfTrue(
                        u16::MIN,
                    ))
                {
                    return Err(OperationCodeConversionError::InvalidFormat);
                }
                let bytes_to_skip = u16::from_ne_bytes([value[1], value[2]]);
                Ok(OperationCode::JumpIfTrue(bytes_to_skip))
            }
            24 => {
                if value.len()
                    < OperationCode::get_instruction_bytes_length(&OperationCode::JumpBack(
                        u16::MAX,
                    ))
                {
                    return Err(OperationCodeConversionError::InvalidFormat);
                }
                let bytes_to_skip = u16::from_ne_bytes([value[1], value[2]]);
                Ok(OperationCode::JumpBack(bytes_to_skip))
            }
            25 => {
                if value.len()
                    < OperationCode::get_instruction_bytes_length(&OperationCode::Call(u8::MAX))
                {
                    return Err(OperationCodeConversionError::InvalidFormat);
                }
                Ok(OperationCode::Call(value[1]))
            }
            26 => {
                if value.len()
                    < OperationCode::get_instruction_bytes_length(&OperationCode::Closure(u8::MAX))
                {
                    return Err(OperationCodeConversionError::InvalidFormat);
                }
                Ok(OperationCode::Closure(value[1]))
            }
            _ => Err(OperationCodeConversionError::InvalidValue(value[0])),
        }
    }
}

pub struct Chunk {
    /// bytecode instructions
    instructions: Vec<u8>,
    /// for each bytecode instruction, it tells from which line in user program it is
    lines: Vec<usize>,
    /// array of constatns
    constants: ValueContainer,
}

#[derive(Debug)]
pub enum ChunkError {
    PatchingNotJumpInstruction,
    PatchingOutsideOfbounds,
}

impl Chunk {
    const INITIAL_INSTRUCTIONS_SIZE: usize = 8;

    pub fn new() -> Self {
        let initial_instruction = Vec::with_capacity(Self::INITIAL_INSTRUCTIONS_SIZE);
        let initial_lines = Vec::with_capacity(Self::INITIAL_INSTRUCTIONS_SIZE);
        let initial_constants = ValueContainer::new();
        Chunk {
            instructions: initial_instruction,
            lines: initial_lines,
            constants: initial_constants,
        }
    }

    pub fn add_instruction(&mut self, operation_code: OperationCode, line: usize) {
        let bytes: Vec<u8> = operation_code.into();
        self.instructions.extend_from_slice(&bytes);
        self.lines.extend_from_slice(&(vec![line; bytes.len()]));
    }

    pub fn patch_jump_instruction(
        &mut self,
        instruction: OperationCode,
        instruction_index: usize,
        bytes_to_skip: u16,
    ) -> Result<(), ChunkError> {
        if instruction_index >= self.get_instructions_length() {
            return Err(ChunkError::PatchingOutsideOfbounds);
        }
        if self.instructions[instruction_index] != u8::from(instruction) {
            return Err(ChunkError::PatchingNotJumpInstruction);
        }
        let bytes = bytes_to_skip.to_ne_bytes();
        self.instructions[instruction_index + 1] = bytes[0];
        self.instructions[instruction_index + 2] = bytes[1];
        Ok(())
    }

    /// return index at which constant is stored
    pub fn add_constant(&mut self, constant: Value) -> usize {
        self.constants.add_value(constant);
        self.constants.get_values_length() - 1
    }

    pub fn read_operation_code(
        &self,
        offset: usize,
    ) -> Result<OperationCode, OperationCodeConversionError> {
        // For now we will just return the slice starting from offset and taking as much bytes as much the longest instruction can take
        // May be in the future it should be somehow improved
        let finish = if offset + LONGEST_INSTRUCTION_LENGTH >= self.get_instructions_length() {
            self.get_instructions_length()
        } else {
            offset + LONGEST_INSTRUCTION_LENGTH
        };
        OperationCode::try_from(&self.instructions[offset..finish])
    }

    pub fn read_constant(&self, offset: u8) -> Value {
        self.constants.get_value(offset as usize)
    }

    pub fn read_line(&self, offset: usize) -> usize {
        self.lines[offset]
    }

    pub fn get_instructions_length(&self) -> usize {
        self.instructions.len()
    }
}

impl Default for Chunk {
    fn default() -> Self {
        Self::new()
    }
}
