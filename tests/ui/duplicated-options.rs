use bash_builtins::BuiltinOptions;

#[derive(BuiltinOptions)]
enum Opt {
    #[opt = 'a']
    A,

    #[opt = 'a']
    A2,
}

fn main() {}
