use bash_builtins::BuiltinOptions;

#[derive(BuiltinOptions)]
enum Opt {
    #[opt = 0]
    A,
}

fn main() {}
