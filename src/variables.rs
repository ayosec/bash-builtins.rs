//! This module contains functions to get, set, or unset shell variables.
//!
//! [`set_value`] and [`unset`] are used to modify shell variables.
//!
//! The `find` functions are used to access to the value contained in existing
//! shell variables. [`find_raw`] provides access to the raw pointer owned by
//! bash, and both [`find`] and [`find_as_string`] provides a safe interface to
//! such value.
//!
//! ## Example
//!
//! The following example uses the variable `$SOMENAME_LIMIT` to set the
//! configuration value for the builtin. If it is not present, or its value is
//! not a valid `usize`, it uses a default value
//!
//! ```
//! use bash_builtins::variables;
//!
//! const DEFAULT_LIMIT: usize = 1024;
//!
//! const VAR_LIMIT: &str = "SOMENAME_LIMIT";
//!
//! fn get_limit() -> usize {
//!     variables::find_as_string(VAR_LIMIT)
//!         .as_ref()
//!         .and_then(|v| v.to_str().ok())
//!         .and_then(|v| v.parse().ok())
//!         .unwrap_or(DEFAULT_LIMIT)
//! }
//! ```
//!

use crate::ffi::variables as ffi;
use std::collections::HashMap;
use std::convert::TryFrom;
use std::ffi::{CStr, CString};
use std::fmt;
use std::os::raw::c_char;

/// Returns a string with the value of the variable `name`.
///
/// If the variable does not exist, or its value is an array, the function
/// returns `None`.
pub fn find_as_string(name: &str) -> Option<CString> {
    unsafe { find_raw(name).and_then(|var| var.as_str().map(|cstr| cstr.to_owned())) }
}

/// Returns a copy of the value of the shell variable referenced by `name`.
///
/// If the variable does not exist, it returns `None`.
///
/// Use [`find_as_string`] if you want to ignore arrays.
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
    let shell_var = unsafe { ffi::find_variable(name.as_ptr()) };

    if shell_var.is_null() {
        return None;
    }

    Some(RawVariable(shell_var))
}

/// Sets the value of the shell variable referenced by `name`.
///
/// `value` is not required to be valid UTF-8, but it can't contain any nul
/// byte.
pub fn set_value<T>(name: &str, value: T) -> Result<(), SetValueError>
where
    T: AsRef<[u8]>,
{
    let name = CString::new(name).map_err(|_| SetValueError::InvalidName)?;
    let value = CString::new(value.as_ref()).map_err(|_| SetValueError::InvalidValue)?;

    let res = unsafe {
        if ffi::legal_identifier(name.as_ptr()) == 0 {
            return Err(SetValueError::InvalidName);
        }

        ffi::bind_variable(name.as_ptr(), value.as_ptr(), 0)
    };

    if res.is_null() {
        Err(SetValueError::InvalidValue)
    } else {
        Ok(())
    }
}

/// Unset the shell variable referenced by `name`.
///
/// Returns `true` if the variable is removed.
pub fn unset(name: &str) -> bool {
    let name = match CString::new(name) {
        Ok(s) => s,
        Err(_) => return false,
    };

    unsafe { ffi::unbind_variable(name.as_ptr()) == 0 }
}

/// An error from [`set_value`].
#[derive(Debug)]
pub enum SetValueError {
    InvalidName,
    InvalidValue,
}

impl fmt::Display for SetValueError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match self {
            SetValueError::InvalidName => fmt.write_str("invalid variable name"),
            SetValueError::InvalidValue => fmt.write_str("invalid variable value"),
        }
    }
}

impl std::error::Error for SetValueError {}

/// Contains the value of a variable.
///
/// Use [`find`] or [`RawVariable::get`] to get the value of a variable.
#[derive(Debug)]
pub enum Variable {
    /// A single string.
    Str(CString),

    /// An indexed array.
    Array(Vec<CString>),

    /// An associative array.
    Assoc(HashMap<CString, CString>),
}

/// Raw reference to a shell variable.
///
/// Every method is unsafe because this type contains a raw pointer to an
/// address owned by bash.
///
/// Whenever possible, use [`find`] or [`find_as_string`] functions to get the
/// value of a variable.
#[derive(Debug)]
pub struct RawVariable(*const ffi::ShellVar);

