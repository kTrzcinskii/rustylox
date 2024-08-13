// For coniditional compiliation of debugger methods - we only want them in debug mode
#![allow(unreachable_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]

use crate::{
    chunk::{Chunk, OperationCode, OperationCodeConversionError},
    value::{Value, ValueContainer},
};

#[derive(Debug)]
pub enum LoggerError {
    ConversionError(OperationCodeConversionError),
}

pub struct Logger {}

impl Logger {
    pub fn disassemble_chunk(chunk: &Chunk, name: &str) -> Result<(), LoggerError> {
        #[cfg(feature = "log_trace_execution")]
        {
            println!("== {} ==", name);
            let mut offset: usize = 0;
            while offset < chunk.get_instructions_length() {
                offset = Self::disassemble_instruction(chunk, offset)?;
            }
        }
        Ok(())
    }

    pub fn disassemble_instruction(chunk: &Chunk, offset: usize) -> Result<usize, LoggerError> {
        #[cfg(feature = "log_trace_execution")]
        {
            print!("{:04} - ", offset);

            if offset > 0 && chunk.read_line(offset) == chunk.read_line(offset - 1) {
                print!("   | ");
            } else {
                print!("{:04} ", chunk.read_line(offset));
            }

            let code = chunk
                .read_operation_code(offset)
                .map_err(LoggerError::ConversionError)?;
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
                OperationCode::Negate => {
                    return Ok(Self::simple_instruction("OP_NEGATE", offset, code))
                }
                OperationCode::Add => return Ok(Self::simple_instruction("OP_ADD", offset, code)),
                OperationCode::Substract => {
                    return Ok(Self::simple_instruction("OP_SUBTRACT", offset, code))
                }
                OperationCode::Multiply => {
                    return Ok(Self::simple_instruction("OP_MULTIPLY", offset, code))
                }
                OperationCode::Divide => {
                    return Ok(Self::simple_instruction("OP_DIVIDE", offset, code))
                }
                OperationCode::Nil => {
                    return Ok(Self::simple_instruction("OP_NIL", offset, code));
                }
                OperationCode::True => {
                    return Ok(Self::simple_instruction("OP_TRUE", offset, code));
                }
                OperationCode::False => {
                    return Ok(Self::simple_instruction("OP_FALSE", offset, code));
                }
                OperationCode::Not => {
                    return Ok(Self::simple_instruction("OP_NOT", offset, code));
                }
                OperationCode::Equal => {
                    return Ok(Self::simple_instruction("OP_EQUAL", offset, code));
                }
                OperationCode::Greater => {
                    return Ok(Self::simple_instruction("OP_GREATER", offset, code));
                }
                OperationCode::Less => {
                    return Ok(Self::simple_instruction("OP_LESS", offset, code));
                }
                OperationCode::Print => {
                    return Ok(Self::simple_instruction("OP_PRINT", offset, code));
                }
                OperationCode::PopStack => {
                    return Ok(Self::simple_instruction("OP_POP_STACK", offset, code));
                }
                OperationCode::DefineGlobal(global_index) => {
                    return Ok(Self::constant_instruction(
                        "OP_DEFINE_GLOBAL",
                        offset,
                        constant_index,
                        chunk.read_constant(constant_index),
                    ))
                }
            }
        }
        Ok(0)
    }

    pub fn show_stack_content(stack: &[Value]) {
        #[cfg(feature = "log_trace_execution")]
        {
            print!("[ ");
            for v in stack {
                print!("{} ", v);
            }
            println!("]");
        }
    }

    #[cfg(feature = "log_trace_execution")]
    fn simple_instruction(name: &str, offset: usize, code: OperationCode) -> usize {
        println!("{}", name);
        offset + OperationCode::get_instruction_bytes_length(&code)
    }

    #[cfg(feature = "log_trace_execution")]
    fn constant_instruction(
        name: &str,
        offset: usize,
        constant_index: usize,
        constant_value: Value,
    ) -> usize {
        print!("{:<16} {:>4} '", name, constant_index);
        println!("{}", &constant_value);
        offset
            + OperationCode::get_instruction_bytes_length(&OperationCode::Constant(constant_index))
    }
}
