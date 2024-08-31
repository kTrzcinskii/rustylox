use std::{cell::RefCell, collections::BTreeSet, rc::Rc};

use crate::{
    chunk::{OperationCode, OperationCodeConversionError},
    compiler::{Compiler, FunctionType, INITIALIZER_NAME},
    logger::Logger,
    native_functions,
    table::{InsertResult, Table},
    value::{
        ClassObject, ClosureObject, NativeFunction, StringObject, UpvalueObject,
        UpvalueObjectBTreeWrapper, Value, ValueType,
    },
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
    InvalidArgumentsCount,
    HandlingUpvalueOutsideOfClosure,
    NotEnoughUpvaluesInClosure,
    UpvalueIncorrectFieldAccess,
    PropertyOutsideInstance,
    UndefinedProperty,
    HandlingMethodWithoutClass,
}

struct CallFrame {
    /// Closure that was called
    closure: Rc<RefCell<ClosureObject>>,
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
    /// Collection of all upvalues that points to variables that are still on the stack
    open_upvalues: BTreeSet<UpvalueObjectBTreeWrapper>,
    /// Special string to lookup initializer method in class
    init_string: Rc<RefCell<StringObject>>,
}

struct BinaryOperationArguments {
    lhs: Value,
    rhs: Value,
}

impl VirtualMachine {
    const INITIAL_STACK_SIZE: usize = 8;

