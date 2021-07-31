use assert_matches::assert_matches;
use bash_builtins::{BuiltinOptions, Error};
use std::ffi::CStr;
use std::os::raw::{c_char, c_int};
use std::sync::atomic::{AtomicUsize, Ordering::SeqCst};

#[cfg(unix)]
#[test]
fn parse_options() {
    #[derive(BuiltinOptions, Debug)]
    enum Opt<'a> {
        #[opt = 'a']
        A,

        #[opt = 'b']
        B(i64),

        #[opt = 'c']
        C(&'a str),

        #[opt = 'd']
        D(&'a std::path::Path),

        #[opt = 'e']
        E(String),

        #[opt = 'f']
        F(Option<&'a str>),

        #[opt = 'g']
        G(std::path::PathBuf),
    }

    assert_eq!(Opt::options(), b"ab:c:d:e:f;g:\0");

    // Valid option

    assert_matches!(
        Opt::from_option(
            b'c' as _,
            Some(&CStr::from_bytes_with_nul(b"0123\0").unwrap())
        ),
        Ok(Opt::C("0123"))
    );

    assert_eq!(SH_NEEDARG_CALLS.swap(0, SeqCst), 0);
    assert_eq!(BUILTIN_USAGE_CALLS.swap(0, SeqCst), 0);

    // Invalid option

    assert_matches!(Opt::from_option(b'z' as _, None), Err(Error::Usage));

    assert_eq!(SH_NEEDARG_CALLS.swap(0, SeqCst), 0);
    assert_eq!(BUILTIN_USAGE_CALLS.swap(0, SeqCst), 1);

    // Missing argument

    assert_matches!(Opt::from_option(b'e' as _, None), Err(Error::Usage));

    assert_eq!(SH_NEEDARG_CALLS.swap(0, SeqCst), 1);
    assert_eq!(BUILTIN_USAGE_CALLS.swap(0, SeqCst), 0);

    // Non-required arguments.

    assert_matches!(Opt::from_option(b'f' as _, None), Ok(Opt::F(None)));

    assert_eq!(SH_NEEDARG_CALLS.swap(0, SeqCst), 0);
    assert_eq!(BUILTIN_USAGE_CALLS.swap(0, SeqCst), 0);

    assert_matches!(
        Opt::from_option(
            b'f' as _,
            Some(&CStr::from_bytes_with_nul(b"abc\0").unwrap())
        ),
        Ok(Opt::F(Some("abc")))
    );

    assert_eq!(SH_NEEDARG_CALLS.swap(0, SeqCst), 0);
    assert_eq!(BUILTIN_USAGE_CALLS.swap(0, SeqCst), 0);
}

// Mock bash functions and static varibles required by the
// `BuiltinOptions` trait.

static SH_NEEDARG_CALLS: AtomicUsize = AtomicUsize::new(0);

static BUILTIN_USAGE_CALLS: AtomicUsize = AtomicUsize::new(0);

#[no_mangle]
extern "C" fn builtin_error(_: *const c_char) {}

#[no_mangle]
extern "C" fn builtin_warning(_: *const c_char) {}

#[no_mangle]
extern "C" fn builtin_usage() {
    BUILTIN_USAGE_CALLS.fetch_add(1, SeqCst);
}

#[no_mangle]
extern "C" fn builtin_help() {}

#[no_mangle]
extern "C" fn sh_needarg(_: *const c_char) {
    SH_NEEDARG_CALLS.fetch_add(1, SeqCst);
}

#[no_mangle]
static mut list_opttype: c_int = 0;

#[no_mangle]
static mut list_optopt: c_int = 0;
