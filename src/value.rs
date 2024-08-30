use core::fmt;
use std::{cell::RefCell, cmp::Ordering, mem::ManuallyDrop, rc::Rc};

use crate::{chunk::Chunk, table::Table};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ValueType {
    Bool,
    Nil,
    Number,
    StringObject,
    FunctionObject,
    NativeFunction,
    ClosureObject,
    ClassObject,
    InstanceObject,
    BoundMethodObject,
}

#[derive(Clone)]
pub struct StringObject {
    value: String,
    hash: u32,
}

impl StringObject {
    fn new(value: &str) -> Self {
        Self {
            value: value.into(),
            hash: Self::hash(value),
        }
    }

    fn transform_to_rc(self) -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(self))
    }

    fn new_rc(value: &str) -> Rc<RefCell<Self>> {
        Self::new(value).transform_to_rc()
    }

    pub fn get_value(&self) -> &str {
        &self.value
    }

    pub fn get_hash(&self) -> u32 {
        self.hash
    }

    // FNV-1a algorithm for calculating hash
    pub fn hash(value: &str) -> u32 {
        let mut hash_resut: u32 = 2166136261;
        for b in value.as_bytes() {
            hash_resut ^= *b as u32;
            hash_resut = hash_resut.wrapping_mul(16777619);
        }
        hash_resut
    }

    pub fn are_equal_rc(lhs: &Rc<RefCell<StringObject>>, rhs: &Rc<RefCell<StringObject>>) -> bool {
        Rc::ptr_eq(lhs, rhs)
    }
}

pub struct FunctionObject {
    pub arity: usize,
    pub upvalues_count: usize,
    pub chunk: Chunk,
    pub name: Rc<RefCell<StringObject>>,
}

impl FunctionObject {
    fn new(name: &str) -> Self {
        FunctionObject {
            arity: 0,
            upvalues_count: 0,
            chunk: Chunk::new(),
            name: StringObject::new_rc(name),
        }
    }

    fn transform_to_rc(self) -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(self))
    }

    pub fn new_rc(name: &str) -> Rc<RefCell<Self>> {
        Self::new(name).transform_to_rc()
    }

    pub fn are_equal_rc(
        lhs: &Rc<RefCell<FunctionObject>>,
        rhs: &Rc<RefCell<FunctionObject>>,
    ) -> bool {
        Rc::ptr_eq(lhs, rhs)
    }
}

impl From<Rc<RefCell<FunctionObject>>> for Value {
    fn from(value: Rc<RefCell<FunctionObject>>) -> Self {
        Value {
            value_type: ValueType::FunctionObject,
            actual_value: UnderlyingValue {
                function_object: ManuallyDrop::new(value),
            },
        }
    }
}

pub type NativeFunction = fn(&[Value]) -> Value;

pub struct ClosureObject {
    pub function: Rc<RefCell<FunctionObject>>,
    pub upvalues: Vec<Rc<RefCell<UpvalueObject>>>,
}

impl ClosureObject {
    pub fn new(function: Rc<RefCell<FunctionObject>>) -> Self {
        ClosureObject {
            function,
            upvalues: vec![],
        }
    }

    fn transform_to_rc(self) -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(self))
    }

    pub fn new_rc(function: Rc<RefCell<FunctionObject>>) -> Rc<RefCell<Self>> {
        Self::new(function).transform_to_rc()
    }

    pub fn are_equal_rc(
        lhs: &Rc<RefCell<ClosureObject>>,
        rhs: &Rc<RefCell<ClosureObject>>,
    ) -> bool {
        Rc::ptr_eq(lhs, rhs)
    }
}

impl From<Rc<RefCell<ClosureObject>>> for Value {
    fn from(value: Rc<RefCell<ClosureObject>>) -> Self {
        Value {
            value_type: ValueType::ClosureObject,
            actual_value: UnderlyingValue {
                closure_object: ManuallyDrop::new(value),
            },
        }
    }
}

