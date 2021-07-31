use bash_builtins::BuiltinOptions;

#[derive(BuiltinOptions)]
enum Opt {
    #[opt = 'a']
    A,

    B,
}

fn main() {}
