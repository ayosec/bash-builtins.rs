//! Bash builtin with options with non-required arguments.

use bash_builtins::{builtin_metadata, Args, Builtin, BuiltinOptions, Result};
use std::io::{self, BufWriter, Write};

builtin_metadata!(name = "nonrequiredargs", create = NonRequiredArgs::default);

#[derive(BuiltinOptions, Debug)]
enum Opt<'a> {
    #[opt = 'f']
    Foo(Option<u64>),

    #[opt = 'b']
    Bar(Option<&'a str>),
}

#[derive(Default)]
struct NonRequiredArgs;

impl Builtin for NonRequiredArgs {
    fn call(&mut self, args: &mut Args) -> Result<()> {
        let stdout_handle = io::stdout();
        let mut output = BufWriter::new(stdout_handle.lock());

        writeln!(&mut output, " -")?;
        for opt in args.options::<Opt>() {
            writeln!(&mut output, "{:?}", opt)?;
        }

        Ok(())
    }
}