// GENERAL IDEA:
// Firsly we store aboslute stack index of the variable
// When we have instruction to close upvalue, we get value from the stack, create rc<refcell<>> of it
// and put it inside here.
#[derive(Clone)]
pub struct UpvalueObject {
    /// None when variable that upvalue references was popped from the stack
    pub stack_index: Option<usize>,
    /// None when variable is still on the stack
    pub variable: Option<Rc<RefCell<Value>>>,
}

impl PartialEq for UpvalueObject {
    fn eq(&self, other: &Self) -> bool {
        if self.stack_index != other.stack_index {
            return false;
        }
        match (&self.variable, &other.variable) {
            (None, None) => true,
            (None, Some(_)) => false,
            (Some(_), None) => false,
            (Some(self_ptr), Some(other_ptr)) => Rc::ptr_eq(self_ptr, other_ptr),
        }
    }
}

impl Eq for UpvalueObject {}

impl Ord for UpvalueObject {
    fn cmp(&self, other: &Self) -> Ordering {
        // Compare stack_index in reverse (descending order)
        other.stack_index.cmp(&self.stack_index)
    }
}

impl PartialOrd for UpvalueObject {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct UpvalueObjectBTreeWrapper(pub Rc<RefCell<UpvalueObject>>);

impl From<UpvalueObjectBTreeWrapper> for Rc<RefCell<UpvalueObject>> {
    fn from(value: UpvalueObjectBTreeWrapper) -> Self {
        value.0
    }
}

pub struct ClassObject {
    name: Rc<RefCell<StringObject>>,
    // We must ensure that the only elements that are inserted here are closures
    pub methods: Table,
}

pub struct InvalidMethodType {}

impl ClassObject {
    fn new(name: &str) -> Self {
        ClassObject {
            name: StringObject::new_rc(name),
            methods: Table::new(),
        }
    }

    fn transform_to_rc(self) -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(self))
    }

    pub fn new_rc(name: &str) -> Rc<RefCell<Self>> {
        Self::new(name).transform_to_rc()
    }

    pub fn are_equal_rc(lhs: &Rc<RefCell<ClassObject>>, rhs: &Rc<RefCell<ClassObject>>) -> bool {
        Rc::ptr_eq(lhs, rhs)
    }

    pub fn add_method(
        &mut self,
        method_name: Rc<RefCell<StringObject>>,
        method: Value,
    ) -> Result<(), InvalidMethodType> {
        if !method.is_closure_object() {
            return Err(InvalidMethodType {});
        }
        self.methods.insert(method_name, method);
        Ok(())
    }
}

pub struct InstanceObject {
    pub fields: Table,
    pub class: Rc<RefCell<ClassObject>>,
}

impl InstanceObject {
    fn new(class: &Rc<RefCell<ClassObject>>) -> Self {
        InstanceObject {
            class: class.clone(),
            fields: Table::new(),
        }
    }

    fn transform_to_rc(self) -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(self))
    }

    pub fn new_rc(class: &Rc<RefCell<ClassObject>>) -> Rc<RefCell<Self>> {
        Self::new(class).transform_to_rc()
    }

    pub fn are_equal_rc(
        lhs: &Rc<RefCell<InstanceObject>>,
        rhs: &Rc<RefCell<InstanceObject>>,
    ) -> bool {
        Rc::ptr_eq(lhs, rhs)
    }
}

pub struct BoundMethodObject {
    instance: Rc<RefCell<InstanceObject>>,
    pub method: Rc<RefCell<ClosureObject>>,
}

impl BoundMethodObject {
    fn new(instance: Rc<RefCell<InstanceObject>>, method: Rc<RefCell<ClosureObject>>) -> Self {
        BoundMethodObject { instance, method }
    }

    fn transform_to_rc(self) -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(self))
    }

    pub fn new_rc(
        instance: Rc<RefCell<InstanceObject>>,
        method: Rc<RefCell<ClosureObject>>,
    ) -> Rc<RefCell<Self>> {
        Self::new(instance, method).transform_to_rc()
    }

    pub fn are_equal_rc(
        lhs: &Rc<RefCell<BoundMethodObject>>,
        rhs: &Rc<RefCell<BoundMethodObject>>,
    ) -> bool {
        Rc::ptr_eq(lhs, rhs)
    }

    pub fn get_instance_as_value(&self) -> Value {
        Value::from(self.instance.clone())
    }
}

