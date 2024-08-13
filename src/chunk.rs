use crate::value::{Value, ValueContainer};

pub enum OperationCode {
    Return,
    /// Load constant operation, arguments: (constant index in `ValueContainer`)
    Constant(usize),
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
    DefineGlobal(usize),
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
        }
    }
}

impl From<OperationCode> for Vec<u8> {
    fn from(value: OperationCode) -> Self {
        match value {
            OperationCode::Return => vec![u8::from(OperationCode::Return)],
            OperationCode::Constant(constant) => {
                vec![u8::from(OperationCode::Constant(constant)), constant as u8]
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
                    global_var as u8,
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
                    < OperationCode::get_instruction_bytes_length(&OperationCode::Constant(
                        usize::MIN,
                    ))
                {
                    return Err(OperationCodeConversionError::InvalidFormat);
                }
                Ok(OperationCode::Constant(value[1] as usize))
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
                        usize::MIN,
                    ))
                {
                    return Err(OperationCodeConversionError::InvalidFormat);
                }
                Ok(OperationCode::DefineGlobal(value[1] as usize))
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
        let finish = if offset + 2 >= self.get_instructions_length() {
            self.get_instructions_length()
        } else {
            offset + 2
        };
        OperationCode::try_from(&self.instructions[offset..finish])
    }

    pub fn read_constant(&self, offset: usize) -> Value {
        self.constants.get_value(offset)
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