impl RawVariable {
    /// Returns `true` if the variable contains an indexed array.
    ///
    /// # Safety
    ///
    /// This method is unsafe because it does not check that the address of the
    /// shell variable is still valid.
    pub unsafe fn is_array(&self) -> bool {
        (*self.0).attributes & ffi::ATT_ARRAY != 0
    }

    /// Returns `true` if the variable contains an associative array.
    ///
    /// # Safety
    ///
    /// This method is unsafe because it does not check that the address of the
    /// shell variable is still valid.
    pub unsafe fn is_assoc(&self) -> bool {
        (*self.0).attributes & ffi::ATT_ASSOC != 0
    }

    /// Extracts the contents of the variable, and returns a copy of the it.
    ///
    /// # Safety
    ///
    /// This method is unsafe because it does not check that the address of the
    /// shell variable is still valid.
    pub unsafe fn get(&self) -> Variable {
        if self.is_assoc() {
            let items = self
                .assoc_items()
                .map(|(k, v)| unsafe { (read_ptr(k), read_ptr(v)) })
                .collect();
            Variable::Assoc(items)
        } else if self.is_array() {
            let items = self.array_items().map(|p| unsafe { read_ptr(p) }).collect();
            Variable::Array(items)
        } else {
            Variable::Str(read_ptr((*self.0).value))
        }
    }

    /// Returns a reference to the string contained in the variable. If the
    /// variable contains an array, returns `None`.
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
        if (*self.0).attributes & (ffi::ATT_ARRAY | ffi::ATT_ASSOC) == 0 {
            Some(CStr::from_ptr((*self.0).value))
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
    /// * It does not check that the variable contains an indexed array.
    pub unsafe fn array_items(&self) -> impl Iterator<Item = *const c_char> + '_ {
        let array = &*((*self.0).value as *const ffi::Array);
        ArrayItemsIterator {
            array,
            elem: (*array.lastref).next,
        }
    }

    /// Returns an iterator over items of the associative array contained in
    /// the variable.
    ///
    /// # Safety
    ///
    /// This method is unsafe for two reasons:
    ///
    /// * It does not check that the address of the shell variable is still
    ///   valid.
    /// * It does not check that the variable contains an associative array.
    pub unsafe fn assoc_items(&self) -> impl Iterator<Item = (*const c_char, *const c_char)> + '_ {
        let table = &*((*self.0).value as *const ffi::HashTable);
        AssocItemsIterator {
            table,
            num_bucket: 0,
            current_bucket_item: None,
        }
    }
}

/// Iterator to get items in an indexed array.
struct ArrayItemsIterator<'a> {
    array: &'a ffi::Array,
    elem: *const ffi::ArrayElement,
}

impl Iterator for ArrayItemsIterator<'_> {
    type Item = *const c_char;

    fn size_hint(&self) -> (usize, Option<usize>) {
        match usize::try_from(self.array.num_elements) {
            Ok(n) => (n, Some(n)),
            Err(_) => (0, None),
        }
    }

    fn next(&mut self) -> Option<Self::Item> {
        if self.elem == self.array.lastref {
            return None;
        }

        let current = unsafe { &(*self.elem) };
        let value = current.value;
        self.elem = current.next;
        Some(value)
    }
}

/// Iterator to get items in an associative array.
struct AssocItemsIterator<'a> {
    table: &'a ffi::HashTable,
    num_bucket: isize,
    current_bucket_item: Option<*const ffi::BucketContents>,
}

impl Iterator for AssocItemsIterator<'_> {
    type Item = (*const c_char, *const c_char);

    fn next(&mut self) -> Option<Self::Item> {
        while self.num_bucket < self.table.nbuckets as isize {
            let bucket = self
                .current_bucket_item
                .take()
                .unwrap_or_else(|| unsafe { *self.table.bucket_array.offset(self.num_bucket) });

            if !bucket.is_null() {
                unsafe {
                    let bucket = &*bucket;
                    let item = ((*bucket).key, (*bucket).data);
                    self.current_bucket_item = Some((*bucket).next);
                    return Some(item);
                }
            }

            self.num_bucket += 1;
        }

        None
    }
}

unsafe fn read_ptr(addr: *const c_char) -> CString {
    CStr::from_ptr(addr).to_owned()
}
