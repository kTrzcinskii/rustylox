use rustylox::{
    chunk::{Chunk, OperationCode},
    debugger::Debugger,
};

fn main() {
    let mut chunk = Chunk::new();
    chunk.add_instruction(OperationCode::Return);
    chunk.add_instruction(OperationCode::Return);
    chunk.add_instruction(OperationCode::Return);
    chunk.add_instruction(OperationCode::Return);
    Debugger::disassemble_chunk(&chunk, "test chunk").unwrap();
}
