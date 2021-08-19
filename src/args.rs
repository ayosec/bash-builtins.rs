//! Module to implement the arguments processor.

use crate::{ffi, Error};
use std::ffi::CStr;
use std::marker::PhantomData;
use std::mem;
use std::os::raw::c_int;
use std::str::Utf8Error;

/// This structure provides access to the command-line arguments passed to the
/// builtin.
///
/// An instance for this type is sent as an argument of [`Builtin::call`].
///
/// # Options
///
/// The [`options`] method parses the arguments with the `internal_getopt`
/// function provided by bash, like most builtins, and extract them as values
/// of a type implementing [`BuiltinOptions`].
///
/// If the builtin does not expect any option, call to [`no_options`] before
/// doing anything else.
///
/// # Free Arguments
///
/// The iterators returned by [`raw_arguments`], [`string_arguments`], and
/// [`path_arguments`] yield the argument values.
///
/// If you use [`options`] before any of the `<type>_arguments` methods, the
/// first item of the iteration is the first argument after the last parsed
/// option.
///
/// The example below shows how to extract options and then process free
/// arguments.
///
/// If the builtin accepts options, but no free arguments, call to
/// [`finished`] after [`options`].
///
/// If the builtin does not expect any option or free argument, call to
/// [`finished`] after [`no_options`].
///
/// # Example
///
/// ```
/// use std::io::{stdout, BufWriter, Write};
/// use bash_builtins::{Args, Builtin, BuiltinOptions, Result};
///
/// struct SomeName;
///
/// #[derive(BuiltinOptions)]
/// enum Opt {
///     #[opt = 'f']
///     Foo,
///
///     #[opt = 'b']
///     Bar(i64),
/// }
///
/// impl Builtin for SomeName {
///     fn call(&mut self, args: &mut Args) -> Result<()> {
///         let mut foo = false;
///         let mut bar = 0;
///
///         for option in args.options() {
///             match option? {
///                 Opt::Foo => foo = true,
///                 Opt::Bar(b) => bar = b,
///             }
///         }
///
///         let stdout_handle = stdout();
///         let mut output = BufWriter::new(stdout_handle.lock());
///
///         writeln!(&mut output, "{}, {}", foo, bar)?;
///
///         for path in args.path_arguments() {
///             writeln!(&mut output, "{}", path.display())?;
///         }
///
///         Ok(())
///     }
/// }
/// ```
///
/// [`Builtin::call`]: crate::Builtin::call
/// [`BuiltinOptions`]: bash_builtins_macro::BuiltinOptions
/// [`finished`]: Args::finished
/// [`no_options`]: Args::no_options
/// [`options`]: Args::options
/// [`path_arguments`]: Args::path_arguments
/// [`raw_arguments`]: Args::raw_arguments
/// [`string_arguments`]: Args::string_arguments
pub struct Args {
    word_list: *const ffi::WordList,
    reset_pending: bool,
}

impl Args {
    /// Create a new instance to wrap the `word_list` created by bash.
    ///
    /// # Safety
    ///
    /// The method is unsafe for two reasons:
    ///
    /// * The caller has to provide a valid `word_list` pointer, which
    ///   is sent by bash when the builtin function is invoked.
    ///
    /// * This `Args` instance has to be dropped before returning to bash,
    ///   since the `word_list` address will be reused by bash to something
    ///   else.
    ///
    ///   This requirement is accomplished by using a mutable reference
    ///   (`&mut Args`) when calling to `Builtin::call`. Thus, users can't
    ///   keep a copy of this `Args` instance.
    #[doc(hidden)]
    pub unsafe fn new(word_list: *const ffi::WordList) -> Args {
        Args {
            word_list,
            reset_pending: true,
        }
    }

    /// Returns `true` if there are no more arguments.
    pub fn is_empty(&self) -> bool {
        self.word_list.is_null()
    }

