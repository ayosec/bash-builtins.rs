//! Bash builtin with a Drop implementation.

use bash_builtins::{builtin_metadata, Args, Builtin, Result};
use std::io::{stdout, Write};

builtin_metadata!(name = "unload", create = Unload::default);

#[derive(Default)]
struct Unload(usize);

impl Builtin for Unload {
    fn call(&mut self, args: &mut Args) -> Result<()> {
        args.no_options()?;
        self.0 += 1;
        writeln!(stdout(), "{}", self.0)?;
        Ok(())
    }
}

impl Drop for Unload {
    fn drop(&mut self) {
        let _ = writeln!(stdout(), "[drop] {}", self.0);
    }
}
