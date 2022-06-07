use std::process::exit;

use cheshire::arena::GcArena;
use cheshire::vm::VirtualMachine;
use gc_arena::ArenaParameters;

pub fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() == 1 {
        repl();
    } else if args.len() == 2 {
        run_file(args[1].clone());
    } else {
        eprintln!("Usage: {} [path]", args[0]);
        exit(64);
    }
}

fn repl() {
    let mut arena = GcArena::new(ArenaParameters::default(), |mc| VirtualMachine::repl(mc));
    loop {
        arena.mutate(|mc, vm| {
            let result = vm.interpret(mc);
            match result {
                Ok(_) => {}
                // Err(err) => eprintln!("{}", err),
                Err(err) => {
                    eprintln!("{}", err);
                    vm.reset_repl(mc);
                }
            }
        });

        arena.collect_debt();
    }
}

fn run_file(path: String) {
    let mut arena = GcArena::new(ArenaParameters::default(), |mc| {
        VirtualMachine::load_file(path, mc)
    });
    loop {
        arena.mutate(|mc, vm| {
            let result = vm.interpret(mc);
            match result {
                Ok(_) => {}
                Err(err) => {
                    eprintln!("{}", err);
                    std::process::exit(1);
                }
            }
        });

        arena.collect_debt();
    }
}
