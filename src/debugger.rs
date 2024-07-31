// For coniditional compiliation of debugger methods - we only want them in debug mode
#![allow(unreachable_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]

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
        #[cfg(feature = "debug_trace_execution")]
        {
            println!("== {} ==", name);
            let mut offset: usize = 0;
            while offset < chunk.get_instructions_length() {
                offset = Self::disassemble_instruction(chunk, offset)?;
            }
        }
        Ok(())
    }

    pub fn disassemble_instruction(chunk: &Chunk, offset: usize) -> Result<usize, DebuggerError> {
        #[cfg(feature = "debug_trace_execution")]
        {
            print!("{:04} - ", offset);

            if offset > 0 && chunk.read_line(offset) == chunk.read_line(offset - 1) {
                print!("   | ");
            } else {
                print!("{:04} ", chunk.read_line(offset));
            }

            let code = chunk
                .read_operation_code(offset)
                .map_err(DebuggerError::ConversionError)?;
            match code {
                OperationCode::Return => {
                    return Ok(Self::simple_instruction("OP_RETURN", offset, code))
                }
                OperationCode::Constant(constant_index) => {
                    return Ok(Self::constant_instruction(
                        "OP_CONSTANT",
                        offset,
                        constant_index,
                        chunk.read_constant(constant_index),
                    ))
                }
            }
        }
        Ok(0)
    }

    #[cfg(feature = "debug_trace_execution")]
    fn simple_instruction(name: &str, offset: usize, code: OperationCode) -> usize {
        println!("{}", name);
        offset + OperationCode::get_instruction_bytes_length(&code)
    }

    #[cfg(feature = "debug_trace_execution")]
    fn constant_instruction(
        name: &str,
        offset: usize,
        constant_index: usize,
        constant_value: Value,
    ) -> usize {
        print!("{:<16} {:>4} '", name, constant_index);
        ValueContainer::print_value(&constant_value);
        println!();
        offset
            + OperationCode::get_instruction_bytes_length(&OperationCode::Constant(constant_index))
    }
}
