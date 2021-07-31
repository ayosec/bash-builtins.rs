//! This crate provides utilities to implement loadable builtins for bash. It
//! reuses functions provided by bash as much as possible in order to keep
//! compatibility with existing builtins.
//!
//! # What are Loadable Builtins
//!
//! Bash, like most shells, has [*builtins*]. A builtin looks like a regular
//! command, but it is executed in the shell process. Some builtins are used to
//! interact with the shell (like `cd` or `jobs`), and others are common
//! utilities (like `printf` or `test`).
//!
//! New builtins can be created in a running shell as [*loadable builtins*],
//! using code from a dynamic library (for example, a `.so` file in Linux). This
//! is done with the [`enable -f`] command.
//!
//! For example, if the crate name is `foo`, and it defines a `bar` builtin,
//! the following commands will load it:
//!
//! ```notrust
//! $ cargo build --release
//!
//! $ enable -f target/release/libfoo.so bar
//! ```
//!
//! [*loadable builtins*]: https://git.savannah.gnu.org/cgit/bash.git/tree/examples/loadables/README?h=bash-5.1
//! [*builtins*]: https://www.gnu.org/software/bash/manual/html_node/Shell-Builtin-Commands.html
//! [`enable -f`]: https://www.gnu.org/software/bash/manual/html_node/Bash-Builtins.html#index-enable
//!
//! # Usage
//!
//! ## Crate Configuration
//!
//! The crate where the builtin is implemented has to include `cdylib` in its
//! [`crate-type` field]. This is required to build a dynamic library.
//!
//! `Cargo.toml` should contain something similar to this:
//!
//! ```notrust
//! [dependencies]
#![doc = concat!(
    env!("CARGO_PKG_NAME"),
    " = \"",
    env!("CARGO_PKG_VERSION"), "\"")
]
//!
//! [lib]
//! crate-type = [ "cdylib" ]
//! ```
//!
//! [`crate-type` field]: https://doc.rust-lang.org/cargo/reference/cargo-targets.html#the-crate-type-field
//!
//! ## Main Items
//!
//! These are the main items to implement a builtin:
//!
//! * The [`builtin_metadata!()`] macro, to generate functions and declarations
//!   required by bash.
//!
//! * The [`BuiltinOptions`] derive macro, to generate an option parser.
//!
//! * The [`Builtin`] trait, to provide the builtin functionality.
//!
//! * The [`Args`] type, to access to the command-line arguments.
//!
//! A single crate can contain multiple builtins. Each builtin requires its own
//! call to [`builtin_metadata!()`].
//!
//! ## Basic Structure
//!
//! ```
//! use bash_builtins::{builtin_metadata, Args, Builtin, BuiltinOptions, Result};
//!
//! builtin_metadata!(
//! #   name = "SomeName", create = SomeName::default,
//!     // Builtin metadata.
//! );
//!
//! # #[derive(Default)]
//! struct SomeName {
//!     // Fields to store state.
//! }
//!
//! #[derive(BuiltinOptions)]
//! enum Opt {
//!     // Options from the command-line arguments.
//! }
//!
//! impl Builtin for SomeName {
//!     fn call(&mut self, args: &mut Args) -> Result<()> {
//!         // builtin implementation
//!         Ok(())
//!     }
//! }
//! ```
//!
//! # Example
//!
//! The following example is a simple counter.
//!
//! It accepts some options to modify the stored value.
//!
#![doc = concat!("```\n", include_str!("../examples/counter.rs"), "```")]
//!
//! This example is available in the `examples/counter.rs` file of the Git
//! repository of this crate.
//!
//! It can be tested with the following commands:
//!
//! ```notrust
//! $ cargo build --release --examples
//!
//! $ enable -f target/release/examples/libcounter.so counter
//!
//! $ counter
//! 0
//!
//! $ counter
//! 1
//!
//! $ help counter
//! counter: counter [-r] [-s value] [-a value]
//!     Print a value, and increment it.
//!
//!     Options:
//!       -r        Reset the value to 0.
//!       -s        Set the counter to a specific value.
//!       -a        Increment the counter by a value.
//!
//! $ counter -s -100
//!
//! $ counter
//! -100
//!
//! $ counter abcd
//! bash: counter: too many arguments
//!
//! $ enable -d counter
//!
//! $ counter
//! bash: counter: command not found
//! ```
//!
//! # Builtin Documentation
//!
//! A bash builtin has two fields for the documentation:
//!
//! * [`short_doc`]: a single line of text to describe how to use the builtin.
//! * [`long_doc`]: a detailed explanation of the builtin.
//!
//! [`short_doc`]: bash_builtins_macro::builtin_metadata!()#short_doc-optional
//! [`long_doc`]: bash_builtins_macro::builtin_metadata!()#long_doc-optional
//!
//! Both fields are optional, but it is recommend to include them.
//!
//! See the documentation of the [`builtin_metadata!()`] macro for more details.
//!
//! # Builtin Initialization
//!
//! When the builtin is loaded, the function given in either [`create`] or
//! [`try_create`] is executed. This function will create a new instance of a
//! type that implements the [`Builtin`] trait.
//!
//! [`try_create`] is used if the initialization mail fails.
//!
//! ## Example of a Fallible Initialization
//!
//! ```
//! # use bash_builtins::*;
//! use std::fs::File;
//!
//! builtin_metadata!(
//! #   name = "x",
//!     // …
//!     try_create = Foo::new,
//! );
//!
//! struct Foo {
//!     file: File
//! }
//!
//! impl Foo {
//!     fn new() -> Result<Foo> {
//!         let file = File::open("/some/config/file")?;
//!         Ok(Foo { file })
//!     }
//! }
//!
//! impl Builtin for Foo {
//!     fn call(&mut self, args: &mut Args) -> Result<()> {
//! #       let _ = args;
//!         // …
//!         Ok(())
//!     }
//! }
//! ```
//!
//! [`create`]: bash_builtins_macro::builtin_metadata!()#create
//! [`try_create`]: bash_builtins_macro::builtin_metadata!()#try_create
//!
//! # Builtin Removal
//!
//! A loadable builtin can be removed from a running shell with `enable -d`.
//!
//! If a builtin needs to run any cleanup process when it is unloaded, then it
//! must implement [`Drop`](std::ops::Drop). The value is dropped just before
//! the builtin is deleted.
//!
//! # Parsing Command Line Options
//!
//! Bash builtins use an internal implementation of `getopt()` to parse command
//! line arguments. The [`BuiltinOptions`] derive macro provides an easy-to-use
//! method to generate an options parser on top of this `getopt()`.
//!
//! See the macro documentation for details on how to use it.
//!
//! # Error Handling
//!
//! The macros [`error!()`] and [`warning!()`] can be used to produce log
//! messages to the standard error stream (*stderr*). They use the bash
//! functions `builtin_error` and `builtin_warning`.
//!
//! [Recoverable errors] can be used as the return value of [`Builtin::call`],
//! usually with the [`?` operator]. In such cases, the message from the error
//! is printed to *stderr*, and the exit code of the builtin is `1`.
//!
//! [Recoverable Errors]: https://doc.rust-lang.org/book/ch09-02-recoverable-errors-with-result.html
//! [`?` operator]: https://doc.rust-lang.org/book/ch09-02-recoverable-errors-with-result.html#a-shortcut-for-propagating-errors-the--operator
//! Use [`Error::ExitCode`] to return a specific exit code. See the [`Error`]
//! documentation for more details.
//!
//! # Panic Handling
//!
//! Panics are captured with [`panic::catch_unwind`], so they should not reach
//! the bash process.
//!
//! After a panic the builtin is [“poisoned”], and any attempt to use it will
//! print the error `invalid internal state` on the terminal. Users will have
//! to remove it (`enable -d`) and enable it again. Also, when a poisoned
//! builtin is removed, its destructors (if any) are not executed.
//!
//! If you want to avoid this behaviour you have to use [`panic::catch_unwind`]
//! in your own code.
//!
//! [“poisoned”]: https://doc.rust-lang.org/stable/std/sync/struct.Mutex.html#poisoning
//!
//! It is important to *not* set the [`panic` setting] to `"abort"`. If the
//! dynamic library is built with this setting, a panic will terminate the bash
//! process.
//!
//! [`panic::catch_unwind`]: std::panic::catch_unwind
//! [`panic` setting]: https://doc.rust-lang.org/cargo/reference/profiles.html#panic
//! [`BuiltinOptions`]: bash_builtins_macro::BuiltinOptions

mod args;
mod errors;

pub mod convert;
pub mod log;

#[doc(hidden)]
pub mod ffi;

// Re-export macros.
pub use bash_builtins_macro::{builtin_metadata, BuiltinOptions};

// Re-export public items.
pub use args::{Args, BuiltinOptions};
pub use errors::{Error, Result};

/// The `Builtin` trait contains the implementation for a bash builtin.
pub trait Builtin: Send {
    /// Method invoked when the builtin is typed in the prompt.
    ///
    /// It returns an instance of [`Result`]. The value `Ok(())` returns the
    /// exit code `0` to the shell. [`Error::ExitCode`] can be used to return a
    /// specific exit code.
    ///
    /// Any error type that implements [`std::error::Error`] can be used with
    /// the `?` operator to return an error from this method.
    ///
    /// Command-line arguments are read from the [`Args`] instance.
    fn call(&mut self, args: &mut Args) -> Result<()>;
}
