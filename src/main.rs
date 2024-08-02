use std::{
    env, fs,
    io::{self, BufRead, Write},
    process::exit,
};

use rustylox::{error, vm};

fn usage() {
    panic!("Usage: ./rustylox [path]")
}

fn start_repl() {
    let mut vm = vm::VirtualMachine::new();

    let stdin = io::stdin();
    loop {
        print!("> ");
        io::stdout().flush().unwrap();
        // TODO: gracefully return from here instead of just calling unwraps
        let line = stdin.lock().lines().next().unwrap().unwrap();
        vm.interpret(&line);
    }
}

fn run_file(file_name: &str) {
    let mut vm = vm::VirtualMachine::new();
    // TODO: handle this error better
    let file_content = fs::read_to_string(file_name).expect("File should exists.");
    let result = vm.interpret(&file_content);

    match result {
        vm::InterpretResult::Ok => {}
        vm::InterpretResult::CompileError => exit(error::COMPILE_ERROR_CODE),
        vm::InterpretResult::RuntimeError => exit(error::RUNTIME_ERROR_CODE),
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() > 2 {
        usage();
    }

    if args.len() == 2 {
        run_file(&args[1]);
    } else {
        start_repl();
    }
}
