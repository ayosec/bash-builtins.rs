//! This module contains functions to get, set, or unset shell variables.
//!
//! Use [`set`] and [`unset`] to modify shell variables.
//!
//! Use the `find` functions to access the value contained in existing shell
//! variables. [`find_raw`] provides access to the raw pointer owned by bash,
//! and both [`find`] and [`find_as_string`] provides a safe interface to such
//! value.
//!
//! Use [`array_set`] and [`array_get`] to access the elements in an indexed
//! array.
//!
//! Use [`assoc_get`] and [`assoc_get`] to access the elements in an associative
//! array.
//!
//! ## Example
//!
//! The following example uses the shell variable `$SOMENAME_LIMIT` to set the
//! configuration value for the builtin. If it is not present, or its value is
//! not a valid `usize`, it uses a default value
//!
//! ```
//! use bash_builtins::variables;
//!
//! const DEFAULT_LIMIT: usize = 1024;
//!
//! const LIMIT_VAR_NAME: &str = "SOMENAME_LIMIT";
//!
//! fn get_limit() -> usize {
//!     variables::find_as_string(LIMIT_VAR_NAME)
//!         .as_ref()
//!         .and_then(|v| v.to_str().ok())
//!         .and_then(|v| v.parse().ok())
//!         .unwrap_or(DEFAULT_LIMIT)
//! }
//! ```
//!
//! # Dynamic Variables
//!
//! Dynamic variables are shell variables that use custom functions each time
//! they are accessed (like `$SECONDS` or `$RANDOM`).
//!
//! Use [`bind`] to create a dynamic variable with any type implementing
//! [`DynamicVariable`].

use crate::ffi::variables as ffi;
use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::fmt;
use std::os::raw::c_char;
use std::ptr::NonNull;

mod arrays;
mod assoc;
mod dynvars;

pub use arrays::{array_get, array_set};
pub use assoc::{assoc_get, assoc_set};
pub use dynvars::DynamicVariable;

/// Returns a string with the value of the shell variable `name`.
///
/// If the shell variable does not exist, or its value is an array, the function
/// returns `None`.
///
/// # Example
///
/// ```no_run
/// use bash_builtins::variables;
///
/// let var = variables::find_as_string("VAR_NAME");
///
/// if let Some(value) = var.as_ref().and_then(|v| v.to_str().ok()) {
///     // `value` is a `&str` here.
/// #   let _ = value;
/// } else {
///     // `$VAR_NAME` is missing, an array, or contains
///     // invalid UTF-8 data.
/// }
/// ```
pub fn find_as_string(name: &str) -> Option<CString> {
    unsafe { find_raw(name).and_then(|var| var.as_str().map(|cstr| cstr.to_owned())) }
}

/// Returns a copy of the value of the shell variable referenced by `name`.
///
/// If the shell variable does not exist, it returns `None`.
///
/// Use [`find_as_string`] if you want to skip arrays.
pub fn find(name: &str) -> Option<Variable> {
    unsafe { find_raw(name).map(|var| var.get()) }
}

/// Returns a reference to the address of the shell variable referenced by
/// `name`.
///
/// Using this reference is unsafe because the memory is owned by bash. Whenever
/// possible, use [`find`] or [`find_as_string`].
pub fn find_raw(name: &str) -> Option<RawVariable> {
    let name = CString::new(name).ok()?;
    let shell_var = unsafe { ffi::find_variable(name.as_ptr()) as *mut _ };

    NonNull::new(shell_var).map(RawVariable)
}

/// Sets the value of the shell variable referenced by `name`.
///
/// `value` is not required to be valid UTF-8, but it can't contain any nul
/// byte.
pub fn set<T>(name: &str, value: T) -> Result<(), VariableError>
where
    T: AsRef<[u8]>,
{
    let name = CString::new(name).map_err(|_| VariableError::InvalidName)?;
    let value = CString::new(value.as_ref()).map_err(|_| VariableError::InvalidValue)?;

    let res = unsafe {
        if ffi::legal_identifier(name.as_ptr()) == 0 {
            return Err(VariableError::InvalidName);
        }

        ffi::bind_variable(name.as_ptr(), value.as_ptr(), 0)
    };

    if res.is_null() {
        Err(VariableError::InvalidValue)
    } else {
        Ok(())
    }
}

/// Unset the shell variable referenced by `name`.
///
/// Returns `true` if the shell variable is removed.
pub fn unset(name: &str) -> bool {
    let name = match CString::new(name) {
        Ok(s) => s,
        Err(_) => return false,
    };

    unsafe { ffi::unbind_variable(name.as_ptr()) == 0 }
}

/// Bind the shell variable referenced by `name` to an instance of
/// [`DynamicVariable`].
///
/// See the documentation of [`DynamicVariable`] for details on how to define a
/// dynamic variable.
pub fn bind(name: &str, dynvar: impl DynamicVariable + 'static) -> Result<(), VariableError> {
    dynvars::bind_dynvar(name, Box::new(dynvar) as Box<dyn DynamicVariable>)
}

/// An error from a shell variable operation, like [`set`] or [`bind`].
#[derive(Debug)]
pub enum VariableError {
    InvalidName,
    InvalidValue,
    NotAssocArray,
    InternalError(&'static str),
}

impl fmt::Display for VariableError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match self {
            VariableError::InvalidName => fmt.write_str("invalid variable name"),
            VariableError::InvalidValue => fmt.write_str("invalid variable value"),
            VariableError::NotAssocArray => fmt.write_str("variable is not an associative array"),
            VariableError::InternalError(cause) => write!(fmt, "internal error: {}", cause),
        }
    }
}

