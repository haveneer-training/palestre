#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use common::{program, run, run_default};
use pbc::{Fault, Instr as I, Program};
use vm::STACK_SIZE;

fn full_stack() -> Vec<I> {
    vec![I::PushI16(0); STACK_SIZE]
}

#[test]
fn halt_ends_the_turn_for_free() {
    let r = run_default(&[I::Halt]);
    assert_eq!(r.fault, None);
    assert_eq!(r.gas_used, 0);
    assert!(r.trace.is_empty());
    assert_eq!(r.act, None);
    assert_eq!(r.mem, [0; 256]);
}

#[test]
fn trace_keeps_values_in_order() {
    let r = run_default(&[I::PushI16(1), I::Trace, I::PushI16(-2), I::Trace, I::Halt]);
    assert_eq!(r.trace, vec![1, -2]);
    assert_eq!(r.gas_used, 4);
}

#[test]
fn const_reads_the_table() {
    let p = program(
        &[i64::MIN, 42],
        &[I::Const(1), I::Trace, I::Const(0), I::Trace, I::Halt],
    );
    let r = run(&p, [0; 256], 1000);
    assert_eq!(r.trace, vec![42, i64::MIN]);
    assert_eq!(r.gas_used, 4);
}

#[test]
fn const_outside_the_table_is_a_paid_badconst() {
    let r = run_default(&[I::Const(0), I::Halt]);
    assert_eq!(r.fault, Some(Fault::BadConst));
    assert_eq!(r.gas_used, 1);
}

#[test]
fn constx_pops_its_index() {
    let p = program(
        &[10, 20, 30],
        &[I::PushI16(2), I::ConstX, I::Trace, I::Halt],
    );
    let r = run(&p, [0; 256], 1000);
    assert_eq!(r.trace, vec![30]);
    assert_eq!(r.gas_used, 4);
}

#[test]
fn constx_rejects_bad_indices() {
    let p = program(&[5], &[I::PushI16(-1), I::ConstX]);
    assert_eq!(run(&p, [0; 256], 1000).fault, Some(Fault::BadConst));
    let p = program(&[5], &[I::PushI16(1), I::ConstX]);
    assert_eq!(run(&p, [0; 256], 1000).fault, Some(Fault::BadConst));
    // 048b : 65536 ne doit pas être tronqué en 0.
    let p = program(&[5, 65_536], &[I::Const(1), I::ConstX]);
    assert_eq!(run(&p, [0; 256], 1000).fault, Some(Fault::BadConst));
    // 047b : pile vide → StackUnderflow, jamais BadConst.
    let r = run_default(&[I::ConstX]);
    assert_eq!((r.fault, r.gas_used), (Some(Fault::StackUnderflow), 2));
}

#[test]
fn shuffles_follow_the_b3_notation() {
    let swap = run_default(&[
        I::PushI16(1),
        I::PushI16(2),
        I::Swap,
        I::Trace,
        I::Trace,
        I::Halt,
    ]);
    assert_eq!(swap.trace, vec![1, 2]);
    let over = run_default(&[
        I::PushI16(1),
        I::PushI16(2),
        I::Over,
        I::Trace,
        I::Trace,
        I::Trace,
        I::Halt,
    ]);
    assert_eq!(over.trace, vec![1, 2, 1]);
    // a b c -- b c a : sur [1, 2, 3], le sommet devient 1.
    let rot = run_default(&[
        I::PushI16(1),
        I::PushI16(2),
        I::PushI16(3),
        I::Rot,
        I::Trace,
        I::Trace,
        I::Trace,
        I::Halt,
    ]);
    assert_eq!(rot.trace, vec![1, 3, 2]);
    assert_eq!(rot.gas_used, 7);
    assert_eq!(
        run_default(&[I::PushI16(5), I::Dup, I::Trace, I::Trace, I::Halt]).trace,
        vec![5, 5]
    );
    assert_eq!(
        run_default(&[I::PushI16(1), I::PushI16(2), I::Drop, I::Trace, I::Halt]).trace,
        vec![1]
    );
}

