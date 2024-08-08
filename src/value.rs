use core::fmt;

#[derive(Clone, Copy, PartialEq)]
enum ValueType {
    Bool,
    Nil,
    Number,
}

#[derive(Clone, Copy)]
#[repr(C)]
union UnderlyingValue {
    boolean: bool,
    number: f64,
}

#[derive(Clone, Copy)]
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

    pub fn is_bool(value: &Value) -> bool {
        value.value_type == ValueType::Bool
    }

    pub fn get_bool(value: &Value) -> Result<bool, ValueInterpretingError> {
        match value.value_type {
            ValueType::Bool => unsafe { Ok(value.actual_value.boolean) },
            _ => Err(ValueInterpretingError {}),
        }
    }

    pub fn new_number(value: f64) -> Value {
        Value {
            value_type: ValueType::Number,
            actual_value: UnderlyingValue { number: value },
        }
    }

    pub fn is_number(value: &Value) -> bool {
        value.value_type == ValueType::Number
    }

    pub fn get_number(value: &Value) -> Result<f64, ValueInterpretingError> {
        match value.value_type {
            ValueType::Number => unsafe { Ok(value.actual_value.number) },
            _ => Err(ValueInterpretingError {}),
        }
    }

    pub fn new_nil() -> Value {
        Value {
            value_type: ValueType::Nil,
            actual_value: UnderlyingValue { number: 0.0 },
        }
    }

    pub fn is_nil(value: &Value) -> bool {
        value.value_type == ValueType::Nil
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.value_type {
            ValueType::Bool => write!(f, "{}", Self::get_bool(self).unwrap()),
            ValueType::Nil => write!(f, "NIL"),
            ValueType::Number => write!(f, "{}", Self::get_number(self).unwrap()),
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
        self.values[offset]
    }
}

impl Default for ValueContainer {
    fn default() -> Self {
        Self::new()
    }
}
