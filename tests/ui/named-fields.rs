use bash_builtins::BuiltinOptions;

#[derive(BuiltinOptions)]
enum Opt {
    #[opt = 'a']
    A(String),

    #[opt = 'b']
    B { s: String }
}

fn main() {}
