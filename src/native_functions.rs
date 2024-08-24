use std::time::{SystemTime, UNIX_EPOCH};

use crate::value::Value;

pub fn clock_native(_: &[Value]) -> Value {
    let time: f64 = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time shouldn't go backwards")
        .as_millis() as f64;
    Value::new_number(time)
}
