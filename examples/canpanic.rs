//! Bash builtin that can panic.

use bash_builtins::{builtin_metadata, Args, Builtin, Result};
use std::io::{stdout, Write};

builtin_metadata!(name = "canpanic", create = CanPanic::default);

#[derive(Default)]
struct CanPanic;

impl Builtin for CanPanic {
    fn call(&mut self, args: &mut Args) -> Result<()> {
        if args.string_arguments().any(|a| a == Ok("panic")) {
            panic!("DO PANIC");
        }

        stdout().write_all(b"OK\n")?;
        Ok(())
    }
}
