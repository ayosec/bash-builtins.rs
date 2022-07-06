//! Access to array variables.

use super::VariableError;
use crate::ffi::variables as ffi;
use std::convert::TryFrom;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

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

/// Iterator to get items in an indexed array.
pub(super) struct ArrayItemsIterator<'a> {
    array: &'a ffi::Array,
    elem: *const ffi::ArrayElement,
}

impl ArrayItemsIterator<'_> {
    pub(super) unsafe fn new(array: &ffi::Array) -> ArrayItemsIterator {
        ArrayItemsIterator {
            array,
            elem: (*array.head).next,
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
        if self.elem == self.array.head {
            return None;
        }

        let current = unsafe { &(*self.elem) };
        let value = current.value;
        self.elem = current.next;
        Some((current.ind, value))
    }
}