impl From<Rc<RefCell<InstanceObject>>> for Value {
    fn from(value: Rc<RefCell<InstanceObject>>) -> Self {
        Value {
            value_type: ValueType::InstanceObject,
            actual_value: UnderlyingValue {
                instance_object: ManuallyDrop::new(value),
            },
        }
    }
}

#[repr(C)]
union UnderlyingValue {
    boolean: bool,
    number: f64,
    string_object: ManuallyDrop<Rc<RefCell<StringObject>>>,
    function_object: ManuallyDrop<Rc<RefCell<FunctionObject>>>,
    native_function: NativeFunction,
    closure_object: ManuallyDrop<Rc<RefCell<ClosureObject>>>,
    class_object: ManuallyDrop<Rc<RefCell<ClassObject>>>,
    instance_object: ManuallyDrop<Rc<RefCell<InstanceObject>>>,
    bound_method_object: ManuallyDrop<Rc<RefCell<BoundMethodObject>>>,
}

pub struct Value {
    value_type: ValueType,
    actual_value: UnderlyingValue,
}

#[derive(Debug)]
pub struct ValueInterpretingError {}

impl Value {
    pub fn new_bool(value: bool) -> Value {
        Value {
            value_type: ValueType::Bool,
            actual_value: UnderlyingValue { boolean: value },
        }
    }

    pub fn is_bool(&self) -> bool {
        self.value_type == ValueType::Bool
    }

    pub fn get_bool(&self) -> Result<bool, ValueInterpretingError> {
        match self.value_type {
            ValueType::Bool => unsafe { Ok(self.actual_value.boolean) },
            _ => Err(ValueInterpretingError {}),
        }
    }

    pub fn new_number(value: f64) -> Value {
        Value {
            value_type: ValueType::Number,
            actual_value: UnderlyingValue { number: value },
        }
    }

    pub fn is_number(&self) -> bool {
        self.value_type == ValueType::Number
    }

    pub fn get_number(&self) -> Result<f64, ValueInterpretingError> {
        match self.value_type {
            ValueType::Number => unsafe { Ok(self.actual_value.number) },
            _ => Err(ValueInterpretingError {}),
        }
    }

    pub fn new_nil() -> Value {
        Value {
            value_type: ValueType::Nil,
            actual_value: UnderlyingValue { number: 0.0 },
        }
    }

    pub fn is_nil(&self) -> bool {
        self.value_type == ValueType::Nil
    }

    pub fn new_string_object(value: &str, intern_strings: &mut Table) -> Value {
        if let Some(already_existing) = intern_strings.find_string(value) {
            return Value {
                actual_value: UnderlyingValue {
                    string_object: ManuallyDrop::new(already_existing.clone()),
                },
                value_type: ValueType::StringObject,
            };
        }
        let key = StringObject::new_rc(value);
        let value_key = key.clone();
        intern_strings.insert(key, Value::new_nil());
        Value {
            value_type: ValueType::StringObject,
            actual_value: UnderlyingValue {
                string_object: ManuallyDrop::new(value_key),
            },
        }
    }

    pub fn is_string_object(&self) -> bool {
        self.value_type == ValueType::StringObject
    }

    pub fn get_string_object(&self) -> Result<&Rc<RefCell<StringObject>>, ValueInterpretingError> {
        match self.value_type {
            ValueType::StringObject => unsafe { Ok(&self.actual_value.string_object) },
            _ => Err(ValueInterpretingError {}),
        }
    }

    pub fn new_function_object(name: &str) -> Value {
        Value {
            value_type: ValueType::FunctionObject,
            actual_value: UnderlyingValue {
                function_object: ManuallyDrop::new(FunctionObject::new_rc(name)),
            },
        }
    }

