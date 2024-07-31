use crate::{
    chunk::{Chunk, OperationCode, OperationCodeConversionError},
    logger::Logger,
    value::ValueContainer,
};

pub enum InterpretResult {
    Ok,
    CompileError,
    RuntimeError,
}

#[derive(Debug)]
pub enum VirtualMachineError {
    InvalidInstructionFormat(OperationCodeConversionError),
}

pub struct VirtualMachine {
    /// Index of the next instruction to be executed on given Chunk
    instruction_pointer: usize,
}

impl VirtualMachine {
    pub fn new() -> Self {
        VirtualMachine {
            instruction_pointer: 0,
        }
    }

    pub fn interpret(&mut self, chunk: &Chunk) -> InterpretResult {
        self.instruction_pointer = 0;
        self.run(chunk)
            .expect("Operations should be all in correct format")
    }

    fn run(&mut self, chunk: &Chunk) -> Result<InterpretResult, OperationCodeConversionError> {
        loop {
            Logger::disassemble_instruction(chunk, self.instruction_pointer).unwrap();

            let instruction = chunk.read_operation_code(self.instruction_pointer)?;
            self.instruction_pointer += OperationCode::get_instruction_bytes_length(&instruction);
            match instruction {
                OperationCode::Return => return Ok(InterpretResult::Ok),
                OperationCode::Constant(constant_index) => {
                    let value = chunk.read_constant(constant_index);
                    ValueContainer::print_value(&value);
                    println!();
                }
            }
        }
    }
}

impl Default for VirtualMachine {
    fn default() -> Self {
        Self::new()
    }
}
