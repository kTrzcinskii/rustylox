use rustylox::{
    chunk::{Chunk, OperationCode},
    debugger::Debugger,
};

fn main() {
    let mut chunk = Chunk::new();
    let constant_index = chunk.add_constant(1.2);
    chunk.add_instruction(OperationCode::Constant(constant_index), 123);
    chunk.add_instruction(OperationCode::Return, 123);
    Debugger::disassemble_chunk(&chunk, "test chunk").unwrap();
}