impl std::error::Error for VariableError {}

/// Contains the value of a shell variable.
///
/// Use [`find`] or [`RawVariable::get`] to get this value.
///
/// # Example
///
/// A function to print the value of `var`.
///
/// ```
/// use bash_builtins::variables::Variable;
/// use std::io::{self, Write};
///
/// fn print<W>(mut output: W, name: &str, var: &Variable) -> io::Result<()>
/// where
///     W: Write,
/// {
///     match var {
///         Variable::Str(s) => {
///             writeln!(output, "{} = {:?}", name, s)?;
///         }
///
///         Variable::Array(a) => {
///             for (idx, elem) in a.iter().enumerate() {
///                 writeln!(&mut output, "{}[{}] = {:?}", name, idx, elem)?;
///             }
///         }
///
///         Variable::Assoc(a) => {
///             for (key, value) in a.iter() {
///                 writeln!(&mut output, "{}[{:?}] = {:?}", name, key, value)?;
///             }
///         }
///     }
///
///     Ok(())
/// }
/// ```
#[derive(Debug)]
pub enum Variable {
    /// A single string.
    Str(CString),

    /// An indexed [array](https://www.gnu.org/software/bash/manual/html_node/Arrays.html).
    Array(Vec<CString>),

    /// An associative [array](https://www.gnu.org/software/bash/manual/html_node/Arrays.html).
    ///
    /// These shell variables are initialized with `declare -A`.
    Assoc(HashMap<CString, CString>),
}

/// Raw reference to a shell variable.
///
/// Every method is unsafe because this type contains a raw pointer to an
/// address owned by bash.
///
/// Whenever possible, use [`find`] or [`find_as_string`] functions to get the
/// value of a shell variable.
#[derive(Debug)]
pub struct RawVariable(NonNull<ffi::ShellVar>);

impl RawVariable {
    /// Returns `true` if the shell variable contains an indexed array.
    ///
    /// # Safety
    ///
    /// This method is unsafe because it does not check that the address of the
    /// shell variable is still valid.
    pub unsafe fn is_array(&self) -> bool {
        self.0.as_ref().attributes & ffi::ATT_ARRAY != 0
    }

    /// Returns `true` if the shell variable contains an associative array.
    ///
    /// # Safety
    ///
    /// This method is unsafe because it does not check that the address of the
    /// shell variable is still valid.
    pub unsafe fn is_assoc(&self) -> bool {
        self.0.as_ref().attributes & ffi::ATT_ASSOC != 0
    }

    /// Extracts the contents of the shell variable, and returns a copy of the it.
    ///
    /// # Safety
    ///
    /// This method is unsafe because it does not check that the address of the
    /// shell variable is still valid.
    pub unsafe fn get(&self) -> Variable {
        unsafe fn cstr(addr: *const c_char) -> CString {
            CStr::from_ptr(addr).to_owned()
        }

        if self.is_assoc() {
            let items = self.assoc_items().map(|(k, v)| (cstr(k), cstr(v)));
            Variable::Assoc(items.collect())
        } else if self.is_array() {
            let items = self.array_items().map(|(_, s)| cstr(s));
            Variable::Array(items.collect())
        } else {
            Variable::Str(cstr(self.0.as_ref().value))
        }
    }

    /// Returns a reference to the string contained in the shell variable. If
    /// the shell variable contains an array, returns `None`.
    ///
    /// # Safety
    ///
    /// This method is unsafe for two reasons:
    ///
    /// * It does not check that the address of the shell variable is still
    ///   valid.
    /// * The `CStr` reference is wrapping a pointer managed by bash, so its
    ///   lifetime is not guaranteed.
    pub unsafe fn as_str(&self) -> Option<&CStr> {
        let var = self.0.as_ref();
        if var.attributes & (ffi::ATT_ARRAY | ffi::ATT_ASSOC) == 0 {
            Some(CStr::from_ptr(var.value))
        } else {
            None
        }
    }

    /// Returns an iterator over items of the indexed array contained in the
    /// variable.
    ///
    /// # Safety
    ///
    /// This method is unsafe for two reasons:
    ///
    /// * It does not check that the address of the shell variable is still
    ///   valid.
    /// * It does not check that the shell variable contains an indexed array.
    pub unsafe fn array_items(&self) -> impl Iterator<Item = (libc::intmax_t, *const c_char)> + '_ {
        let array = &*(self.0.as_ref().value as *const ffi::Array);
        arrays::ArrayItemsIterator::new(array)
    }

    /// Returns an iterator over items of the associative array contained in
    /// the shell variable.
    ///
    /// # Safety
    ///
    /// This method is unsafe for two reasons:
    ///
    /// * It does not check that the address of the shell variable is still
    ///   valid.
    /// * It does not check that the shell variable contains an associative
    ///   array.
    pub unsafe fn assoc_items(&self) -> impl Iterator<Item = (*const c_char, *const c_char)> + '_ {
        let table = &*(self.0.as_ref().value as *const ffi::HashTable);
        assoc::AssocItemsIterator::new(table)
    }
}
