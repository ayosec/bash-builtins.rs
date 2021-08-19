use std::os::raw::{c_char, c_int};

#[repr(C)]
#[doc(hidden)]
pub struct WordList {
    pub(crate) next: *const WordList,
    pub(crate) word: *const WordDesc,
}

#[repr(C)]
#[doc(hidden)]
pub struct WordDesc {
    pub(crate) word: *const c_char,
    pub(crate) flags: c_int,
}

#[doc(hidden)]
pub type BuiltinFunc = extern "C" fn(*const WordList) -> c_int;

#[repr(C)]
pub struct BashBuiltin {
    /// The name that the user types.
    pub name: *const c_char,

    /// The address of the invoked function.
    pub function: BuiltinFunc,

    /// Builtin flags.
    pub flags: c_int,

    /// NULL terminated array of strings.
    pub long_doc: *const *const c_char,

    /// Short version of documentation.
    pub short_doc: *const c_char,

    /// For internal use.
    pub handle: *const c_char,
}

pub(crate) const GETOPT_EOF: c_int = -1;
pub(crate) const GETOPT_HELP: c_int = -99;

extern "C" {
    pub(crate) static mut list_optarg: *const c_char;
    pub(crate) static mut list_opttype: c_int;
    pub(crate) static mut list_optopt: c_int;
    pub(crate) static mut loptend: *const WordList;

    pub(crate) fn internal_getopt(_: *const WordList, _: *const c_char) -> c_int;
    pub(crate) fn reset_internal_getopt();

    pub(crate) fn sh_needarg(_: *const c_char);
    pub(crate) fn no_options(_: *const WordList) -> c_int;

    pub(crate) fn builtin_error(_: *const c_char, ...);
    pub(crate) fn builtin_warning(_: *const c_char, ...);
    pub(crate) fn builtin_usage();
    pub(crate) fn builtin_help();

}

/// Flags for the `BashBuiltin` struct.
pub mod flags {
    use std::os::raw::c_int;

    /// This builtin is enabled.
    pub const BUILTIN_ENABLED: c_int = 0x01;

    /// This has been deleted with enable -d.
    pub const BUILTIN_DELETED: c_int = 0x02;

    /// This builtin is not dynamically loaded.
    pub const STATIC_BUILTIN: c_int = 0x04;

    /// This is a Posix `special' builtin.
    pub const SPECIAL_BUILTIN: c_int = 0x08;

    /// This builtin takes assignment statements.
    pub const ASSIGNMENT_BUILTIN: c_int = 0x10;

    /// This builtins is special in the Posix command search order.
    pub const POSIX_BUILTIN: c_int = 0x20;

    /// This builtin creates local variables .
    pub const LOCALVAR_BUILTIN: c_int = 0x40;
}

/// Exit statuses from builtins
pub mod exit {
    use std::os::raw::c_int;

    /// Builtin failed.
    pub const EXECUTION_FAILURE: c_int = 1;

    /// Builtin succeeded.
    pub const EXECUTION_SUCCESS: c_int = 0;

    /// Shell syntax error.
    pub const EX_BADSYNTAX: c_int = 257;

    /// Syntax error in usage.
    pub const EX_USAGE: c_int = 258;

    /// Redirection failed.
    pub const EX_REDIRFAIL: c_int = 259;

    /// Variable assignment error.
    pub const EX_BADASSIGN: c_int = 260;

    /// Word expansion failed.
    pub const EX_EXPFAIL: c_int = 261;

    /// Fall back to disk command from builtin.
    pub const EX_DISKFALLBACK: c_int = 262;
}

#[cfg(test)]
mod mock_bash_symbols {
    use std::os::raw::{c_char, c_int};

    #[no_mangle]
    extern "C" fn builtin_error(_: *const c_char) {}

    #[no_mangle]
    extern "C" fn builtin_warning(_: *const c_char) {}

    #[no_mangle]
    extern "C" fn builtin_usage() {}

    #[no_mangle]
    extern "C" fn builtin_help() {}

    #[no_mangle]
    extern "C" fn reset_internal_getopt() {}

    #[no_mangle]
    extern "C" fn sh_needarg(_: *const c_char) {}

    #[no_mangle]
    static mut list_opttype: c_int = 0;

    #[no_mangle]
    static mut list_optopt: c_int = 0;

    #[no_mangle]
    static mut list_optarg: *const c_char = std::ptr::null();
}
