//! Traits for conversions between types.
//!
//! This module implements the trait [`FromWordPointer`] to convert [`CStr`]
//! instances to another type.
//!
//! [`CStr`]: std::ffi::CStr

use std::ffi::CStr;
use std::fmt;
use std::str::{FromStr, Utf8Error};

#[cfg(unix)]
use std::os::unix::ffi::{OsStrExt, OsStringExt};

#[cfg(unix)]
use std::ffi::{OsStr, OsString};

/// Parse a value from a [`CStr`] instance.
///
/// [`CStr`]: std::ffi::CStr
pub trait FromWordPointer<'a>: Sized + 'a {
    /// The type returned in the event of a conversion error.
    type Err: std::fmt::Display;

    /// Parse the value in `s` to return a value of this type.
    fn from_cstr(s: &'a CStr) -> Result<Self, Self::Err>;

    /// Character to build the string for `getopt` to indicate that the option
    /// has an argument.
    ///
    /// This should be overridden only by the impl of `Option`.
    #[doc(hidden)]
    const OPTSTR_ARGUMENT: u8 = b':';

    /// Try to extract the value from a raw argument.
    #[doc(hidden)]
    fn extract_value(arg: Option<&'a CStr>) -> crate::Result<Self> {
        match arg {
            None => {
                crate::log::missing_argument();
                Err(crate::Error::Usage)
            }

            Some(arg) => Self::from_cstr(arg).map_err(|e| {
                crate::error!("{:?}: {}", arg, e);
                crate::Error::Usage
            }),
        }
    }
}

// For non-required arguments.
impl<'a, T: FromWordPointer<'a>> FromWordPointer<'a> for Option<T> {
    const OPTSTR_ARGUMENT: u8 = b';';

    type Err = <T as FromWordPointer<'a>>::Err;

    fn from_cstr(s: &'a CStr) -> Result<Self, Self::Err> {
        <T as FromWordPointer<'a>>::from_cstr(s).map(Some)
    }

    fn extract_value(arg: Option<&'a CStr>) -> crate::Result<Self> {
        match arg {
            None => Ok(None),

            Some(arg) => match <T as FromWordPointer<'a>>::from_cstr(arg) {
                Ok(v) => Ok(Some(v)),

                Err(e) => {
                    crate::error!("{:?}: {}", arg, e);
                    Err(crate::Error::Usage)
                }
            },
        }
    }
}

// Standard types.

impl<'a> FromWordPointer<'a> for &'a str {
    type Err = Utf8Error;

    fn from_cstr(s: &'a CStr) -> Result<Self, Self::Err> {
        s.to_str()
    }
}

impl<'a> FromWordPointer<'a> for String {
    type Err = Utf8Error;

    fn from_cstr(s: &'a CStr) -> Result<Self, Self::Err> {
        s.to_str().map(str::to_owned)
    }
}

#[cfg(unix)]
impl<'a> FromWordPointer<'a> for &'a std::path::Path {
    type Err = std::convert::Infallible;

    fn from_cstr(s: &'a CStr) -> Result<Self, Self::Err> {
        Ok(std::path::Path::new(OsStr::from_bytes(s.to_bytes())))
    }
}

#[cfg(unix)]
impl<'a> FromWordPointer<'a> for std::path::PathBuf {
    type Err = std::convert::Infallible;

    fn from_cstr(s: &'a CStr) -> Result<Self, Self::Err> {
        Ok(Self::from(OsStr::from_bytes(s.to_bytes())))
    }
}

#[cfg(unix)]
impl<'a> FromWordPointer<'a> for &'a OsStr {
    type Err = std::convert::Infallible;

    fn from_cstr(s: &'a CStr) -> Result<Self, Self::Err> {
        Ok(OsStr::from_bytes(s.to_bytes()))
    }
}

#[cfg(unix)]
impl<'a> FromWordPointer<'a> for OsString {
    type Err = std::convert::Infallible;

    fn from_cstr(s: &'a CStr) -> Result<Self, Self::Err> {
        Ok(OsString::from_vec(s.to_bytes().into()))
    }
}

// Types that have to be converted to a string as an intermediate step.

#[doc(hidden)]
pub enum Utf8OrParseError<T> {
    Utf8(Utf8Error),
    Parse(T),
}

impl<T: fmt::Display> fmt::Display for Utf8OrParseError<T> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Utf8OrParseError::Utf8(e) => e.fmt(fmt),
            Utf8OrParseError::Parse(e) => e.fmt(fmt),
        }
    }
}

macro_rules! impl_primitive {
    ($ty:ty) => {
        impl<'a> FromWordPointer<'a> for $ty {
            type Err = Utf8OrParseError<<$ty as FromStr>::Err>;

            fn from_cstr(s: &'a CStr) -> Result<Self, Self::Err> {
                let s = s.to_str().map_err(Utf8OrParseError::Utf8)?;
                <$ty as FromStr>::from_str(s).map_err(Utf8OrParseError::Parse)
            }
        }
    };
}

impl_primitive!(bool);
impl_primitive!(char);
impl_primitive!(f32);
impl_primitive!(f64);
impl_primitive!(i8);
impl_primitive!(i16);
impl_primitive!(i32);
impl_primitive!(i64);
impl_primitive!(i128);
impl_primitive!(isize);
impl_primitive!(u8);
impl_primitive!(u16);
impl_primitive!(u32);
impl_primitive!(u64);
impl_primitive!(u128);
impl_primitive!(usize);
