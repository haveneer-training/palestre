#![allow(dead_code, clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::num::NonZeroU16;

use pbc::{Instr, Program};
use vm::{Machine, Memory, Report};

pub fn program(constants: &[i64], instrs: &[Instr]) -> Program {
    let mut code = Vec::new();
    for instr in instrs {
        pbc::encode(*instr, &mut code);
    }
    Program::new(constants.to_vec(), code).unwrap()
}

pub fn budget(gas: u16) -> NonZeroU16 {
    NonZeroU16::new(gas).unwrap()
}

/// Exécute un programme qui ne pose aucun rappel.
pub fn run(p: &Program, mem: Memory, gas: u16) -> Report {
    Machine::new(p, mem, budget(gas))
        .run()
        .done()
        .expect("aucun rappel attendu")
}

/// Mémoire nulle, budget usuel de 1000, aucune constante.
pub fn run_default(instrs: &[Instr]) -> Report {
    run(&program(&[], instrs), [0; 256], 1000)
}
