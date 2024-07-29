pub enum OperationCode {
    Return,
}

impl From<OperationCode> for u8 {
    fn from(value: OperationCode) -> Self {
        match value {
            OperationCode::Return => 0,
        }
    }
}

#[derive(Debug)]
pub struct OperationCodeConversionError(u8);

impl TryFrom<u8> for OperationCode {
    type Error = OperationCodeConversionError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(OperationCode::Return),
            _ => Err(OperationCodeConversionError(value)),
        }
    }
}

pub struct Chunk {
    instructions: Vec<u8>,
}

impl Chunk {
    const INITIAL_INSTRUCTIONS_SIZE: usize = 8;

    pub fn new() -> Self {
        let initial_instruction = Vec::with_capacity(Self::INITIAL_INSTRUCTIONS_SIZE);
        return Chunk {
            instructions: initial_instruction,
        };
    }

    pub fn add_instruction(&mut self, operation_code: OperationCode) {
        self.instructions.push(operation_code.into());
    }

    pub fn read_operation_code(
        &self,
        offset: usize,
    ) -> Result<OperationCode, OperationCodeConversionError> {
        return OperationCode::try_from(self.instructions[offset]);
    }

    pub fn get_instructions_length(&self) -> usize {
        self.instructions.len()
    }
}
