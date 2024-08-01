use rustylox::{
    chunk::{Chunk, OperationCode},
    vm::VirtualMachine,
};

fn main() {
    let mut vm = VirtualMachine::new();
    let mut chunk = Chunk::new();

    // simple example by hand - evalue value of:
    // -((1.2 + 3.4) / 5.6)

    let mut constant_index = chunk.add_constant(1.2);
    chunk.add_instruction(OperationCode::Constant(constant_index), 123);

    constant_index = chunk.add_constant(3.4);
    chunk.add_instruction(OperationCode::Constant(constant_index), 123);

    chunk.add_instruction(OperationCode::Add, 123);

    constant_index = chunk.add_constant(5.6);
    chunk.add_instruction(OperationCode::Constant(constant_index), 123);

    chunk.add_instruction(OperationCode::Divide, 123);

    chunk.add_instruction(OperationCode::Negate, 123);

    chunk.add_instruction(OperationCode::Return, 123);

    vm.interpret(&chunk);
}
