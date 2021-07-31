//! Functions to write log messages.

use crate::ffi;
use std::os::raw::{c_char, c_int};

/// Shows the usage text for this builtin.
///
/// This function should be used when an invalid option is found.
#[inline]
pub fn show_usage() {
    unsafe {
        ffi::builtin_usage();
    }
}

/// Shows the help text for this builtin.
#[inline]
pub fn show_help() {
    unsafe {
        ffi::builtin_help();
    }
}

/// Display an error when an argument is missing.
///
/// It uses the `sh_needarg` function from Bash.
pub fn missing_argument() {
    unsafe {
        let msg = [ffi::list_opttype as _, ffi::list_optopt as _, 0];
        ffi::sh_needarg(msg.as_ptr());
    }
}

macro_rules! log_fn {
    ($name:ident, $bash_fn:ident, $doc:literal) => {
        #[inline]
        #[doc = $doc]
        pub fn $name<S: AsRef<[u8]>>(msg: S) {
            const MSG_FORMAT: *const c_char = b"%.*s\0".as_ptr().cast();
            let bytes = msg.as_ref();
            unsafe {
                $crate::ffi::$bash_fn(MSG_FORMAT, bytes.len() as c_int, bytes.as_ptr());
            }
        }
    };
}

// Errors.

log_fn!(
    error,
    builtin_error,
    "Shows an error message using the `builtin_error()` function from Bash."
);

/// Macro to use [`error()`] with a [`format`](std::format) string.
///
/// # Example
///
/// See the example in the [`warning!()`] documentation.
#[macro_export]
macro_rules! error {
    ($($arg:tt)+) => {
        $crate::log::error(format!($($arg)+))
    }
}

// Warnings.

log_fn!(
    warning,
    builtin_warning,
    "Shows a warning message using the `builtin_warning()` function from Bash."
);

/// Macro to use [`warning()`] with a [`format`](std::format) string.
///
/// # Example
///
/// ```
/// # use bash_builtins::*;
/// # use std::{fs, io::{self, BufWriter, Write}};
/// struct FileSize;
///
/// impl Builtin for FileSize {
///     fn call(&mut self, args: &mut Args) -> Result<()> {
///         let stdout_handle = io::stdout();
///         let mut output = BufWriter::new(stdout_handle.lock());
///
///         let mut result = Ok(());
///
///         for path in args.path_arguments() {
///             match fs::metadata(&path) {
///                 Ok(m) => {
///                     writeln!(&mut output, "{}\t{}", m.len(), path.display())?;
///                 }
///
///                 Err(e) => {
///                     warning!("{}: {}", path.display(), e);
///                     result = Err(Error::ExitCode(1));
///                 }
///             }
///         }
///
///         result
///     }
/// }
/// ```
#[macro_export]
macro_rules! warning {
    ($($arg:tt)+) => {
        $crate::log::warning(format!($($arg)+))
    }
}
