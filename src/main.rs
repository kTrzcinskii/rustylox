use rustylox::{
    chunk::{Chunk, OperationCode},
    vm::VirtualMachine,
};

#[cfg(feature = "debug_trace_execution")]
use rustylox::debugger::Debugger;

fn main() {
    let mut vm = VirtualMachine::new();
    let mut chunk = Chunk::new();
    let constant_index = chunk.add_constant(1.2);
    chunk.add_instruction(OperationCode::Constant(constant_index), 123);
    chunk.add_instruction(OperationCode::Return, 123);
    #[cfg(feature = "debug_trace_execution")]
    Debugger::disassemble_chunk(&chunk, "test chunk").unwrap();
    vm.interpret(&chunk);
}
