//! Access to array variables.

use std::convert::TryFrom;
use std::ffi::{c_int, c_void, CStr, CString};

use super::VariableError;
use crate::ffi::variables as ffi;

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
    let var = super::find_raw(name)?;

    let mut result = None;

    unsafe {
        if !var.is_array() {
            return None;
        }

        #[repr(C)]
        struct Data {
            result: *mut Option<CString>,
            index: usize,
        }

        unsafe extern "C" fn collect(elem: *mut ffi::ArrayElement, data: *mut c_void) -> c_int {
            let data = &mut *data.cast::<Data>();

            if usize::try_from((*elem).ind) == Ok(data.index) {
                data.result
                    .write(Some(CStr::from_ptr((*elem).value).to_owned()));
                -1
            } else {
                1
            }
        }

        let data = Data {
            result: &mut result,
            index,
        };

        ffi::array_walk(
            (*var.0.as_ptr()).value,
            collect,
            &data as *const Data as *const c_void,
        );
    }

    result
}

pub(crate) unsafe fn array_items(shell_var: *const ffi::ShellVar) -> Vec<(i64, CString)> {
    let array: ffi::ArrayPtr = unsafe { (*shell_var).value.cast() };
    let mut vec = Vec::new();

    #[repr(C)]
    struct Data(*mut Vec<(i64, CString)>);

    unsafe extern "C" fn collect(elem: *mut ffi::ArrayElement, data: *mut c_void) -> c_int {
        let vec = &mut *(*data.cast::<Data>()).0;

        let index = (*elem).ind;
        let value = CStr::from_ptr((*elem).value).to_owned();

        vec.push((index, value));

        1
    }

    ffi::array_walk(
        array,
        collect,
        &Data(&mut vec) as *const Data as *const c_void,
    );

    vec
}