    pub fn is_function_object(&self) -> bool {
        self.value_type == ValueType::FunctionObject
    }

    pub fn get_function_object(
        &self,
    ) -> Result<&Rc<RefCell<FunctionObject>>, ValueInterpretingError> {
        match self.value_type {
            ValueType::FunctionObject => unsafe { Ok(&self.actual_value.function_object) },
            _ => Err(ValueInterpretingError {}),
        }
    }

    pub fn new_native_function(function: NativeFunction) -> Value {
        Value {
            value_type: ValueType::NativeFunction,
            actual_value: UnderlyingValue {
                native_function: function,
            },
        }
    }

    pub fn is_native_function(&self) -> bool {
        self.value_type == ValueType::NativeFunction
    }

    pub fn get_native_function(&self) -> Result<NativeFunction, ValueInterpretingError> {
        match self.value_type {
            ValueType::NativeFunction => unsafe { Ok(self.actual_value.native_function) },
            _ => Err(ValueInterpretingError {}),
        }
    }

    pub fn new_closure_object(function: Rc<RefCell<FunctionObject>>) -> Value {
        Value {
            value_type: ValueType::ClosureObject,
            actual_value: UnderlyingValue {
                closure_object: ManuallyDrop::new(ClosureObject::new_rc(function)),
            },
        }
    }

    pub fn is_closure_object(&self) -> bool {
        self.value_type == ValueType::ClosureObject
    }

    pub fn get_closure_object(
        &self,
    ) -> Result<&Rc<RefCell<ClosureObject>>, ValueInterpretingError> {
        match self.value_type {
            ValueType::ClosureObject => unsafe { Ok(&self.actual_value.closure_object) },
            _ => Err(ValueInterpretingError {}),
        }
    }

    pub fn new_class_object(name: &str) -> Value {
        Value {
            value_type: ValueType::ClassObject,
            actual_value: UnderlyingValue {
                class_object: ManuallyDrop::new(ClassObject::new_rc(name)),
            },
        }
    }

    pub fn is_class_object(&self) -> bool {
        self.value_type == ValueType::ClassObject
    }

    pub fn get_class_object(&self) -> Result<&Rc<RefCell<ClassObject>>, ValueInterpretingError> {
        match self.value_type {
            ValueType::ClassObject => unsafe { Ok(&self.actual_value.class_object) },
            _ => Err(ValueInterpretingError {}),
        }
    }

    pub fn new_instance_object(class: &Rc<RefCell<ClassObject>>) -> Value {
        Value {
            value_type: ValueType::InstanceObject,
            actual_value: UnderlyingValue {
                instance_object: ManuallyDrop::new(InstanceObject::new_rc(class)),
            },
        }
    }

    pub fn is_instance_object(&self) -> bool {
        self.value_type == ValueType::InstanceObject
    }

    pub fn get_instance_object(
        &self,
    ) -> Result<&Rc<RefCell<InstanceObject>>, ValueInterpretingError> {
        match self.value_type {
            ValueType::InstanceObject => unsafe { Ok(&self.actual_value.instance_object) },
            _ => Err(ValueInterpretingError {}),
        }
    }

    pub fn new_bound_method_object(
        instance: Rc<RefCell<InstanceObject>>,
        method: Rc<RefCell<ClosureObject>>,
    ) -> Value {
        Value {
            value_type: ValueType::BoundMethodObject,
            actual_value: UnderlyingValue {
                bound_method_object: ManuallyDrop::new(BoundMethodObject::new_rc(instance, method)),
            },
        }
    }

    pub fn is_bound_method_object(&self) -> bool {
        self.value_type == ValueType::BoundMethodObject
    }

    pub fn get_bound_method_object(
        &self,
    ) -> Result<&Rc<RefCell<BoundMethodObject>>, ValueInterpretingError> {
        match self.value_type {
            ValueType::BoundMethodObject => unsafe { Ok(&self.actual_value.bound_method_object) },
            _ => Err(ValueInterpretingError {}),
        }
    }

    pub fn get_type(&self) -> ValueType {
        self.value_type
    }

