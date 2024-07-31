use crate::value::{Value, ValueContainer};

pub enum OperationCode {
    /// Return operation (no arguments)
    Return,
    /// Load constant operation (1 argument - constant index in `ValueContainer`)
    Constant(usize),
}

impl OperationCode {
    pub fn get_instruction_bytes_length(code: &Self) -> usize {
        match code {
            OperationCode::Return => 1,
            OperationCode::Constant(_) => 2,
        }
    }
}

impl From<OperationCode> for u8 {
    fn from(value: OperationCode) -> Self {
        match value {
            OperationCode::Return => 0,
            OperationCode::Constant(_) => 1,
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
