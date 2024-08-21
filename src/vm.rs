use std::{cell::RefCell, rc::Rc};

use crate::{
    chunk::{OperationCode, OperationCodeConversionError},
    compiler::{Compiler, FunctionType},
    logger::Logger,
    table::{InsertResult, Table},
    value::{FunctionObject, StringObject, Value, ValueType},
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
    InvalidVariableNameType,
    UndefinedVariable,
    CallOnNotCallable,
}

struct CallFrame {
    /// Function that was called
    function: Rc<RefCell<FunctionObject>>,
    /// Index of next to execute instruction in function chunk
    instruction_pointer: usize,
    /// Index of stack where frame local variables start
    stack_start: u8,
}

pub struct VirtualMachine {
    /// Internal stack holding current function invocations stack
    frames: Vec<CallFrame>,
    /// Internal stack for holding literals
    stack: Vec<Value>,
    /// Collection of intern strings
    strings: Table,
    /// Collection of global variables
    globals: Table,
}

struct BinaryOperationArguments {
    lhs: Value,
    rhs: Value,
}

impl VirtualMachine {
    const INITIAL_STACK_SIZE: usize = 8;

    pub fn new() -> Self {
        VirtualMachine {
            frames: Vec::with_capacity(Self::INITIAL_STACK_SIZE),
            stack: Vec::with_capacity(Self::INITIAL_STACK_SIZE),
            strings: Table::new(),
            globals: Table::new(),
        }
    }

    pub fn reset(&mut self) {
        self.reset_stack();
    }

    fn reset_stack(&mut self) {
        self.stack.clear();
    }

    pub fn interpret(&mut self, source: &str) -> InterpretResult {
        let mut compiler = Compiler::new(source, FunctionType::Script);
        let compile_result = compiler.compile(&mut self.strings);
        match compile_result {
            Ok(function) => {
                self.stack_push(Value::from(function.clone()));
                // Calling our implicit main which wraps the whole program
                self.handle_function_call(function, 0);
                let result = match self.run() {
                    Ok(_) => InterpretResult::Ok,
                    Err(_) => InterpretResult::RuntimeError,
                };
                // TODO: after implementing whole function logic check if this makes sense
                self.stack_pop().expect("Should remove first element - the global script function - to leave stack empty for future repl interpreting");
                result
            }
            Err(_) => InterpretResult::CompileError,
        }
    }