    /// Returns an iterator to parse the command-line arguments with the
    /// `getops` function provided by bash.
    ///
    /// The generic type `T` implements the [`BuiltinOptions`] trait. See its
    /// documentation for details on how to create the parser.
    ///
    /// # Example
    ///
    /// See the [example](struct.Args.html#example) in the [`Args`](self::Args)
    /// documentation.
    ///
    /// [`BuiltinOptions`]: derive.BuiltinOptions.html
    pub fn options<'a, T>(&'a mut self) -> impl Iterator<Item = crate::Result<T>> + 'a
    where
        T: crate::BuiltinOptions<'a> + 'a,
    {
        self.ensure_reset();
        OptionsIterator {
            args: self,
            phantom: PhantomData,
        }
    }

    /// Returns an iterator to get the arguments passed to the builtin.
    ///
    /// Each item is an instance of [`CStr`], and its lifetime is bound to the
    /// [`Args`] instance.
    ///
    /// If this method is called after [`options`](Args::options), the first
    /// item of the iteration is the first argument after the last parsed
    /// option.
    ///
    /// It is recommended to use [`path_arguments`] if the builtin expects file
    /// names as arguments, or [`string_arguments`] if it expects valid UTF-8
    /// strings.
    ///
    /// # Example
    ///
    /// See [`path_arguments`] for an example.
    ///
    /// [`CStr`]: std::ffi::CStr
    /// [`path_arguments`]: Args::path_arguments
    /// [`string_arguments`]: Args::string_arguments
    pub fn raw_arguments(&mut self) -> impl Iterator<Item = &'_ CStr> {
        self.ensure_reset();

        WordListIterator(self)
    }

    /// Like [`raw_arguments`], but items are [`Path`] instances.
    ///
    /// Use this iterator if the builtin arguments are file names.
    ///
    /// # Example
    ///
    /// ```
    /// use std::path::Path;
    /// use bash_builtins::{Args, Builtin, Result};
    ///
    /// struct SomeName;
    ///
    /// impl Builtin for SomeName {
    ///     fn call(&mut self, args: &mut Args) -> Result<()> {
    ///         args.no_options()?;
    ///
    ///         for path in args.path_arguments() {
    ///             process(path)?;
    ///         }
    ///
    ///         Ok(())
    ///     }
    /// }
    ///
    /// fn process(path: &Path) -> Result<()> {
    ///     // …
    /// #   let _ = path;
    ///     Ok(())
    /// }
    /// ```
    ///
    /// [`raw_arguments`]: Args::raw_arguments
    /// [`Path`]: std::path::Path
    #[cfg(unix)]
    #[cfg_attr(docsrs, doc(cfg(unix)))]
    pub fn path_arguments(&mut self) -> impl Iterator<Item = &'_ std::path::Path> {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;
        use std::path::Path;

        self.raw_arguments()
            .map(|a| Path::new(OsStr::from_bytes(a.to_bytes())))
    }

    /// Like [`raw_arguments`], but each item is a string reference if the
    /// argument contains valid UTF-8 data, or a [`Utf8Error`] otherwise.
    ///
    /// [`Utf8Error`]: std::str::Utf8Error
    /// [`raw_arguments`]: Args::raw_arguments
    pub fn string_arguments(&mut self) -> impl Iterator<Item = Result<&'_ str, Utf8Error>> {
        self.raw_arguments()
            .map(|a| std::str::from_utf8(a.to_bytes()))
    }

    /// Returns an error if there are more arguments to be processed.
    ///
    /// If the builtin accepts options but no free arguments, then this method
    /// should be called after [`options`].
    ///
    /// # Example
    ///
    /// ```
    /// # use bash_builtins::{Args, Builtin, BuiltinOptions, Result};
    /// # struct SomeName;
    /// #[derive(BuiltinOptions)]
    /// enum Opt {
    ///     // Builtin options.
    /// #   #[opt = 'a'] A,
    /// }
    ///
    /// impl Builtin for SomeName {
    ///     fn call(&mut self, args: &mut Args) -> Result<()> {
    ///         for option in args.options() {
    ///             match option? {
    ///                 // Parse options.
    /// #               Opt::A => ()
    ///             }
    ///         }
    ///
    ///         // This builtin does not accept free arguments.
    ///         args.finished()?;
    ///
    /// #       fn run_builtin_with_options() -> Result<()> { Ok(()) }
    ///         run_builtin_with_options()?;
    ///
    ///         Ok(())
    ///     }
    /// }
    /// ```
    ///
    /// [`options`]: Args::options
    pub fn finished(&mut self) -> crate::Result<()> {
        if self.word_list.is_null() {
            Ok(())
        } else {
            crate::log::error("too many arguments");
            Err(Error::Usage)
        }
    }

    /// Returns an error if any option is passed as the first argument.
    ///
    /// If the builtin expects no options, then call this method before doing
    /// anything else.
    ///
    /// It uses the `no_options` function provided by bash. The special option
    /// `--help` is handled properly.
    ///
    /// # Example
    ///
    /// ```
    /// // Builtin to convert to uppercase its arguments.
    ///
    /// # use std::io::{self, BufWriter, Write};
    /// # use bash_builtins::{Args, Builtin, Result};
    /// # struct Upcase;
    /// impl Builtin for Upcase {
    ///     fn call(&mut self, args: &mut Args) -> Result<()> {
    ///         args.no_options()?;
    ///
    ///         let stdout_handle = io::stdout();
    ///         let mut output = BufWriter::new(stdout_handle.lock());
    ///
    ///         for argument in args.string_arguments() {
    ///             writeln!(&mut output, "{}", argument?.to_uppercase())?;
    ///         }
    ///
    ///         Ok(())
    ///     }
    /// }
    /// ```
    pub fn no_options(&mut self) -> crate::Result<()> {
        if unsafe { ffi::no_options(self.word_list) } == 0 {
            Ok(())
        } else {
            Err(Error::Usage)
        }
    }

    /// Reset `internal_getopt` state.
    #[inline]
    fn ensure_reset(&mut self) {
        if mem::take(&mut self.reset_pending) {
            unsafe {
                ffi::reset_internal_getopt();
                ffi::list_optarg = std::ptr::null();
                ffi::list_optopt = 0;
            };
        }
    }
}