    pub fn is_falsey(&self) -> bool {
        match self.value_type {
            ValueType::Bool => !self.get_bool().expect("Bool type should contain bool"),
            ValueType::Nil => true,
            _ => false,
        }
    }

    pub fn are_values_equal(lhs: &Value, rhs: &Value) -> bool {
        if lhs.value_type != rhs.value_type {
            return false;
        }
        match lhs.value_type {
            ValueType::Bool => {
                lhs.get_bool().expect("Bool type should contain bool")
                    == rhs.get_bool().expect("Bool type should contain bool")
            }
            ValueType::Nil => true,
            ValueType::Number => {
                lhs.get_number().expect("Number type should contain number")
                    == rhs.get_number().expect("Number type should contain number")
            }
            ValueType::StringObject => StringObject::are_equal_rc(
                lhs.get_string_object()
                    .expect("StringObject type should contain String Object"),
                rhs.get_string_object()
                    .expect("StringObject type should contain String Object"),
            ),
            ValueType::FunctionObject => FunctionObject::are_equal_rc(
                lhs.get_function_object()
                    .expect("FunctionObject type should contain function object"),
                rhs.get_function_object()
                    .expect("FunctionObject type should contain function object"),
            ),
            ValueType::NativeFunction => {
                // We are comparing function pointers here
                lhs.get_native_function()
                    .expect("NativeFunction type should contain native function")
                    == rhs
                        .get_native_function()
                        .expect("NativeFunction type should contain native function")
            }
            ValueType::ClosureObject => ClosureObject::are_equal_rc(
                lhs.get_closure_object()
                    .expect("ClosureObject type should contain closure object"),
                rhs.get_closure_object()
                    .expect("ClosureObject type should contain closure object"),
            ),
            ValueType::ClassObject => ClassObject::are_equal_rc(
                lhs.get_class_object()
                    .expect("ClassObject type should containt class object"),
                rhs.get_class_object()
                    .expect("ClassObject type should containt class object"),
            ),
            ValueType::InstanceObject => InstanceObject::are_equal_rc(
                lhs.get_instance_object()
                    .expect("InstanceObject type should contain instance object"),
                rhs.get_instance_object()
                    .expect("InstanceObject type should contain instance object"),
            ),
            ValueType::BoundMethodObject => BoundMethodObject::are_equal_rc(
                lhs.get_bound_method_object()
                    .expect("BoundMethodObject type should contain bound method object."),
                rhs.get_bound_method_object()
                    .expect("BoundMethodObject type should contain bound method object."),
            ),
        }
    }
}

impl Clone for Value {
    fn clone(&self) -> Self {
        let actual_value_clone = match self.value_type {
            ValueType::Bool => UnderlyingValue {
                boolean: self.get_bool().expect("Bool type should contain bool"),
            },
            ValueType::Nil => UnderlyingValue { number: 0.0 },
            ValueType::Number => UnderlyingValue {
                number: self
                    .get_number()
                    .expect("Number type type should contain number"),
            },
            ValueType::StringObject => UnderlyingValue {
                string_object: ManuallyDrop::new(
                    self.get_string_object()
                        .expect("StringObject type should contain String Object")
                        .clone(),
                ),
            },
            ValueType::FunctionObject => UnderlyingValue {
                function_object: ManuallyDrop::new(
                    self.get_function_object()
                        .expect("FunctionObject type should containt Function Object")
                        .clone(),
                ),
            },
            ValueType::NativeFunction => UnderlyingValue {
                native_function: self
                    .get_native_function()
                    .expect("NativeFunction type should contain native function"),
            },
            ValueType::ClosureObject => UnderlyingValue {
                closure_object: ManuallyDrop::new(
                    self.get_closure_object()
                        .expect("ClosureObject type should contain closure object")
                        .clone(),
                ),
            },
            ValueType::ClassObject => UnderlyingValue {
                class_object: ManuallyDrop::new(
                    self.get_class_object()
                        .expect("ClassObject type should contain class object")
                        .clone(),
                ),
            },
            ValueType::InstanceObject => UnderlyingValue {
                instance_object: ManuallyDrop::new(
                    self.get_instance_object()
                        .expect("InstanceObject type should contain instance object")
                        .clone(),
                ),
            },
            ValueType::BoundMethodObject => UnderlyingValue {
                bound_method_object: ManuallyDrop::new(
                    self.get_bound_method_object()
                        .expect("BoundMethodObject type should contain bound method object.")
                        .clone(),
                ),
            },
        };
        Self {
            value_type: self.value_type,
            actual_value: actual_value_clone,
        }
    }
}

