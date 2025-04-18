use std::{ffi::CString, sync::Arc};

use parasol_runtime::{ComputeKey, Encryption, Evaluation};

use crate::{Buffer, Error, FheApplication, FheComputer, Symbol, error::Result};

/// Runs a program by generating a new [`FheComputer`]. This function is meant
/// for simple testing of a program; for full applications see the
/// [`FheComputer`] struct.
pub fn run_program(
    compute_key: ComputeKey,
    elf_file: &[u8],
    program_name: &str,
    arguments: &[Buffer],
) -> Result<Vec<Buffer>> {
    let enc = Encryption::default();
    let eval = Evaluation::with_default_params(Arc::new(compute_key));
    let mut proc = FheComputer::new(&enc, &eval);

    let fhe_app = FheApplication::parse_elf(elf_file)?;

    let c_program_name = CString::new(program_name).map_err(Error::CStringCreationError)?;

    let program = fhe_app
        .get_program(&Symbol::new(c_program_name.as_c_str()))
        .ok_or(Error::ElfSymbolNotFound(program_name.to_string()))?;

    proc.run_program(program, arguments)?;

    Ok(arguments.to_owned())
}

#[cfg(test)]
mod tests {
    use std::borrow::Borrow;

    use super::*;
    use crate::test_utils::buffer_from_value_128;
    use parasol_runtime::{
        Encryption,
        test_utils::{get_compute_key_128, get_secret_keys_128},
    };

    const CMUX_ELF: &[u8] = include_bytes!("../tests/test_data/cmux.o");

    #[test]
    fn test_run_program() {
        let compute_key = get_compute_key_128();
        let compute_key: &ComputeKey = compute_key.borrow();
        let enc = Encryption::default();

        let bound = 20u8;
        let a = 6u8;
        let b = 3u8;

        let buffer_0 = buffer_from_value_128(bound, &enc, true);
        let buffer_1 = buffer_from_value_128(a, &enc, true);
        let buffer_2 = buffer_from_value_128(b, &enc, true);
        let output_buffer = buffer_from_value_128(0u8, &enc, true);

        let arguments = vec![buffer_0, buffer_1, buffer_2, output_buffer];

        let result = run_program(compute_key.clone(), CMUX_ELF, "cmux", &arguments).unwrap();

        let output = result[3]
            .cipher_try_into_value::<u8>(&enc, &get_secret_keys_128())
            .unwrap();
        let expected = if bound > 10 { a } else { b };

        assert_eq!(expected, output);
    }
}
