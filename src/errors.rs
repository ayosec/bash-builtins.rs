//! Types to handle errors.

use crate::ffi;
use std::fmt;
use std::os::raw::c_int;

/// The error type for [`Builtin::call`].
///
/// Usually, you don't need to construct this type manually. Instead, use the
/// [`?` operator] for any [`Result`] in the body of the [`Builtin::call`]
/// method, and errors will be converted to this type.
///
/// However, if you want to return a specific exit code, use the
/// [`ExitCode`] variant.
///
/// [`?` operator]: https://doc.rust-lang.org/book/ch09-02-recoverable-errors-with-result.html#a-shortcut-for-propagating-errors-the--operator
/// [`Builtin::call`]: crate::Builtin::call
/// [`ExitCode`]: Error::ExitCode
/// [`Result`]: std::result::Result
#[derive(Debug)]
pub enum Error {
    /// Syntax error in usage.
    Usage,

    /// Exit with a specific code.
    ///
    /// # Example
    ///
    /// ```
    /// use bash_builtins::{Args, Builtin, Error::ExitCode, Result};
    ///
    /// # struct SomeName;
    /// impl Builtin for SomeName {
    ///     fn call(&mut self, args: &mut Args) -> Result<()> {
    ///         // In this builtin, we return `127` if there are
    ///         // no arguments.
    ///         if args.is_empty() {
    ///             return Err(ExitCode(127));
    ///         }
    ///
    ///         // …
    ///
    ///         Ok(())
    ///     }
    /// }
    /// ```
    ExitCode(c_int),

    /// Wrapper for any error.
    ///
    /// This variant is used when the builtin propagates any error inside
    /// [`Builtin::call`].
    ///
    /// # Example
    ///
    /// ```
    /// use std::fs;
    /// use bash_builtins::{Args, Builtin, Error::ExitCode, Result};
    ///
    /// # struct SomeName;
    /// impl Builtin for SomeName {
    ///     fn call(&mut self, args: &mut Args) -> Result<()> {
    /// #       let _ = args;
    ///
    ///         // fs::read can return an `io::Error`, which is wrapped
    ///         // by `GenericError` and then used as the return value.
    ///         let _ = fs::read("/some/config/file")?;
    ///
    ///         // …
    ///
    ///         Ok(())
    ///     }
    /// }
    /// ```
    ///
    /// [`Builtin::call`]: crate::Builtin::call
    GenericError(Box<dyn std::error::Error>),
}

impl Error {
    /// Returns `true` if this error should be printed to [`stderr`] when
    /// it is the result of [`Builtin::call`].
    ///
    /// [`Builtin::call`]: crate::Builtin::call
    /// [`stderr`]: std::io::stderr
    #[doc(hidden)]
    pub fn print_on_return(&self) -> bool {
        let ignore = matches!(self, Error::Usage | Error::ExitCode(_));
        !ignore
    }

    /// Numeric exit code for the builtin invocation.
    #[doc(hidden)]
    pub fn exit_code(&self) -> c_int {
        match self {
            Error::Usage => ffi::exit::EX_USAGE,
            Error::ExitCode(s) => *s,
            _ => ffi::exit::EXECUTION_FAILURE,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Usage => fmt.write_str("usage error"),
            Error::ExitCode(s) => write!(fmt, "exit code {}", s),
            Error::GenericError(e) => e.fmt(fmt),
        }
    }
}

impl<E> From<E> for Error
where
    E: std::error::Error + 'static,
{
    fn from(error: E) -> Self {
        Error::GenericError(Box::new(error))
    }
}

/// A specialized [`Result`] type for this crate.
///
/// [`Result`]: std::result::Result
pub type Result<T> = std::result::Result<T, Error>;
