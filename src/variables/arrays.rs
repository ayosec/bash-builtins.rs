//! Access to array variables.

use super::VariableError;
use crate::ffi::variables as ffi;
use std::convert::TryFrom;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::sync::Once;

/// Change an element of the array contained in the shell variable referenced by
/// `name`.
///
/// `value` is not required to be valid UTF-8, but it can't contain any nul
/// byte.
pub fn array_set<T>(name: &str, index: usize, value: T) -> Result<(), VariableError>
where
    T: AsRef<[u8]>,
{
    let name = CString::new(name).map_err(|_| VariableError::InvalidName)?;
    let value = CString::new(value.as_ref()).map_err(|_| VariableError::InvalidValue)?;

    let res = unsafe {
        if ffi::legal_identifier(name.as_ptr()) == 0 {
            return Err(VariableError::InvalidName);
        }

        ffi::bind_array_variable(name.as_ptr(), index as _, value.as_ptr(), 0)
    };

    if res.is_null() {
        Err(VariableError::InvalidValue)
    } else {
        Ok(())
    }
}

/// Returns a copy of the value corresponding to an element in the array.
pub fn array_get(name: &str, index: usize) -> Option<CString> {
    let index = index as libc::intmax_t;
    let var = super::find_raw(name)?;

    unsafe {
        if !var.is_array() {
            return None;
        }

        let value = var
            .array_items()
            .find(|&(i, _)| i == index)
            .map(|(_, s)| CStr::from_ptr(s).to_owned());

        value
    }
}

/// Get the field to iterate over an array. The offset of this field
/// [was changed][change] in Bash 5.1.
///
/// [change]: https://git.savannah.gnu.org/cgit/bash.git/commit/array.h?id=8868edaf2250e09c4e9a1c75ffe3274f28f38581
fn array_head(array: &ffi::Array) -> *const ffi::ArrayElement {
    static mut IS_5_0: bool = false;
    static INIT: Once = Once::new();

    let is_5_0 = unsafe {
        INIT.call_once(|| {
            let shver = CStr::from_ptr(crate::ffi::shell_version_string());
            IS_5_0 = shver.to_bytes().starts_with(b"5.0.".as_ref());
        });

        IS_5_0
    };

    if is_5_0 {
        array.lastref
    } else {
        array.head
    }
}

/// Iterator to get items in an indexed array.
pub(super) struct ArrayItemsIterator<'a> {
    array: &'a ffi::Array,
    elem: *const ffi::ArrayElement,
}

impl ArrayItemsIterator<'_> {
    pub(super) unsafe fn new(array: &ffi::Array) -> ArrayItemsIterator {
        ArrayItemsIterator {
            array,
            elem: (*array_head(array)).next,
        }
    }
}

impl Iterator for ArrayItemsIterator<'_> {
    type Item = (libc::intmax_t, *const c_char);

    fn size_hint(&self) -> (usize, Option<usize>) {
        match usize::try_from(self.array.num_elements) {
            Ok(n) => (n, Some(n)),
            Err(_) => (0, None),
        }
    }

    fn next(&mut self) -> Option<Self::Item> {
        if self.elem == array_head(self.array) {
            return None;
        }

        let current = unsafe { &(*self.elem) };
        let value = current.value;
        self.elem = current.next;
        Some((current.ind, value))
    }
}