struct WordListIterator<'a>(&'a mut Args);

impl<'a> Iterator for WordListIterator<'a> {
    type Item = &'a CStr;

    fn next(&mut self) -> Option<Self::Item> {
        if self.0.word_list.is_null() {
            return None;
        }

        let word = unsafe {
            let current = &*self.0.word_list;
            self.0.word_list = current.next;
            CStr::from_ptr((*current.word).word)
        };

        Some(word)
    }
}

/// Trait implemented by the `BuiltinOptions` derive macro.
#[doc(hidden)]
pub trait BuiltinOptions<'a>: Sized {
    fn options() -> &'static [u8];

    fn from_option(opt: c_int, arg: Option<&'a CStr>) -> crate::Result<Self>;
}

struct OptionsIterator<'a, T> {
    args: &'a mut Args,
    phantom: PhantomData<T>,
}

impl<'a, T: BuiltinOptions<'a>> Iterator for OptionsIterator<'a, T> {
    type Item = crate::Result<T>;

    fn next(&mut self) -> Option<Self::Item> {
        let opt =
            unsafe { ffi::internal_getopt(self.args.word_list, T::options().as_ptr().cast()) };

        match opt {
            ffi::GETOPT_EOF => {
                self.args.word_list = unsafe { ffi::loptend };
                None
            }

            ffi::GETOPT_HELP => {
                crate::log::show_help();
                Some(Err(Error::Usage))
            }

            _ => Some(T::from_option(opt, unsafe { Self::optarg() })),
        }
    }
}

impl<'a, T> OptionsIterator<'a, T> {
    unsafe fn optarg() -> Option<&'a CStr> {
        let optarg = ffi::list_optarg;
        if optarg.is_null() {
            None
        } else {
            Some(::std::ffi::CStr::from_ptr(optarg))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ffi::{WordDesc, WordList};

    #[test]
    fn update_word_list_after_arguments() {
        let words = [
            WordDesc {
                word: b"abc\0".as_ptr().cast(),
                flags: 0,
            },
            WordDesc {
                word: b"def\0".as_ptr().cast(),
                flags: 0,
            },
        ];

        let wl1 = WordList {
            word: &words[1],
            next: std::ptr::null(),
        };

        let wl0 = WordList {
            word: &words[0],
            next: &wl1,
        };

        let mut args = unsafe { Args::new(&wl0) };

        let mut string_args = args.string_arguments();
        assert_eq!(string_args.next(), Some(Ok("abc")));
        assert_eq!(string_args.next(), Some(Ok("def")));
        assert_eq!(string_args.next(), None);
        drop(string_args);

        args.finished().unwrap();
    }
}