impl Drop for Value {
    fn drop(&mut self) {
        if self.is_string_object() {
            unsafe { ManuallyDrop::drop(&mut self.actual_value.string_object) }
        } else if self.is_function_object() {
            unsafe { ManuallyDrop::drop(&mut self.actual_value.function_object) }
        } else if self.is_closure_object() {
            unsafe { ManuallyDrop::drop(&mut self.actual_value.closure_object) }
        } else if self.is_class_object() {
            unsafe { ManuallyDrop::drop(&mut self.actual_value.class_object) }
        } else if self.is_instance_object() {
            unsafe { ManuallyDrop::drop(&mut self.actual_value.instance_object) }
        } else if self.is_bound_method_object() {
            unsafe { ManuallyDrop::drop(&mut self.actual_value.bound_method_object) }
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.value_type {
            ValueType::Bool => write!(
                f,
                "{}",
                self.get_bool().expect("Bool type should contain bool")
            ),
            ValueType::Nil => write!(f, "NIL"),
            ValueType::Number => write!(
                f,
                "{}",
                self.get_number()
                    .expect("Number type should contain number")
            ),
            ValueType::StringObject => write!(
                f,
                "{}",
                self.get_string_object()
                    .expect("StringObject type should contain String Object")
                    .borrow()
                    .get_value()
            ),
            ValueType::FunctionObject => write!(
                f,
                "<fn {}>",
                self.get_function_object()
                    .expect("FunctionObject type should contain Function Object")
                    .borrow()
                    .name
                    .borrow()
                    .get_value()
            ),
            ValueType::NativeFunction => write!(f, "<native function>"),
            ValueType::ClosureObject => write!(
                f,
                "<fn {}>",
                self.get_closure_object()
                    .expect("ClosureObject type should contain closure object")
                    .borrow()
                    .function
                    .borrow()
                    .name
                    .borrow()
                    .get_value()
            ),
            ValueType::ClassObject => write!(
                f,
                "<class {}>",
                self.get_class_object()
                    .expect("ClassObject type should contain class object")
                    .borrow()
                    .name
                    .borrow()
                    .get_value()
            ),
            ValueType::InstanceObject => write!(
                f,
                "<instance {}>",
                self.get_instance_object()
                    .expect("InstanceObject type should contain instance object")
                    .borrow()
                    .class
                    .borrow()
                    .name
                    .borrow()
                    .get_value()
            ),
            ValueType::BoundMethodObject => write!(
                f,
                "<fn {}>",
                self.get_bound_method_object()
                    .expect("BoundMethodObject type should contain bound method object.")
                    .borrow()
                    .method
                    .borrow()
                    .function
                    .borrow()
                    .name
                    .borrow()
                    .get_value()
            ),
        }
    }
}

pub struct ValueContainer {
    values: Vec<Value>,
}

impl ValueContainer {
    const INITIAL_INSTRUCTIONS_SIZE: usize = 8;

    pub fn new() -> Self {
        let initial_values = Vec::with_capacity(Self::INITIAL_INSTRUCTIONS_SIZE);
        ValueContainer {
            values: initial_values,
        }
    }

    pub fn add_value(&mut self, value: Value) {
        self.values.push(value);
    }

    pub fn get_values_length(&self) -> usize {
        self.values.len()
    }

    pub fn get_value(&self, offset: usize) -> Value {
        self.values[offset].clone()
    }
}

impl Default for ValueContainer {
    fn default() -> Self {
        Self::new()
    }
}
