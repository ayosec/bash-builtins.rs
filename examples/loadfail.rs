//! Bash builtin that always fails to load.

use bash_builtins::{builtin_metadata, Args, Builtin, Result};

builtin_metadata!(name = "loadfail", try_create = try_create);

// Returns an error to indicates that the builtin could not
// be initialized.
fn try_create() -> std::result::Result<LoadFail, &'static str> {
    Err("something really bad happened")
}

struct LoadFail;

impl Builtin for LoadFail {
    fn call(&mut self, _args: &mut Args) -> Result<()> {
        unreachable!()
    }
}
