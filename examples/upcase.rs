//! Bash builtin to print the uppercase equivalent of the arguments.

use bash_builtins::{builtin_metadata, Args, Builtin, Result};
use std::io::{self, BufWriter, Write};

builtin_metadata!(
    name = "upcase",
    create = Upcase::default,
    short_doc = "upcase [args]",
    long_doc = "
        Print the uppercase equivalent of the arguments.
    ",
);

#[derive(Default)]
struct Upcase;

impl Builtin for Upcase {
    fn call(&mut self, args: &mut Args) -> Result<()> {
        args.no_options()?;

        let stdout_handle = io::stdout();
        let mut output = BufWriter::new(stdout_handle.lock());

        for (index, argument) in args.string_arguments().enumerate() {
            let sep = if index > 0 { " " } else { "" };
            write!(&mut output, "{}{}", sep, argument?.to_uppercase())?;
        }

        output.write_all(b"\n")?;

        Ok(())
    }
}