#[test]
fn underflows_are_paid_faults() {
    let cases: [(&[I], u16); 6] = [
        (&[I::Drop], 1),
        (&[I::Dup], 1),
        (&[I::Trace], 1),
        (&[I::PushI16(1), I::Swap], 2),
        (&[I::PushI16(1), I::Over], 2),
        (&[I::PushI16(1), I::PushI16(2), I::Rot], 3),
    ];
    for (instrs, gas) in cases {
        let r = run_default(instrs);
        assert_eq!(r.fault, Some(Fault::StackUnderflow), "{instrs:?}");
        assert_eq!(r.gas_used, gas, "{instrs:?}");
    }
}

#[test]
fn the_65th_value_overflows() {
    let mut instrs = full_stack();
    instrs.push(I::PushI16(0));
    let r = run_default(&instrs);
    assert_eq!((r.fault, r.gas_used), (Some(Fault::StackOverflow), 65));
    let mut instrs = full_stack();
    instrs.push(I::Dup);
    assert_eq!(run_default(&instrs).fault, Some(Fault::StackOverflow));
}

#[test]
fn gas_is_charged_before_the_stack_is_checked() {
    // Cas 026 : pile pleine et plus aucun gaz → OutOfGas, pas StackOverflow.
    let mut instrs = full_stack();
    instrs.push(I::PushI16(0));
    let r = run(&program(&[], &instrs), [0; 256], 64);
    assert_eq!((r.fault, r.gas_used), (Some(Fault::OutOfGas), 64));
}

#[test]
fn const_reads_the_table_before_pushing() {
    // D-005 : la lecture de table (BadConst) précède l'empilement (StackOverflow).
    let mut instrs = full_stack();
    instrs.push(I::Const(0));
    assert_eq!(run_default(&instrs).fault, Some(Fault::BadConst));
}

#[test]
fn a_refused_instruction_is_not_charged() {
    // DIV coûte 7, il n'en reste que 2 : refusée à l'étape 4 de B.2, donc non facturée.
    let r = run(
        &program(&[], &[I::PushI16(7), I::PushI16(1), I::Div, I::Halt]),
        [0; 256],
        4,
    );
    assert_eq!((r.fault, r.gas_used), (Some(Fault::OutOfGas), 2));
}

#[test]
fn halt_still_runs_with_no_gas_left() {
    let r = run(
        &program(&[], &[I::PushI16(1), I::Trace, I::Halt]),
        [0; 256],
        2,
    );
    assert_eq!((r.fault, r.gas_used), (None, 2));
}

#[test]
fn exact_exhaustion_reports_the_whole_budget() {
    let r = run(
        &program(&[], &[I::PushI16(1), I::Trace, I::PushI16(1)]),
        [0; 256],
        2,
    );
    assert_eq!((r.fault, r.gas_used), (Some(Fault::OutOfGas), 2));
    assert_eq!(r.trace, vec![1]);
}

#[test]
fn gas_reads_what_is_left_after_its_own_charge() {
    assert_eq!(run_default(&[I::Gas, I::Trace, I::Halt]).trace, vec![998]);
    assert_eq!(
        run_default(&[I::PushI16(0), I::Drop, I::Gas, I::Trace, I::Halt]).trace,
        vec![996]
    );
    let r = run(&program(&[], &[I::Gas, I::Trace, I::Halt]), [0; 256], 3);
    assert_eq!((r.trace, r.gas_used, r.fault), (vec![1], 3, None));
}

#[test]
fn decode_faults_cost_nothing() {
    let p = Program::new(vec![], vec![0x02, 0x00, 0x01, 0xFF]).unwrap();
    let r = run(&p, [0; 256], 1000);
    assert_eq!((r.fault, r.gas_used), (Some(Fault::BadOpcode), 1));
    let p = Program::new(vec![], vec![0x02, 0x00]).unwrap();
    let r = run(&p, [0; 256], 1000);
    assert_eq!((r.fault, r.gas_used), (Some(Fault::Truncated), 0));
    let r = run_default(&[I::PushI16(1)]);
    assert_eq!((r.fault, r.gas_used), (Some(Fault::PcOutOfRange), 1));
}

#[test]
fn initial_memory_is_handed_back() {
    let mut mem = [0; 256];
    mem[3] = 42;
    assert_eq!(run(&program(&[], &[I::Halt]), mem, 1000).mem[3], 42);
}

#[test]
fn the_trace_buffer_is_sized_once_for_the_budget() {
    // TRACE coûte 1 : au plus `budget` entrées (B.3). Le tampon est alloué dans `new`, jamais pendant le tour.
    let r = run_default(&[I::Halt]);
    assert!(r.trace.capacity() >= 1000);
}
