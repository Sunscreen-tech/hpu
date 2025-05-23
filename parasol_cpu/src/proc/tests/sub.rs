use std::sync::Arc;

use crate::{
    ArgsBuilder, Memory,
    proc::IsaOp,
    register_names::*,
    test_utils::{MaybeEncryptedUInt, make_computer_80},
};

use parasol_runtime::test_utils::get_secret_keys_80;

#[test]
fn can_sub_inputs() {
    let test = |((val1, enc1), (val2, enc2), expected_sum)| {
        let (mut proc, enc) = make_computer_80();
        let sk = get_secret_keys_80();

        let encrypted_computation = enc1 || enc2;

        let args = ArgsBuilder::new()
            .arg(MaybeEncryptedUInt::<32>::new(val1 as u64, &enc, &sk, enc1))
            .arg(MaybeEncryptedUInt::<32>::new(val2 as u64, &enc, &sk, enc2))
            .return_value::<MaybeEncryptedUInt<32>>();

        let memory = Arc::new(Memory::new_default_stack());

        let program = memory.allocate_program(&[IsaOp::Sub(A0, A0, A1), IsaOp::Ret()]);

        let ans_sum = proc.run_program(program, &memory, args).unwrap();

        let ans_sum = ans_sum.get(&enc, &sk);

        assert_eq!(
            expected_sum, ans_sum,
            "val1: {:#02x}, val2: {:#02x}, expected_sum: {:#02x}, ans_sum: {:#02x}, encrypted computation?: {}",
            val1, val2, expected_sum, ans_sum, encrypted_computation
        );
    };

    for test_case in [
        // Unencrypted tests
        ((15, false), (12, false), 3),
        ((0u32, false), (1u32, false), u32::MAX),
        ((0u32, false), (0xffff_ffffu32, false), 1u32),
        // Encrypted tests
        ((15, true), (12, false), 3),
        ((0u32, false), (1u32, true), u32::MAX),
        ((0u32, true), (0xffff_ffffu32, true), 1u32),
    ] {
        test(test_case);
    }
}

#[test]
fn can_sub_borrow_inputs() {
    let test = |(
        (val1, enc1),
        (val2, enc2),
        (input_borrow, enc_input_borrow),
        expected_sum,
        expected_borrow,
    )| {
        let (mut proc, enc) = make_computer_80();
        let sk = get_secret_keys_80();

        let encrypted_computation = enc1 || enc2 || enc_input_borrow;

        let memory = Arc::new(Memory::new_default_stack());

        let program = memory.allocate_program(&[
            IsaOp::Trunc(A2, A2, 1),
            IsaOp::SubB(A0, A1, A0, A1, A2),
            IsaOp::Zext(A1, A1, 8),
            IsaOp::Ret(),
        ]);

        let args = ArgsBuilder::new()
            .arg(MaybeEncryptedUInt::<32>::new(val1 as u64, &enc, &sk, enc1))
            .arg(MaybeEncryptedUInt::<32>::new(val2 as u64, &enc, &sk, enc1))
            .arg(MaybeEncryptedUInt::<32>::new(
                input_borrow as u64,
                &enc,
                &sk,
                enc_input_borrow,
            ))
            .return_value::<[MaybeEncryptedUInt<8>; 5]>();

        // Diff || borrow is 5 bytes: 4 for the difference and 1 for the borrow
        let result = proc.run_program(program, &memory, args).unwrap();

        let ans_diff = u32::from_le_bytes(
            result
                .iter()
                .take(4)
                .map(|x| x.get(&enc, &sk))
                .collect::<Vec<_>>()
                .try_into()
                .unwrap(),
        );

        let ans_borrow = result[4].get(&enc, &sk) as u32;

        assert_eq!(
            expected_sum, ans_diff,
            "val1: {:#02x}, val2: {:#02x}, input_borrow: {:#02x}, expected_sum: {:#02x}, ans_sum: {:#02x}, encrypted computation?: {}",
            val1, val2, input_borrow, expected_sum, ans_diff, encrypted_computation
        );

        assert_eq!(
            expected_borrow, ans_borrow,
            "val1: {:#02x}, val2: {:#02x}, input_borrow: {:#02x}, expected_borrow: {:#02x}, ans_borrow: {:#02x}, encrypted computation?: {}",
            val1, val2, input_borrow, expected_borrow, ans_borrow, encrypted_computation
        );
    };

    for test_case in [
        // Plaintext computations
        ((15, false), (12, false), (0, false), 3, 0), // sub no borrow, no borrow out
        ((15, false), (12, false), (1, false), 2, 0), // sub borrow, no borrow out
        ((0u32, false), (1u32, false), (0u32, false), u32::MAX, 1u32), // sub no borrow, borrow out
        (
            (0u32, false),
            (1u32, false),
            (1u32, false),
            u32::MAX - 1,
            1u32,
        ), // sub borrow, borrow out
        (
            (0u32, false),
            (0xffff_ffffu32, false),
            (0u32, false),
            1u32,
            1u32,
        ), // sub no borrow, borrow out
        (
            (0u32, false),
            (0xffff_ffffu32, false),
            (1u32, false),
            0u32,
            1u32,
        ), // sub borrow, borrow out
        // Encrypted computations
        ((15, true), (12, false), (0, false), 3, 0), // sub no borrow, no borrow out
        ((15, false), (12, true), (1, false), 2, 0), // sub borrow, no borrow out
        ((0u32, false), (1u32, false), (0u32, true), u32::MAX, 1u32), // sub no borrow, borrow out
        (
            (0u32, true),
            (1u32, true),
            (1u32, false),
            u32::MAX - 1,
            1u32,
        ), // sub borrow, borrow out
        (
            (0u32, true),
            (0xffff_ffffu32, false),
            (0u32, true),
            1u32,
            1u32,
        ), // sub no borrow, borrow out
        (
            (0u32, false),
            (0xffff_ffffu32, true),
            (1u32, true),
            0u32,
            1u32,
        ), // sub borrow, borrow out
        (
            (0u32, true),
            (0xffff_ffffu32, true),
            (1u32, true),
            0u32,
            1u32,
        ), // sub borrow, borrow out
    ] {
        test(test_case);
    }
}

#[test]
fn sub_use_same_dst_and_src() {
    let (mut proc, _enc) = make_computer_80();

    let memory = Arc::new(Memory::new_default_stack());

    let args = ArgsBuilder::new().arg(10u32).return_value::<u32>();

    let ans = proc
        .run_program(
            memory.allocate_program(&[IsaOp::Sub(A0, A0, A0), IsaOp::Ret()]),
            &memory,
            args,
        )
        .unwrap();

    assert_eq!(ans, 0);
}
