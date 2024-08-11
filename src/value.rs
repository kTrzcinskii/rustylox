use core::fmt;
use std::{cell::RefCell, mem::ManuallyDrop, rc::Rc};

#[derive(Clone, Copy, PartialEq)]
pub enum ValueType {
    Bool,
    Nil,
    Number,
    HeapObject,
}

#[derive(Clone)]
pub struct StringObject {
    value: String,
    hash: u32,
}

impl StringObject {
    pub fn new(value: &str) -> Self {
        Self {
            value: value.into(),
            hash: Self::hash(value),
        }
    }

    pub fn get_value(&self) -> &str {
        &self.value
    }

    pub fn get_hash(&self) -> u32 {
        self.hash
    }

    // FNV-1a algorithm for calculating hash
    fn hash(value: &str) -> u32 {
        let mut hash_resut: u32 = 2166136261;
        for b in value.as_bytes() {
            hash_resut ^= *b as u32;
            hash_resut *= 16777619;
        }
        hash_resut
    }
}

#[derive(Clone)]
pub enum HeapObject {
    String(StringObject),
}

impl HeapObject {
    pub fn are_objects_equal(lhs: &HeapObject, rhs: &HeapObject) -> bool {
        match (lhs, rhs) {
            (HeapObject::String(lsh_string_object), HeapObject::String(rhs_string_object)) => {
                lsh_string_object.value == rhs_string_object.value
            }
        }
    }
}

impl fmt::Display for HeapObject {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HeapObject::String(string_object) => write!(f, "{}", string_object.value),
        }
    }
}

#[repr(C)]
union UnderlyingValue {
    boolean: bool,
    number: f64,
    object: ManuallyDrop<Rc<RefCell<HeapObject>>>,
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

    pub fn new_heap_object(value: HeapObject) -> Value {
        Value {
            value_type: ValueType::HeapObject,
            actual_value: UnderlyingValue {
                object: ManuallyDrop::new(Rc::new(RefCell::new(value))),
            },
        }
    }

    pub fn is_heap_object(&self) -> bool {
        self.value_type == ValueType::HeapObject
    }

    pub fn get_heap_object(&self) -> Result<&Rc<RefCell<HeapObject>>, ValueInterpretingError> {
        match self.value_type {
            ValueType::HeapObject => unsafe { Ok(&self.actual_value.object) },
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
            ValueType::Number => false,
            ValueType::HeapObject => true,
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
            ValueType::HeapObject => {
                let lhs_unwrap = lhs
                    .get_heap_object()
                    .expect("HeapObject type should contain heap object");
                let rhs_unwrap = rhs
                    .get_heap_object()
                    .expect("HeapObject type should contain heap object");
                HeapObject::are_objects_equal(&lhs_unwrap.borrow(), &rhs_unwrap.borrow())
            }
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
            ValueType::HeapObject => UnderlyingValue {
                object: ManuallyDrop::new(
                    self.get_heap_object()
                        .expect("HeapObject type should contain heap object")
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
        if self.is_heap_object() {
            unsafe {
                ManuallyDrop::drop(&mut self.actual_value.object);
            }
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
            ValueType::HeapObject => write!(
                f,
                "{}",
                self.get_heap_object()
                    .expect("HeapObject type should contain heap object")
                    .borrow()
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