    pub fn new() -> Self {
        let mut strings = Table::new();
        let init_string = Value::new_string_object(INITIALIZER_NAME, &mut strings);

        let mut vm = VirtualMachine {
            frames: Vec::with_capacity(Self::INITIAL_STACK_SIZE),
            stack: Vec::with_capacity(Self::INITIAL_STACK_SIZE),
            strings,
            globals: Table::new(),
            open_upvalues: BTreeSet::new(),
            init_string: init_string.get_string_object().unwrap().clone(),
        };

        vm.define_native_function("clock", native_functions::clock_native);

        vm
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
                // Pushing on the stack for GC reasons
                self.stack_push(Value::from(function.clone()));
                let main_closure = ClosureObject::new_rc(function);
                self.stack_pop().unwrap();
                self.stack_push(Value::from(main_closure.clone()));
                // Calling our implicit main which wraps the whole program
                self.handle_function_call(main_closure, 0, None)
                    .expect("Should never fail, as fail can only be by invalid arguments count");
                match self.run() {
                    Ok(_) => InterpretResult::Ok,
                    Err(_) => InterpretResult::RuntimeError,
                }
            }
            Err(_) => InterpretResult::CompileError,
        }
    }

    fn run(&mut self) -> Result<InterpretResult, VirtualMachineError> {
        let mut frame = self.frames.pop().expect("Shouldn't be empty.");
        loop {
            Logger::show_stack_content(&self.stack);
            Logger::disassemble_instruction(
                &frame.closure.borrow().function.borrow().chunk,
                frame.instruction_pointer,
            )
            .unwrap();

            let instruction = frame
                .closure
                .borrow_mut()
                .function
                .borrow_mut()
                .chunk
                .read_operation_code(frame.instruction_pointer)
                .map_err(VirtualMachineError::InvalidInstructionFormat)?;
            frame.instruction_pointer += OperationCode::get_instruction_bytes_length(&instruction);
            match instruction {
                OperationCode::Return => {
                    let result = self.stack_pop().expect(
                        "When returning from function there should be result value on the stack",
                    );
                    // Close every upvalue owned by returning function
                    self.close_upvalue(frame.stack_start as usize);
                    // We executed all the frames (including the "implicit" main one) - it's time to finish
                    if self.frames.is_empty() {
                        self.stack_pop().expect("When finish program there should be the global script on the stack that must be removed");
                        return Ok(InterpretResult::Ok);
                    }
                    // Remove function itself, function arguments and function local variables from the stack (so we want to remove everything that starts in the stack_start of frame)
                    self.stack.truncate(frame.stack_start as usize);
                    frame = self.frames.pop().expect("Shouldn't be empty");
                    // Push result back on stack to make it available for outter function
                    self.stack_push(result);
                }
                OperationCode::Constant(constant_index) => {
                    let value = frame
                        .closure
                        .borrow_mut()
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
                        .closure
                        .borrow()
                        .function
                        .borrow()
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
                        .closure
                        .borrow()
                        .function
                        .borrow()
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
                        .closure
                        .borrow()
                        .function
                        .borrow()
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
                    let callee = self.stack_peek(arguments_count as usize)?.clone();
                    // We don't use frames with native functions, as we let rust handle them
                    let should_swap_frames = self.should_swap_frames(&callee);
                    self.handle_call_value(callee, arguments_count, &frame)?;
                    if should_swap_frames {
                        frame = self.swap_call_frames_top(frame);
                    }
                }
                OperationCode::Closure(function_index) => {
                    let function = frame
                        .closure
                        .borrow()
                        .function
                        .borrow()
                        .chunk
                        .read_constant(function_index)
                        .get_function_object()
                        .expect("Closure operation should store index to function")
                        .clone();
                    let closure = Value::new_closure_object(function);
                    // Handle closure upvalues
                    let upvalues_count = closure
                        .get_closure_object()
                        .unwrap()
                        .borrow()
                        .function
                        .borrow()
                        .upvalues_count;
                    for _ in 0..upvalues_count {
                        let instruction = frame
                            .closure
                            .borrow_mut()
                            .function
                            .borrow_mut()
                            .chunk
                            .read_operation_code(frame.instruction_pointer)
                            .map_err(VirtualMachineError::InvalidInstructionFormat)?;
                        frame.instruction_pointer +=
                            OperationCode::get_instruction_bytes_length(&instruction);

                        match instruction {
                            OperationCode::LocalUpvalue(upvalue_index) => closure
                                .get_closure_object()
                                .unwrap()
                                .borrow_mut()
                                .upvalues
                                .push(self.capture_upvalue(frame.stack_start + upvalue_index)),
                            OperationCode::NonLocalUpvalue(upvalue_index) => closure
                                .get_closure_object()
                                .unwrap()
                                .borrow_mut()
                                .upvalues
                                .push(
                                    frame.closure.borrow().upvalues[upvalue_index as usize].clone(),
                                ),
                            _ => return Err(VirtualMachineError::NotEnoughUpvaluesInClosure),
                        }
                    }
                    self.stack_push(closure);
                }
                OperationCode::LocalUpvalue(_) => {
                    return Err(VirtualMachineError::HandlingUpvalueOutsideOfClosure)
                }
                OperationCode::NonLocalUpvalue(_) => {
                    return Err(VirtualMachineError::HandlingUpvalueOutsideOfClosure)
                }
                OperationCode::GetUpvalue(upvalue_index) => {
                    let closure = frame.closure.borrow();
                    let upvalue = &closure.upvalues[upvalue_index as usize];
                    let upvalue_borrow = upvalue.borrow();
                    let variable = upvalue_borrow.variable.clone();
                    let stack_index = upvalue_borrow.stack_index;

                    let value = match (stack_index, variable) {
                        (None, None) => {
                            return Err(VirtualMachineError::UpvalueIncorrectFieldAccess)
                        }
                        (None, Some(value)) => value.borrow().clone(),
                        (Some(stack_index), None) => self.stack[stack_index].clone(),
                        (Some(_), Some(_)) => {
                            return Err(VirtualMachineError::UpvalueIncorrectFieldAccess)
                        }
                    };

                    self.stack_push(value);
                }
                OperationCode::SetUpvalue(upvalue_index) => {
                    let closure = frame.closure.borrow_mut();
                    let upvalue = &closure.upvalues[upvalue_index as usize];
                    let upvalue_borrow = upvalue.borrow();
                    let variable = upvalue_borrow.variable.clone();
                    let stack_index = upvalue_borrow.stack_index;

                    match (stack_index, variable) {
                        (None, None) => {
                            return Err(VirtualMachineError::UpvalueIncorrectFieldAccess)
                        }
                        (None, Some(value)) => {
                            let mut inner_value = value.borrow_mut();
                            *inner_value = self.stack_peek(0)?.clone();
                        }
                        (Some(stack_index), None) => {
                            self.stack[stack_index] = self.stack_peek(0)?.clone()
                        }
                        (Some(_), Some(_)) => {
                            return Err(VirtualMachineError::UpvalueIncorrectFieldAccess)
                        }
                    };
                }
                OperationCode::CloseUpvalue => {
                    self.close_upvalue(self.stack.len() - 1);
                    self.stack_pop()?;
                }
                OperationCode::Class(class_name_index) => {
                    let name = frame
                        .closure
                        .borrow()
                        .function
                        .borrow()
                        .chunk
                        .read_constant(class_name_index);
                    let name_string_object = name
                        .get_string_object()
                        .map_err(|_| VirtualMachineError::InvalidVariableNameType)?;
                    let new_class_object =
                        Value::new_class_object(name_string_object.borrow().get_value());
                    self.stack_push(new_class_object);
                }
                OperationCode::GetProperty(property_name_index) => {
                    if !self.stack_peek(0)?.is_instance_object() {
                        self.runtime_error_message("Only instances can have properties.", &frame);
                        return Err(VirtualMachineError::PropertyOutsideInstance);
                    }
                    let instance = self.stack_peek(0)?.get_instance_object().unwrap().clone();

                    let property_name = frame
                        .closure
                        .borrow()
                        .function
                        .borrow()
                        .chunk
                        .read_constant(property_name_index);
                    let property_name_string = property_name
                        .get_string_object()
                        .expect("Field name should only be represented as string object");

                    let borrowed_instance = instance.borrow();
                    let get_field_result = borrowed_instance.fields.get(property_name_string);

                    // Field take precedence over methods, so we check for them first
                    match get_field_result {
                        Ok(property_value) => {
                            // Remove instance from stack
                            self.stack_pop()?;
                            self.stack_push(property_value.clone());
                        }
                        Err(_) => {
                            // We couldn't find field, so we check if such method exists
                            let bind_method_result = self.find_and_bind_method(
                                &borrowed_instance.class,
                                property_name_string,
                            );
                            if bind_method_result.is_err() {
                                self.runtime_error_message(
                                    &format!(
                                        "Undefined property {}.",
                                        property_name_string.borrow().get_value()
                                    ),
                                    &frame,
                                );
                                return Err(VirtualMachineError::UndefinedProperty);
                            }
                        }
                    }
                }
                OperationCode::SetProperty(property_name_index) => {
                    if !self.stack_peek(1)?.is_instance_object() {
                        self.runtime_error_message("Only instances can have properties.", &frame);
                        return Err(VirtualMachineError::PropertyOutsideInstance);
                    }
                    let instance = self.stack_peek(1)?.get_instance_object().unwrap().clone();

                    let field_name = frame
                        .closure
                        .borrow()
                        .function
                        .borrow()
                        .chunk
                        .read_constant(property_name_index);
                    let field_name_string = field_name
                        .get_string_object()
                        .expect("Field name should only be represented as string object");

                    instance
                        .borrow_mut()
                        .fields
                        .insert(field_name_string.clone(), self.stack_peek(0)?.clone());

                    // On the stack we have: [instance field_value]
                    // we want to have: [field_value];
                    let field_value = self.stack_pop()?;
                    self.stack_pop()?;
                    self.stack_push(field_value);
                }
                OperationCode::Method(method_name_index) => {
                    let method_name = frame
                        .closure
                        .borrow()
                        .function
                        .borrow()
                        .chunk
                        .read_constant(method_name_index);
                    let method_name_string = method_name
                        .get_string_object()
                        .expect("Method name should only be represented as string object");
                    self.define_method(method_name_string)?;
                }
                OperationCode::InvokeProperty(property_name_index, arguments_count) => {
                    let property_name = frame
                        .closure
                        .borrow()
                        .function
                        .borrow()
                        .chunk
                        .read_constant(property_name_index);
                    let property_name_string = property_name
                        .get_string_object()
                        .expect("Property name should only be represented as string object");
                    if let Err(e) =
                        self.invoke_property(property_name_string, arguments_count, &frame)
                    {
                        self.runtime_error_message("Only instances have properties.", &frame);
                        return Err(e);
                    }
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

        // Print current function
        let inner_most_closure = frame.closure.borrow();
        let inner_most_function = inner_most_closure.function.borrow();
        let inner_most_line = inner_most_function
            .chunk
            .read_line(frame.instruction_pointer - 1);
        let inner_most_name = inner_most_function.name.borrow();
        eprintln!(
            "[line {}] in {}",
            inner_most_line,
            inner_most_name.get_value()
        );

        // Print the call stack
        for frame in self.frames.iter().rev() {
            // -1 becuase the current instruction_pointer points to the next instruction to be executed
            let last_executed_instruction = frame.instruction_pointer - 1;
            let current_closure = frame.closure.borrow();
            let current_function = current_closure.function.borrow();
            let current_line = current_function.chunk.read_line(last_executed_instruction);
            let current_name = current_function.name.borrow();
            eprintln!("[line {}] in {}", current_line, current_name.get_value());
        }

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

    fn should_swap_frames(&self, callee: &Value) -> bool {
        match callee.get_type() {
            ValueType::NativeFunction => false,
            ValueType::ClassObject => {
                // We should only swap frames if we wanna call initializer
                callee
                    .get_class_object()
                    .unwrap()
                    .borrow()
                    .methods
                    .get(&self.init_string)
                    .is_ok()
            }
            _ => true,
        }
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
        frame: &CallFrame,
    ) -> Result<(), VirtualMachineError> {
        match callee.get_type() {
            ValueType::ClosureObject => {
                self.handle_function_call(
                    callee.get_closure_object().unwrap().clone(),
                    arguments_count,
                    Some(frame),
                )?;
                Ok(())
            }
            ValueType::NativeFunction => {
                self.handle_native_function_call(
                    callee.get_native_function().unwrap(),
                    arguments_count,
                );
                Ok(())
            }
            ValueType::ClassObject => {
                self.handle_class_initializer_call(
                    callee.get_class_object().unwrap(),
                    arguments_count,
                    Some(frame),
                )?;
                Ok(())
            }
            ValueType::BoundMethodObject => {
                // Put "this" at special slot zero of locals
                let top = self.stack.len();
                self.stack[top - arguments_count as usize - 1] = callee
                    .get_bound_method_object()
                    .unwrap()
                    .borrow()
                    .get_instance_as_value();

                let raw_closure = callee
                    .get_bound_method_object()
                    .unwrap()
                    .borrow()
                    .method
                    .clone();
                self.handle_function_call(raw_closure, arguments_count, Some(frame))?;
                Ok(())
            }
            _ => Err(VirtualMachineError::CallOnNotCallable),
        }
    }

    fn handle_function_call(
        &mut self,
        closure: Rc<RefCell<ClosureObject>>,
        arguments_count: u8,
        frame: Option<&CallFrame>,
    ) -> Result<(), VirtualMachineError> {
        if arguments_count != closure.borrow().function.borrow().arity as u8 {
            self.runtime_error_message(
                &format!(
                    "Expected {} arguments, but got {}",
                    closure.borrow().function.borrow().arity,
                    arguments_count
                ),
                frame.expect("This should be none only when call from interpret, when we don't pass any arguments and arity is 0, so this line should never be called if it's empty"),
            );
            return Err(VirtualMachineError::InvalidArgumentsCount);
        }
        let call_frame = CallFrame {
            closure,
            instruction_pointer: 0,
            // We do it so that for frame it seems stack start at functions's position,
            // as we have: <function <arg1> <arg2> ... <argN> <STACK_TOP>
            // so from stack top we must substract (n + 1)
            stack_start: self.stack.len() as u8 - (arguments_count + 1),
        };
        self.frames.push(call_frame);
        Ok(())
    }

    fn handle_native_function_call(
        &mut self,
        native_function: NativeFunction,
        arguments_count: u8,
    ) {
        let arguments_start = self.stack.len() - arguments_count as usize;
        let arguments_end = self.stack.len();
        let arguments = &self.stack[arguments_start..arguments_end];
        // Call native function
        let result = native_function(arguments);
        // Remove native function arguments + native function itself from the stack
        self.stack
            .truncate(self.stack.len() - (arguments_count + 1) as usize);
        // Put result back on the stack
        self.stack_push(result);
    }

    fn handle_class_initializer_call(
        &mut self,
        class: &Rc<RefCell<ClassObject>>,
        arguments_count: u8,
        frame: Option<&CallFrame>,
    ) -> Result<(), VirtualMachineError> {
        let new_instance = Value::new_instance_object(class);
        let top = self.stack.len();
        self.stack[top - arguments_count as usize - 1] = new_instance;
        match class.borrow().methods.get(&self.init_string) {
            Ok(initializer) => {
                // Initialize exits, we just call it as normal function
                self.handle_function_call(
                    initializer.get_closure_object().unwrap().clone(),
                    arguments_count,
                    frame,
                )
            }
            Err(_) => {
                // Initializer doesn't exists, if we find any arguments passed anyway we know it's an error
                if arguments_count > 0 {
                    self.runtime_error_message(
                        &format!("Expected 0 arguments, got {}", arguments_count),
                        frame.unwrap(),
                    );
                    return Err(VirtualMachineError::InvalidArgumentsCount);
                }
                Ok(())
            }
        }
    }

    fn define_method(
        &mut self,
        name: &Rc<RefCell<StringObject>>,
    ) -> Result<(), VirtualMachineError> {
        let method = self.stack_peek(0)?;
        let class = self
            .stack_peek(1)?
            .get_class_object()
            .map_err(|_| VirtualMachineError::HandlingMethodWithoutClass)?;
        class
            .borrow_mut()
            .add_method(name.clone(), method.clone())
            .map_err(|_| VirtualMachineError::InvalidVariableType)?;
        // Remove closure
        self.stack_pop()?;
        Ok(())
    }

    // It only makes sense to use this function before program starts executing
    fn define_native_function(&mut self, name: &str, native_function: NativeFunction) {
        // We are pushing and popping of the stack because of GC
        let function_name = Value::new_string_object(name, &mut self.strings);
        self.stack_push(function_name);
        let function = Value::new_native_function(native_function);
        self.stack_push(function);
        self.globals.insert(
            self.stack_peek(1)
                .unwrap()
                .get_string_object()
                .unwrap()
                .clone(),
            self.stack_peek(0).unwrap().clone(),
        );
        // Removing temporary values
        self.stack_pop().unwrap();
        self.stack_pop().unwrap();
    }

    fn capture_upvalue(&mut self, index: u8) -> Rc<RefCell<UpvalueObject>> {
        let new_upvalue = Rc::new(RefCell::new(UpvalueObject {
            stack_index: Some(index as usize),
            variable: None,
        }));
        let wrapper = UpvalueObjectBTreeWrapper(new_upvalue.clone());
        match self.open_upvalues.get(&wrapper) {
            Some(already_exisiting) => already_exisiting.0.clone(),
            None => {
                self.open_upvalues.insert(wrapper);
                new_upvalue
            }
        }
    }

    fn close_upvalue(&mut self, last_to_remove: usize) {
        let to_close: Vec<UpvalueObjectBTreeWrapper> = self
            .open_upvalues
            .iter()
            .filter(|u| {
                u.0.borrow()
                    .stack_index
                    .expect("Every open upvalue should have stack_index set")
                    >= last_to_remove
            })
            .cloned()
            .collect();

        for upvalue_wrapper in to_close {
            self.open_upvalues.remove(&upvalue_wrapper);
            let upvalue = upvalue_wrapper.0;
            let index = upvalue.borrow().stack_index.unwrap();
            upvalue.borrow_mut().variable = Some(Rc::new(RefCell::new(self.stack[index].clone())));
            upvalue.borrow_mut().stack_index = None;
        }
    }

    // If it find method with such name in the class it put it onto stack
    fn find_and_bind_method(
        &mut self,
        class: &Rc<RefCell<ClassObject>>,
        method_name: &Rc<RefCell<StringObject>>,
    ) -> Result<(), VirtualMachineError> {
        let borrowed_class = class.borrow();
        let method_lookup_result = borrowed_class.methods.get(method_name);
        match method_lookup_result {
            Ok(method) => {
                let instance = self.stack_peek(0)?.get_instance_object().unwrap();
                let bound_method = Value::new_bound_method_object(
                    instance.clone(),
                    method.get_closure_object().unwrap().clone(),
                );
                // Replace instance with the bounded method on the stack
                self.stack_pop()?;
                self.stack_push(bound_method);
                Ok(())
            }
            Err(_) => Err(VirtualMachineError::UndefinedProperty),
        }
    }

    fn invoke_property(
        &mut self,
        name: &Rc<RefCell<StringObject>>,
        arguments_count: u8,
        frame: &CallFrame,
    ) -> Result<(), VirtualMachineError> {
        let instance = self
            .stack_peek(arguments_count as usize)?
            .get_instance_object()
            .map_err(|_| VirtualMachineError::PropertyOutsideInstance)?
            .clone();

        let result = match instance.borrow().fields.get(name) {
            // Check if there is field with such name
            Ok(field) => {
                let top = self.stack.len();
                self.stack[top - arguments_count as usize - 1] = field.clone();
                self.handle_call_value(field.clone(), arguments_count, frame)
            }
            Err(_) => {
                let class = instance.borrow().class.clone();
                self.invoke_property_from_class(&class, name, arguments_count, frame)
            }
        };
        result
    }

    fn invoke_property_from_class(
        &mut self,
        class: &Rc<RefCell<ClassObject>>,
        name: &Rc<RefCell<StringObject>>,
        arguments_count: u8,
        frame: &CallFrame,
    ) -> Result<(), VirtualMachineError> {
        match class.borrow().methods.get(name) {
            Ok(method) => self.handle_function_call(
                method.get_closure_object().unwrap().clone(),
                arguments_count,
                Some(frame),
            ),
            Err(_) => {
                self.runtime_error_message(
                    &format!("Undefined property {}.", name.borrow().get_value()),
                    frame,
                );
                Err(VirtualMachineError::UndefinedProperty)
            }
        }
    }
}

impl Default for VirtualMachine {
    fn default() -> Self {
        Self::new()
    }
}
