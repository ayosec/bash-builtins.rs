use bash_builtins::BuiltinOptions;

#[derive(BuiltinOptions)]
enum Opt {
    #[opt = 'a']
    A,

    #[opt = 'b']
    B(String),

    #[opt = 'c']
    C(String, String),
}

fn main() {}
