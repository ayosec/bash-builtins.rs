use bash_builtins::{Args, BuiltinOptions, Builtin, Result};

#[derive(BuiltinOptions)]
enum Opt<'a> {
    #[opt = 's']
    S(&'a str)
}

struct Foo(&'static str);

impl Builtin for Foo {
    fn call(&mut self, args: &mut Args) -> Result<()> {
        for opt in args.options() {
            match opt? {
                Opt::S(s) => self.0 = s,
            }
        }

        Ok(())
    }
}

fn main() {}
