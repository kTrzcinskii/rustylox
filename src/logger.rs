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
    HandlingUpvalueOutsideOfClosure,
    MissingUpvalueInsideClosure,
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
                        global_index,
                        chunk.read_constant(global_index),
                    ))
                }
                OperationCode::GetGlobal(global_index) => {
                    return Ok(Self::constant_instruction(
                        "OP_GET_GLOBAL",
                        offset,
                        global_index,
                        chunk.read_constant(global_index),
                    ))
                }
                OperationCode::SetGlobal(global_index) => {
                    return Ok(Self::constant_instruction(
                        "OP_SET_GLOBAL",
                        offset,
                        global_index,
                        chunk.read_constant(global_index),
                    ))
                }
                OperationCode::GetLocal(local_index) => {
                    return Ok(Self::byte_instruction("OP_GET_LOCAL", offset, local_index))
                }
                OperationCode::SetLocal(local_index) => {
                    return Ok(Self::byte_instruction("OP_SET_LOCAL", offset, local_index))
                }
                OperationCode::JumpIfFalse(bytes_to_skip) => {
                    return Ok(Self::jump_instruction(
                        "OP_JUMP_IF_FALSE",
                        1,
                        offset,
                        bytes_to_skip,
                    ))
                }
                OperationCode::Jump(bytes_to_skip) => {
                    return Ok(Self::jump_instruction("OP_JUMP", 1, offset, bytes_to_skip))
                }
                OperationCode::JumpIfTrue(bytes_to_skip) => {
                    return Ok(Self::jump_instruction(
                        "OP_JUMP_IF_TRUE",
                        1,
                        offset,
                        bytes_to_skip,
                    ))
                }
                OperationCode::JumpBack(bytes_to_skip) => {
                    return Ok(Self::jump_instruction(
                        "OP_JUMP_BACK",
                        -1,
                        offset,
                        bytes_to_skip,
                    ))
                }
                OperationCode::Call(arguments_count) => {
                    return Ok(Self::byte_instruction("OP_CALl", offset, arguments_count))
                }
                OperationCode::Closure(function_index) => {
                    return Ok(Self::closure_instruction(offset, function_index, &chunk)?)
                }
                OperationCode::LocalUpvalue(_) => {
                    return Err(LoggerError::HandlingUpvalueOutsideOfClosure)
                }
                OperationCode::NonLocalUpvalue(_) => {
                    return Err(LoggerError::HandlingUpvalueOutsideOfClosure)
                }
                OperationCode::GetUpvalue(upvalue_index) => {
                    return Ok(Self::byte_instruction(
                        "OP_GET_UPVALUE",
                        offset,
                        upvalue_index,
                    ))
                }
                OperationCode::SetUpvalue(upvalue_index) => {
                    return Ok(Self::byte_instruction(
                        "OP_SET_UPVALUE",
                        offset,
                        upvalue_index,
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
        constant_index: u8,
        constant_value: Value,
    ) -> usize {
        print!("{:<16} {:>4} '", name, constant_index);
        println!("{}", &constant_value);
        offset
            + OperationCode::get_instruction_bytes_length(&OperationCode::Constant(constant_index))
    }

    #[cfg(feature = "log_trace_execution")]
    fn byte_instruction(name: &str, offset: usize, index: u8) -> usize {
        println!("{:<16} {:>4}", name, index);
        offset + OperationCode::get_instruction_bytes_length(&OperationCode::GetLocal(index))
    }

    #[cfg(feature = "log_trace_execution")]
    fn jump_instruction(name: &str, direction: i16, offset: usize, bytes_to_skip: u16) -> usize {
        use crate::chunk;

        let len = chunk::JUMP_INSTRUCTION_ARGUMENT_LENGTH + 1;

        println!(
            "{:<16} {:>4} -> {}",
            name,
            offset,
            (offset + len) as i16 + (direction * bytes_to_skip as i16)
        );

        offset + len
    }

    #[cfg(feature = "log_trace_execution")]
    fn closure_instruction(
        mut offset: usize,
        index: u8,
        chunk: &Chunk,
    ) -> Result<usize, LoggerError> {
        let function = chunk.read_constant(index);
        println!("OP_CLOSURE {:>4} {}", index, function);
        offset += OperationCode::get_instruction_bytes_length(&OperationCode::Closure(index));
        let function = function
            .get_function_object()
            .expect("Closure should point to function object");
        for _ in 0..function.borrow().upvalues_count {
            let upvalue_instruction = chunk
                .read_operation_code(offset)
                .map_err(LoggerError::ConversionError)?;

            let to_print: &str;
            let index: u8;

            let mut should_increase = false;

            match upvalue_instruction {
                OperationCode::LocalUpvalue(upvalue_index) => {
                    to_print = "local";
                    index = upvalue_index;
                }
                OperationCode::NonLocalUpvalue(upvalue_index) => {
                    to_print = "upvalue";
                    index = upvalue_index;
                }
                _ => {
                    return Err(LoggerError::MissingUpvalueInsideClosure);
                }
            }

            println!(
                "{:<16}      |                     {} {}",
                offset, to_print, index
            );

            offset += 2;
        }
        Ok(offset)
    }
}
