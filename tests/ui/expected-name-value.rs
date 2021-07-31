use bash_builtins::BuiltinOptions;

#[derive(BuiltinOptions)]
enum Opt {
    #[opt('a')]
    A,
}

fn main() {}
