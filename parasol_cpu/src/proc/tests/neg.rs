use std::sync::Arc;

use crate::{ArgsBuilder, Memory, proc::IsaOp, register_names::*, test_utils::make_computer_80};

#[test]
fn can_neg_plaintext_inputs() {
    let (mut proc, _enc) = make_computer_80();

    let val1 = 14u8;
    let expected = val1.wrapping_neg();

    let memory = Arc::new(Memory::new_default_stack());

    let args = ArgsBuilder::new().arg(val1).return_value::<u8>();

    let program = memory.allocate_program(&[IsaOp::Neg(A0, A0), IsaOp::Ret()]);

    let ans = proc.run_program(program, &memory, args).unwrap();

    assert_eq!(expected, ans);
}
