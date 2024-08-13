use crate::{
    chunk::{Chunk, OperationCode, OperationCodeConversionError},
    compiler::Compiler,
    logger::Logger,
    table::Table,
    value::{StringObject, Value, ValueType},
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
    StackOutOfBounds,
    InvalidVariableType,
    DivideByZero,
}

pub struct VirtualMachine {
    /// Index of the next instruction to be executed on given Chunk
    instruction_pointer: usize,
    /// Internal stack for holding literals
    stack: Vec<Value>,
    /// Collection of intern strings
    strings: Option<Table>,
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
            strings: None,
        }
    }

    pub fn reset(&mut self) {
        self.reset_stack();
    }

    fn reset_stack(&mut self) {
        self.stack.clear();
    }

    pub fn interpret(&mut self, source: &str) -> InterpretResult {
        let mut compiler = Compiler::new(source);
        match compiler.compile() {
            Ok(chunk_result) => {
                self.instruction_pointer = 0;
                self.strings = Some(chunk_result.intern_strings);
                match self.run(&chunk_result.chunk) {
                    Ok(_) => InterpretResult::Ok,
                    Err(_) => InterpretResult::RuntimeError,
                }
            }
            Err(_) => InterpretResult::CompileError,
        }
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
                    return Ok(InterpretResult::Ok);
                }
                OperationCode::Constant(constant_index) => {
                    let value = chunk.read_constant(constant_index);
                    self.stack_push(value);
                }
                OperationCode::Negate => {
                    let value = self.stack_pop()?;
                    match Value::get_number(&value) {
                        Ok(num_value) => self.stack_push(Value::new_number(-num_value)),
                        Err(_) => {
                            self.runtime_error_message("Operand must be a number", chunk);
                            return Err(VirtualMachineError::InvalidVariableType);
                        }
                    }
                }
                OperationCode::Add => {
                    let args = self.read_binary_operation_arguments()?;

                    match args.lhs.get_type() {
                        ValueType::Number => match self.add_numbers(&args.lhs, &args.rhs) {
                            Ok(value) => self.stack_push(value),
                            Err(VirtualMachineError::InvalidVariableType) => {
                                self.runtime_error_message(
                                    "Both operands must be numbers or strings",
                                    chunk,
                                );
                                return Err(VirtualMachineError::InvalidVariableType);
                            }
                            Err(_) => panic!("Shouldn't raise any other type of error"),
                        },
                        ValueType::StringObject => match self.add_strings(&args.lhs, &args.rhs) {
                            Ok(value) => self.stack_push(value),
                            Err(VirtualMachineError::InvalidVariableType) => {
                                self.runtime_error_message(
                                    "Both operands must be numbers or strings",
                                    chunk,
                                );
                                return Err(VirtualMachineError::InvalidVariableType);
                            }
                            Err(_) => panic!("Shouldn't raise any other type of error"),
                        },
                        _ => {
                            self.runtime_error_message(
                                "Both operand must be numbers or strings",
                                chunk,
                            );
                            return Err(VirtualMachineError::InvalidVariableType);
                        }
                    }
                }
                OperationCode::Substract => {
                    let args = self.read_binary_operation_arguments()?;
                    match self.substract_numbers(&args.lhs, &args.rhs) {
                        Ok(value) => self.stack_push(value),
                        Err(VirtualMachineError::InvalidVariableType) => {
                            self.runtime_error_message("Both operands must be numbers", chunk);
                            return Err(VirtualMachineError::InvalidVariableType);
                        }
                        Err(_) => panic!("Shouldn't raise any other type of error"),
                    }
                }
                OperationCode::Multiply => {
                    let args = self.read_binary_operation_arguments()?;
                    match self.multiply_numbers(&args.lhs, &args.rhs) {
                        Ok(value) => self.stack_push(value),
                        Err(VirtualMachineError::InvalidVariableType) => {
                            self.runtime_error_message("Both operands must be numbers", chunk);
                            return Err(VirtualMachineError::InvalidVariableType);
                        }
                        Err(_) => panic!("Shouldn't raise any other type of error"),
                    }
                }
                OperationCode::Divide => {
                    let args = self.read_binary_operation_arguments()?;
                    match self.divide_numbers(&args.lhs, &args.rhs) {
                        Ok(value) => self.stack_push(value),
                        Err(VirtualMachineError::InvalidVariableType) => {
                            self.runtime_error_message("Both operands must be numbers", chunk);
                            return Err(VirtualMachineError::InvalidVariableType);
                        }
                        Err(VirtualMachineError::DivideByZero) => {
                            self.runtime_error_message("You cannot divide by 0", chunk);
                            return Err(VirtualMachineError::DivideByZero);
                        }
                        Err(_) => panic!("Shouldn't raise any other type of error"),
                    }
                }
                OperationCode::Nil => self.stack_push(Value::new_nil()),
                OperationCode::True => self.stack_push(Value::new_bool(true)),
                OperationCode::False => self.stack_push(Value::new_bool(false)),
                OperationCode::Not => {
                    let value = self.stack_pop()?;
                    self.stack_push(Value::new_bool(value.is_falsey()));
                }
                OperationCode::Equal => {
                    let args = self.read_binary_operation_arguments()?;
                    self.stack_push(Value::new_bool(Value::are_values_equal(
                        &args.lhs, &args.rhs,
                    )))
                }
                OperationCode::Greater => {
                    let args = self.read_binary_operation_arguments()?;
                    match self.compare_greater(&args.lhs, &args.rhs) {
                        Ok(value) => self.stack_push(Value::new_bool(value)),
                        Err(VirtualMachineError::InvalidVariableType) => {
                            self.runtime_error_message("Both operands must be numbers", chunk);
                            return Err(VirtualMachineError::InvalidVariableType);
                        }
                        Err(_) => panic!("Shouldn't raise any other type of error"),
                    }
                }
                OperationCode::Less => {
                    let args = self.read_binary_operation_arguments()?;
                    match self.compare_less(&args.lhs, &args.rhs) {
                        Ok(value) => self.stack_push(Value::new_bool(value)),
                        Err(VirtualMachineError::InvalidVariableType) => {
                            self.runtime_error_message("Both operands must be numbers", chunk);
                            return Err(VirtualMachineError::InvalidVariableType);
                        }
                        Err(_) => panic!("Shouldn't raise any other type of error"),
                    }
                }
                OperationCode::Print => {
                    let value = self.stack_pop()?;
                    println!("{}", value);
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

    fn runtime_error_message(&mut self, message: &str, chunk: &Chunk) {
        eprintln!("{}", message);

        let line = chunk.read_line(self.instruction_pointer - 1);
        eprintln!("[line {}] in script", line);
        self.reset();
    }

    fn add_numbers(&mut self, lhs: &Value, rhs: &Value) -> Result<Value, VirtualMachineError> {
        let lhs = lhs
            .get_number()
            .map_err(|_| VirtualMachineError::InvalidVariableType)?;
        let rhs = rhs
            .get_number()
            .map_err(|_| VirtualMachineError::InvalidVariableType)?;
        Ok(Value::new_number(lhs + rhs))
    }

    fn add_strings(&mut self, lhs: &Value, rhs: &Value) -> Result<Value, VirtualMachineError> {
        let lhs: &StringObject = &lhs
            .get_string_object()
            .map_err(|_| VirtualMachineError::InvalidVariableType)?
            .borrow();
        let rhs: &StringObject = &rhs
            .get_string_object()
            .map_err(|_| VirtualMachineError::InvalidVariableType)?
            .borrow();

        let mut content = String::new();
        content.push_str(lhs.get_value());
        content.push_str(rhs.get_value());
        Ok(Value::new_string_object(
            &content,
            self.strings
                .as_mut()
                .expect("Intern string should never be empty in VM after compiling stage"),
        ))
    }

    fn substract_numbers(
        &mut self,
        lhs: &Value,
        rhs: &Value,
    ) -> Result<Value, VirtualMachineError> {
        let lhs = lhs
            .get_number()
            .map_err(|_| VirtualMachineError::InvalidVariableType)?;
        let rhs = rhs
            .get_number()
            .map_err(|_| VirtualMachineError::InvalidVariableType)?;
        Ok(Value::new_number(lhs - rhs))
    }

    fn multiply_numbers(&mut self, lhs: &Value, rhs: &Value) -> Result<Value, VirtualMachineError> {
        let lhs = lhs
            .get_number()
            .map_err(|_| VirtualMachineError::InvalidVariableType)?;
        let rhs = rhs
            .get_number()
            .map_err(|_| VirtualMachineError::InvalidVariableType)?;
        Ok(Value::new_number(lhs * rhs))
    }

    fn divide_numbers(&mut self, lhs: &Value, rhs: &Value) -> Result<Value, VirtualMachineError> {
        let lhs = lhs
            .get_number()
            .map_err(|_| VirtualMachineError::InvalidVariableType)?;
        let rhs = rhs
            .get_number()
            .map_err(|_| VirtualMachineError::InvalidVariableType)?;

        if rhs == 0.0 {
            return Err(VirtualMachineError::DivideByZero);
        }

        Ok(Value::new_number(lhs / rhs))
    }

    fn compare_greater(&self, lhs: &Value, rhs: &Value) -> Result<bool, VirtualMachineError> {
        let lhs = lhs
            .get_number()
            .map_err(|_| VirtualMachineError::InvalidVariableType)?;
        let rhs = rhs
            .get_number()
            .map_err(|_| VirtualMachineError::InvalidVariableType)?;
        Ok(lhs > rhs)
    }

    fn compare_less(&self, lhs: &Value, rhs: &Value) -> Result<bool, VirtualMachineError> {
        let lhs = lhs
            .get_number()
            .map_err(|_| VirtualMachineError::InvalidVariableType)?;
        let rhs = rhs
            .get_number()
            .map_err(|_| VirtualMachineError::InvalidVariableType)?;
        Ok(lhs < rhs)
    }
}

impl Default for VirtualMachine {
    fn default() -> Self {
        Self::new()
    }
}
