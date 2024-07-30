use crate::{
    chunk::{Chunk, OperationCode, OperationCodeConversionError},
    value::{Value, ValueContainer},
};

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

        if offset > 0 && chunk.read_line(offset) == chunk.read_line(offset - 1) {
            print!("   | ");
        } else {
            print!("{:04} ", chunk.read_line(offset));
        }

        let code = chunk
            .read_operation_code(offset)
            .map_err(|err| DebuggerError::ConversionError(err))?;
        match code {
            OperationCode::Return => Ok(Self::simple_instruction("OP_RETURN", offset, code)),
            OperationCode::Constant(constant_index) => Ok(Self::constant_instruction(
                "OP_CONSTANT",
                offset,
                constant_index,
                chunk.read_constant(constant_index),
            )),
        }
    }

    fn simple_instruction(name: &str, offset: usize, code: OperationCode) -> usize {
        println!("{}", name);
        offset + OperationCode::get_instruction_bytes_length(code)
    }

    fn constant_instruction(
        name: &str,
        offset: usize,
        constant_index: usize,
        constant_value: Value,
    ) -> usize {
        print!("{:<16} {:>4} '", name, constant_index);
        ValueContainer::print_value(constant_value);
        println!();
        offset
            + OperationCode::get_instruction_bytes_length(OperationCode::Constant(constant_index))
    }
}
