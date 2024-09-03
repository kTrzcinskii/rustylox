use std::{
    env, fs,
    io::{self, BufRead, Write},
    process::exit,
};

use rustylox::{error, vm};

fn usage() {
    eprintln!("Usage: ./rustylox [path]");
    exit(error::INVALID_USAGE_ERROR_CODE);
}

fn start_repl() {
    let mut vm = vm::VirtualMachine::new();

    let stdin = io::stdin();
    loop {
        print!("> ");
        io::stdout().flush().unwrap();
        let line = stdin
            .lock()
            .lines()
            .next()
            .and_then(|res| res.ok())
            .unwrap_or_default();
        if line.is_empty() {
            break;
        }
        vm.interpret(&line);
    }
}

fn run_file(file_name: &str) {
    let mut vm = vm::VirtualMachine::new();
    match fs::read_to_string(file_name) {
        Ok(file_content) => {
            let result = vm.interpret(&file_content);

            match result {
                vm::InterpretResult::Ok => {}
                vm::InterpretResult::CompileError => exit(error::COMPILE_ERROR_CODE),
                vm::InterpretResult::RuntimeError => exit(error::RUNTIME_ERROR_CODE),
            }
        }
        Err(_) => {
            eprintln!("Couldn't read from {}", file_name);
            exit(error::FILE_NOT_FOUND_ERROR_CODE);
        }
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
