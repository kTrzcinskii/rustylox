use crate::{
    chunk::{Chunk, OperationCode, OperationCodeConversionError},
    compiler::Compiler,
    logger::Logger,
    value::{Value, ValueContainer},
};

pub enum InterpretResult {
    Ok,
    CompileError,
    RuntimeError,
}

#[derive(Debug)]
pub enum VirtualMachineError {
    InvalidInstructionFormat(OperationCodeConversionError),
    EmptyStack,
}

pub struct VirtualMachine {
    /// Index of the next instruction to be executed on given Chunk
    instruction_pointer: usize,
    /// Internal stack for holding literals
    stack: Vec<Value>,
}

struct BinaryOperationArguments {
    lhs: Value,
    rhs: Value,
}

impl VirtualMachine {
    const INITIAL_STACK_SIZE: usize = 8;

    pub fn new() -> Self {
        VirtualMachine {
            instruction_pointer: 0,
            stack: Vec::with_capacity(Self::INITIAL_STACK_SIZE),
        }
    }

    pub fn reset(&mut self) {
        self.reset_stack();
    }

    fn reset_stack(&mut self) {
        self.stack.clear();
    }

    pub fn interpret(&mut self, source: &str) -> InterpretResult {
        let compiler = Compiler::new();
        compiler.compile(source);
        InterpretResult::Ok
    }

    fn run(&mut self, chunk: &Chunk) -> Result<InterpretResult, VirtualMachineError> {
        loop {
            Logger::show_stack_content(&self.stack);
            Logger::disassemble_instruction(chunk, self.instruction_pointer).unwrap();

            let instruction = chunk
                .read_operation_code(self.instruction_pointer)
                .map_err(VirtualMachineError::InvalidInstructionFormat)?;
            self.instruction_pointer += OperationCode::get_instruction_bytes_length(&instruction);
            match instruction {
                OperationCode::Return => {
                    if let Some(value) = self.stack.pop() {
                        ValueContainer::print_value(&value);
                    }
                    return Ok(InterpretResult::Ok);
                }
                OperationCode::Constant(constant_index) => {
                    let value = chunk.read_constant(constant_index);
                    self.stack_push(value);
                }
                OperationCode::Negate => {
                    let value = self.stack_pop()?;
                    self.stack_push(-value);
                }
                OperationCode::Add => {
                    let args = self.read_binary_operation_arguments()?;
                    self.stack_push(args.lhs + args.rhs);
                }
                OperationCode::Substract => {
                    let args = self.read_binary_operation_arguments()?;
                    self.stack_push(args.lhs - args.rhs);
                }
                OperationCode::Multiply => {
                    let args = self.read_binary_operation_arguments()?;
                    self.stack_push(args.lhs * args.rhs);
                }
                OperationCode::Divide => {
                    //TODO: handle dividing by 0 (which should be impossible if our frontend runs correctly...)
                    let args = self.read_binary_operation_arguments()?;
                    self.stack_push(args.lhs / args.rhs);
                }
            }
        }
    }

    fn read_binary_operation_arguments(
        &mut self,
    ) -> Result<BinaryOperationArguments, VirtualMachineError> {
        let rhs = self.stack_pop()?;
        let lhs = self.stack_pop()?;
        Ok(BinaryOperationArguments { lhs, rhs })
    }

    fn stack_push(&mut self, value: Value) {
        self.stack.push(value);
    }

    fn stack_pop(&mut self) -> Result<Value, VirtualMachineError> {
        self.stack.pop().ok_or(VirtualMachineError::EmptyStack)
    }
}

impl Default for VirtualMachine {
    fn default() -> Self {
        Self::new()
    }
}