    fn run(&mut self) -> Result<InterpretResult, VirtualMachineError> {
        // TODO: properly wire it up with the rest call frames logic in the future
        let mut frame = self.frames.pop().expect("Shouldn't be empty.");
        loop {
            Logger::show_stack_content(&self.stack);
            Logger::disassemble_instruction(
                &frame.function.borrow().chunk,
                frame.instruction_pointer,
            )
            .unwrap();

            let instruction = frame
                .function
                .borrow_mut()
                .chunk
                .read_operation_code(frame.instruction_pointer)
                .map_err(VirtualMachineError::InvalidInstructionFormat)?;
            frame.instruction_pointer += OperationCode::get_instruction_bytes_length(&instruction);
            match instruction {
                OperationCode::Return => {
                    return Ok(InterpretResult::Ok);
                }
                OperationCode::Constant(constant_index) => {
                    let value = frame
                        .function
                        .borrow_mut()
                        .chunk
                        .read_constant(constant_index);
                    self.stack_push(value);
                }
                OperationCode::Negate => {
                    let value = self.stack_pop()?;
                    match Value::get_number(&value) {
                        Ok(num_value) => self.stack_push(Value::new_number(-num_value)),
                        Err(_) => {
                            self.runtime_error_message("Operand must be a number", &frame);
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
                                    &frame,
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
                                    &frame,
                                );
                                return Err(VirtualMachineError::InvalidVariableType);
                            }
                            Err(_) => panic!("Shouldn't raise any other type of error"),
                        },
                        _ => {
                            self.runtime_error_message(
                                "Both operand must be numbers or strings",
                                &frame,
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
                            self.runtime_error_message("Both operands must be numbers", &frame);
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
                            self.runtime_error_message("Both operands must be numbers", &frame);
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
                            self.runtime_error_message("Both operands must be numbers", &frame);
                            return Err(VirtualMachineError::InvalidVariableType);
                        }
                        Err(VirtualMachineError::DivideByZero) => {
                            self.runtime_error_message("You cannot divide by 0", &frame);
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
                            self.runtime_error_message("Both operands must be numbers", &frame);
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
                            self.runtime_error_message("Both operands must be numbers", &frame);
                            return Err(VirtualMachineError::InvalidVariableType);
                        }
                        Err(_) => panic!("Shouldn't raise any other type of error"),
                    }
                }
                OperationCode::Print => {
                    let value = self.stack_pop()?;
                    println!("{}", value);
                }
                OperationCode::PopStack => {
                    self.stack_pop()?;
                }
                OperationCode::DefineGlobal(global_var_index) => {
                    let name = frame
                        .function
                        .borrow_mut()
                        .chunk
                        .read_constant(global_var_index);
                    match name.get_string_object() {
                        // Popping only after the value is added to globals is by design.
                        // It's done this way to ensue the VM can still find the value
                        // even if garbage collection is triggered right in the middle of adding to the `Table`
                        Ok(var_name) => {
                            self.globals
                                .insert(var_name.clone(), self.stack_peek(0)?.clone());
                            self.stack_pop()?;
                        }
                        Err(_) => return Err(VirtualMachineError::InvalidVariableNameType),
                    }
                }
                OperationCode::GetGlobal(global_var_index) => {
                    let name = frame
                        .function
                        .borrow_mut()
                        .chunk
                        .read_constant(global_var_index);
                    let name_string_object = name
                        .get_string_object()
                        .map_err(|_| VirtualMachineError::InvalidVariableNameType)?;
                    match self.globals.get(name_string_object) {
                        Ok(value) => self.stack_push(value.clone()),
                        Err(_) => {
                            self.runtime_error_message(
                                &format!(
                                    "Undefined variable '{}'",
                                    name_string_object.borrow().get_value()
                                ),
                                &frame,
                            );
                            return Err(VirtualMachineError::UndefinedVariable);
                        }
                    }
                }
                OperationCode::SetGlobal(global_var_index) => {
                    let name = frame
                        .function
                        .borrow_mut()
                        .chunk
                        .read_constant(global_var_index);
                    let name_string_object = name
                        .get_string_object()
                        .map_err(|_| VirtualMachineError::InvalidVariableNameType)?;
                    match self
                        .globals
                        .insert(name_string_object.clone(), self.stack_peek(0)?.clone())
                    {
                        InsertResult::Added => {
                            // The variable hasn't been defined yet - it's runtime error, as there is no implicit variable declaration in lox
                            // We delete this, as we don't want to keep this "zombie" variable to provide better REPL experience
                            self.globals.remove(name_string_object).expect("It should always be possible to remove element that was just added");
                            self.runtime_error_message(
                                &format!(
                                    "Undefined variable '{}'.",
                                    name_string_object.borrow().get_value()
                                ),
                                &frame,
                            );
                            return Err(VirtualMachineError::UndefinedVariable);
                        }
                        InsertResult::Replaced => {}
                    }
                }
                OperationCode::GetLocal(local_var_index) => {
                    // We must push it even though it's already on the stack as other instructions
                    // read data only from the top of the stack
                    self.stack_push(
                        self.stack[(frame.stack_start + local_var_index) as usize].clone(),
                    );
                }
                OperationCode::SetLocal(local_var_index) => {
                    self.stack[(frame.stack_start + local_var_index) as usize] = self.stack_peek(0).expect("Index of local var in the stack should be correct as the same index is used in compiler locals").clone();
                }
                OperationCode::JumpIfFalse(bytes_to_skip) => {
                    if self
                        .stack_peek(0)
                        .expect("Stack shouldn't be empty during conditional jump operation")
                        .is_falsey()
                    {
                        frame.instruction_pointer += bytes_to_skip as usize;
                    }
                }
                OperationCode::Jump(bytes_to_skip) => {
                    frame.instruction_pointer += bytes_to_skip as usize;
                }
                OperationCode::JumpIfTrue(bytes_to_skip) => {
                    if !self
                        .stack_peek(0)
                        .expect("Stack shouldn't be empty during conditional jump operation")
                        .is_falsey()
                    {
                        frame.instruction_pointer += bytes_to_skip as usize;
                    }
                }
                OperationCode::JumpBack(bytes_to_skip) => {
                    frame.instruction_pointer -= bytes_to_skip as usize;
                }
                OperationCode::Call(arguments_count) => {
                    // We know that on the stack its always like:
                    // <function> arg0 arg1 arg2 ...
                    // So peeking arguments_count always gets us the function itself from the stack
                    self.handle_call_value(
                        self.stack_peek(arguments_count as usize)?.clone(),
                        arguments_count,
                    )?;
                    frame = self.swap_call_frames_top(frame);
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

    fn stack_peek(&self, distance: usize) -> Result<&Value, VirtualMachineError> {
        let index = self.stack.len() - 1 - distance;
        if self.stack.len() < 1 + distance {
            return Err(VirtualMachineError::StackOutOfBounds);
        }
        Ok(&self.stack[index])
    }

    fn runtime_error_message(&mut self, message: &str, frame: &CallFrame) {
        eprintln!("{}", message);

        let line = frame
            .function
            .borrow()
            .chunk
            .read_line(frame.instruction_pointer - 1);
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
        Ok(Value::new_string_object(&content, &mut self.strings))
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

    // Logic here is that in current_frame we store currently executed frame
    // When we call new function, we want to put current frame back on stack, and return the call frame of the new function
    // We know that at the point of calling this, the frame is already on the stack (or at least should be)
    fn swap_call_frames_top(&mut self, current_frame: CallFrame) -> CallFrame {
        let top = self.frames.pop().unwrap();
        self.frames.push(current_frame);
        top
    }

    fn handle_call_value(
        &mut self,
        callee: Value,
        arguments_count: u8,
    ) -> Result<(), VirtualMachineError> {
        match callee.get_type() {
            ValueType::FunctionObject => {
                self.handle_function_call(
                    callee.get_function_object().unwrap().clone(),
                    arguments_count,
                );
                Ok(())
            }
            _ => Err(VirtualMachineError::CallOnNotCallable),
        }
    }

    fn handle_function_call(&mut self, function: Rc<RefCell<FunctionObject>>, arguments_count: u8) {
        let function_frame = CallFrame {
            function,
            instruction_pointer: 0,
            // We do it so that for frame it seems stack start at functions's position,
            // as we have: <function <arg1> <arg2> ... <argN> <STACK_TOP>
            // so from stack top we must substract (n + 1)
            stack_start: self.stack.len() as u8 - (arguments_count + 1),
        };
        self.frames.push(function_frame);
    }
}

impl Default for VirtualMachine {
    fn default() -> Self {
        Self::new()
    }
}
