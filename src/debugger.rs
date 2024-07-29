use crate::chunk::{Chunk, OperationCode, OperationCodeConversionError};

#[derive(Debug)]
pub enum DebuggerError {
    ConversionError(OperationCodeConversionError),
}

pub struct Debugger {}

impl Debugger {
    pub fn disassemble_chunk(chunk: &Chunk, name: &str) -> Result<(), DebuggerError> {
        println!("== {} ==", name);
        let mut offset: usize = 0;
        while offset < chunk.get_instructions_length() {
            offset = Self::disassemble_instruction(chunk, offset)?;
        }
        Ok(())
    }

    fn disassemble_instruction(chunk: &Chunk, offset: usize) -> Result<usize, DebuggerError> {
        print!("{:04} - ", offset);
        let code = chunk
            .read_operation_code(offset)
            .map_err(|err| DebuggerError::ConversionError(err))?;
        match code {
            OperationCode::Return => Ok(Self::simple_instruction("OP_RETURN", offset)),
        }
    }

    fn simple_instruction(name: &str, offset: usize) -> usize {
        println!("{}", name);
        offset + 1
    }
}
