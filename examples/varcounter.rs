//! Bash builtin to create a counter with a dynamic variable.

use bash_builtins::variables::{self, DynamicVariable};
use bash_builtins::{builtin_metadata, error, Args, Builtin, Result};
use std::ffi::{CStr, CString};

builtin_metadata!(
    name = "varcounter",
    create = VarCounter::default,
    short_doc = "varcounter [NAME] ...",
    long_doc = "
        Creates a counter in a dynamic variable.

        For each NAME, a variable $NAME will be incremented each time it is
        read.

        The value in the counter can be modified with NAME=<N>.
    ",
);

#[derive(Default)]
struct VarCounter;

impl Builtin for VarCounter {
    fn call(&mut self, args: &mut Args) -> Result<()> {
        args.no_options()?;

        for name in args.string_arguments() {
            variables::bind(name?, Counter(0))?;
        }

        Ok(())
    }
}

/// Counter for the dynamic variable.
struct Counter(isize);

impl DynamicVariable for Counter {
    fn get(&mut self) -> Option<CString> {
        let value = CString::new(format!("{}", self.0)).ok();
        self.0 += 1;
        value
    }

    fn set(&mut self, value: &CStr) {
        self.0 = match value.to_str().map(str::parse) {
            Ok(Ok(n)) => n,
            _ => {
                error!("invalid value: {:?}", value);
                return;
            }
        }
    }
}
