//! Module for [`DynamicVariable`].

#![allow(clippy::fn_address_comparisons)]

use super::VariableError;
use crate::ffi::variables as ffi;
use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::mem::MaybeUninit;
use std::os::raw::c_char;
use std::sync::atomic::AtomicBool;
use std::sync::{atomic::Ordering::SeqCst, Mutex, MutexGuard};
use std::{mem, panic};

/// The `DynamicVariable` provides the implementation to create dynamic
/// variables.
///
/// [`get`] is called when the value of the shell variable is required. [`set`]
/// is called when a value is assigned to a variable.
///
/// Use [`variables::bind`] to create a dynamic variable with an instance of a
/// type implementing `DynamicVariable`.
///
/// # Deleting Dynamic Variables
///
/// A dynamic variable can be deleted with [`unset`]. However, the instance
/// bound to it is not dropped, since bash does not notify when a variable is
/// removed.
///
/// If the builtin is removed (`enable -d <name>`), dynamic variables are
/// removed before unloading the shared object.
///
/// # Example
///
/// To create a counter using `DynamicVariable`, first we need to implement a
/// `Counter` type similar to this:
///
/// ```
/// use std::ffi::{CStr, CString};
/// use bash_builtins::error;
/// use bash_builtins::variables::DynamicVariable;
///
/// struct Counter(isize);
///
/// impl DynamicVariable for Counter {
///     fn get(&mut self) -> Option<CString> {
///         let value = CString::new(format!("{}", self.0)).ok();
///         self.0 += 1;
///         value
///     }
///
///     fn set(&mut self, value: &CStr) {
///         self.0 = match value.to_str().map(str::parse) {
///             Ok(Ok(n)) => n,
///             _ => {
///                 error!("invalid value: {:?}", value);
///                 return;
///             }
///         }
///     }
/// }
/// ```
///
/// Then, dynamic variables with any name can be created using a function like
/// this:
///
/// ```
/// # struct Counter(isize);
/// # impl bash_builtins::variables::DynamicVariable for Counter {
/// #   fn get(&mut self) -> Option<std::ffi::CString> { None }
/// #   fn set(&mut self, _: &std::ffi::CStr) {}
/// # }
/// #
/// use bash_builtins::variables::{bind, VariableError};
///
/// fn create_counter(name: &str) -> Result<(), VariableError> {
///     bind(name, Counter(0))
/// }
/// ```
///
/// The [`varcounter` example] implements this functionality:
///
/// ```notrust
/// $ cargo build --release --examples
///
/// $ enable -f target/release/examples/libvarcounter.so varcounter
///
/// $ varcounter FOO BAR
///
/// $ echo $FOO
/// 0
///
/// $ echo $FOO
/// 1
///
/// $ echo $FOO
/// 2
///
/// $ echo $FOO $BAR $BAR
/// 3 0 1
///
/// $ FOO=1000
///
/// $ echo $FOO
/// 1000
///
/// $ echo $FOO
/// 1001
/// ```
///
/// [`varcounter` example]: https://github.com/ayosec/bash-builtins.rs/blob/main/examples/varcounter.rs
/// [`get`]: DynamicVariable::get
/// [`set`]: DynamicVariable::set
/// [`unset`]: https://www.gnu.org/software/bash/manual/html_node/Bourne-Shell-Builtins.html#index-unset
/// [`variables::bind`]: crate::variables::bind
pub trait DynamicVariable {
    /// Returns the value for the shell variable.
    ///
    /// If it returns `None`, the variable will be empty.
    fn get(&mut self) -> Option<CString>;

    /// Called when a string is assigned to the shell variable.
    fn set(&mut self, value: &CStr);
}

pub(super) fn bind_dynvar(
    name: &str,
    dynvar: Box<dyn DynamicVariable>,
) -> Result<(), VariableError> {
    let name = CString::new(name).map_err(|_| VariableError::InvalidName)?;

    unsafe {
        let mut shell_var = ffi::bind_variable(name.as_ptr(), std::ptr::null(), 0);

        if shell_var.is_null() {
            return Err(VariableError::InvalidName);
        }

        (*shell_var).dynamic_value = read_var;
        (*shell_var).assign_func = assign_var;
    }

    global_state().insert(name, dynvar);

    Ok(())
}

/// Track if the global state is initialized.
static STATE_INIT: AtomicBool = AtomicBool::new(false);

type State = HashMap<CString, Box<dyn DynamicVariable>>;

/// Global state to store the instances of `DynamicVariable` with their
/// shell variables.
fn global_state() -> MutexGuard<'static, State> {
    static mut STATE: MaybeUninit<Mutex<State>> = MaybeUninit::uninit();

    if !STATE_INIT.fetch_or(true, SeqCst) {
        unsafe {
            STATE = MaybeUninit::new(Mutex::new(State::default()));
            libc::atexit(remove_all_dynvars);
        }
    }

    match unsafe { (*STATE.as_ptr()).lock() } {
        Ok(l) => l,
        Err(e) => e.into_inner(),
    }
}

/// Unset variables that contains references to function in this crate.
///
/// This function is executed when the shared object is unloaded.
extern "C" fn remove_all_dynvars() {
    let state: State = mem::take(&mut *global_state());
    STATE_INIT.store(false, SeqCst);

    for (varname, _) in state {
        unsafe {
            let shell_var = ffi::find_variable(varname.as_ptr());
            if !shell_var.is_null() && (*shell_var).dynamic_value == read_var {
                ffi::unbind_variable(varname.as_ptr());
            }
        }
    }
}

/// Called by bash when a variable is read.
unsafe extern "C" fn read_var(shell_var: *mut ffi::ShellVar) -> *const ffi::ShellVar {
    if !STATE_INIT.load(SeqCst) {
        return shell_var;
    }

    let result = panic::catch_unwind(|| {
        global_state()
            .get_mut(CStr::from_ptr((*shell_var).name))
            .map(|dynvar| dynvar.get())
    });

    let new_value = match result {
        Ok(Some(v)) => v,

        _ => {
            crate::ffi::internal_error(b"dynamic variable unavailable\0".as_ptr().cast());
            return shell_var;
        }
    };

    libc::free((*shell_var).value.cast());

    // Use the pointer from `CString`. Its memory should be allocated from
    // `malloc`, so it is safe to use the pointer with `free`.
    (*shell_var).value = match new_value {
        Some(v) => v.into_raw(),
        None => libc::calloc(1, 1).cast(),
    };

    shell_var
}

/// Called by bash when a variable is assigned.
unsafe extern "C" fn assign_var(
    shell_var: *mut ffi::ShellVar,
    value: *const c_char,
    _: libc::intmax_t,
    _: *const c_char,
) -> *const ffi::ShellVar {
    if value.is_null() {
        return shell_var;
    }

    let result = panic::catch_unwind(|| {
        global_state()
            .get_mut(CStr::from_ptr((*shell_var).name))
            .map(|dynvar| dynvar.set(CStr::from_ptr(value)))
    });

    if result.is_err() {
        crate::ffi::internal_error(b"dynamic variable unavailable\0".as_ptr().cast());
    }

    shell_var
}
