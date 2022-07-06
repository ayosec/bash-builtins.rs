//! Access to associative array variables.

use super::VariableError;
use crate::ffi::variables as ffi;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

/// Returns a pointer to a C-string with the contents of `bytes`. `bytes` can't
/// contain the nul byte.
///
/// This pointer is expected to be freed by Bash, so its memory is allocated
/// with libc.
fn cstrdup<T: AsRef<[u8]>>(bytes: T) -> Result<*const c_char, VariableError> {
    let bytes = bytes.as_ref();

    if bytes.contains(&b'\0') {
        return Err(VariableError::InvalidValue);
    }

    unsafe { Ok(libc::strndup(bytes.as_ptr().cast(), bytes.len())) }
}

/// Change an element of the associative array contained in the shell variable
/// referenced by `name`.
///
/// `value` is not required to be valid UTF-8, but it can't contain any nul
/// byte.
pub fn assoc_set<T0, T1>(name: &str, key: T0, value: T1) -> Result<(), VariableError>
where
    T0: AsRef<[u8]>,
    T1: AsRef<[u8]>,
{
    let name = CString::new(name).map_err(|_| VariableError::InvalidName)?;
    let key = cstrdup(key)?;
    let value = cstrdup(value)?;

    let res = unsafe {
        if ffi::legal_identifier(name.as_ptr()) == 0 {
            return Err(VariableError::InvalidName);
        }

        let mut shell_var = ffi::find_variable(name.as_ptr());

        if shell_var.is_null() {
            shell_var = ffi::make_new_assoc_variable(name.as_ptr());
        } else if (*shell_var).attributes & ffi::ATT_ASSOC == 0 {
            return Err(VariableError::NotAssocArray);
        }

        ffi::bind_assoc_variable(shell_var, name.as_ptr(), key, value, 0)
    };

    if res.is_null() {
        Err(VariableError::InvalidValue)
    } else {
        Ok(())
    }
}

/// Returns a copy of the value corresponding to a key in an associative array.
pub fn assoc_get<T: AsRef<[u8]>>(name: &str, key: T) -> Option<CString> {
    let key = key.as_ref();
    let var = super::find_raw(name)?;

    unsafe {
        if !var.is_assoc() {
            return None;
        }

        let value = var
            .assoc_items()
            .find(|&(k, _)| libc::strncmp(key.as_ptr().cast(), k, key.len()) == 0)
            .map(|(_, s)| CStr::from_ptr(s).to_owned());

        value
    }
}

/// Iterator to get items in an associative array.
pub(super) struct AssocItemsIterator<'a> {
    table: &'a ffi::HashTable,
    num_bucket: isize,
    current_bucket_item: Option<*const ffi::BucketContents>,
}

impl AssocItemsIterator<'_> {
    pub(super) unsafe fn new(table: &ffi::HashTable) -> AssocItemsIterator {
        AssocItemsIterator {
            table,
            num_bucket: 0,
            current_bucket_item: None,
        }
    }
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
