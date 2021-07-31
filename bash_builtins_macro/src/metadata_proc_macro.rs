//! Implementation of the `builtin_metadata!()` macro.

use proc_macro::TokenStream;
use quote::{format_ident, quote};

// The macro generates the following items:
//
// * `<NAME>_struct`
//
//     A global variable to initialize the fields required by the
//     [`struct builtin`].
//
//     This symbol is loaded by bash to get the builtin metadata.
//
// * `__bash_builtin__state_<NAME>`
//
//     A reference to a global variable used to store the builtin instance.
//
// * `__bash_builtin__state_init_<NAME>`
//
//     A global variable to track if the state has been initialized.
//
// * `<NAME>_builtin_load`
//
//     A function invoked by bash to initialize the builtin.
//
// * `<NAME>_builtin_unload`
//
//     A function invoked by bash when the builtin is removed
//     (`enable -d <NAME>` in the prompt).
//
// * `__bash_builtin__func_<NAME>`
//
//     The function invoked by bash when the builtin is typed in the prompt.
//
// [`struct builtin`]: https://git.savannah.gnu.org/cgit/bash.git/tree/builtins.h?h=bash-5.1#n52

pub(crate) fn macro_impl(args: TokenStream) -> TokenStream {
    let args = syn::parse_macro_input!(args as args::MacroArgs);

    let name = args
        .name
        .as_ref()
        .expect("`name` argument is required")
        .value();

    // Symbols expected by bash.
    let struct_bash_symbol = format_ident!("{}_struct", name);
    let load_bash_symbol = format_ident!("{}_builtin_load", name);
    let unload_bash_symbol = format_ident!("{}_builtin_unload", name);

    // Internal items.
    let global_state = format_ident!("__bash_builtin__state_{}", name);
    let global_state_init = format_ident!("__bash_builtin__state_init_{}", name);
    let builtin_func = format_ident!("__bash_builtin__func_{}", name);

    let empty_str = quote! { [0].as_ptr() };

    // Builtin documentation.
    let short_doc = match args.short_doc.as_ref() {
        Some(s) => {
            let cs = strings::to_cstr(&s.value());
            quote! { #cs }
        }

        None => empty_str.clone(),
    };

    let long_doc = match args.long_doc.as_ref() {
        Some(s) => strings::to_c_arrays(&s.value()),

        None => {
            quote! {
                [ #empty_str, ::std::ptr::null() ].as_ptr()
            }
        }
    };

    // Path to the constructor.
    let constructor = match (args.create.as_ref(), args.try_create.as_ref()) {
        (Some(path), None) => quote! { Box::new(#path()) },

        (None, Some(path)) => quote! {
            match #path() {
                Ok(s) => Box::new(s),

                Err(e) => {
                    use ::std::io::{stderr, Write};
                    let _ = writeln!(stderr(), concat!(#name, ": error: {}"), e);
                    return 0;
                },
            }
        },
        _ => panic!("one of `create` or `try_create` is required"),
    };

    let struct_type = quote! { ::bash_builtins::ffi::BashBuiltin };
    let name_field_value = strings::to_cstr(&name);

    // Acquire lock to store builtin state.
    let store_access = quote! {
        match #global_state().lock() {
            Ok(lock) => lock,

            _ => {
                ::bash_builtins::log::error("invalid internal state");
                return RETVAL_ERROR;
            }
        }
    };

    let state_type = quote! {
        ::std::sync::Mutex<
            ::std::option::Option<
                ::std::boxed::Box<dyn ::bash_builtins::Builtin>>>
    };

    // Final code.
    let tokens = quote! {
        #[no_mangle]
        #[doc(hidden)]
        pub static mut #struct_bash_symbol: #struct_type = #struct_type {
            name: #name_field_value,
            function: #builtin_func,
            flags: ::bash_builtins::ffi::flags::BUILTIN_ENABLED,
            short_doc: #short_doc,
            long_doc: #long_doc,
            handle: ::std::ptr::null()
        };

        #[doc(hidden)]
        static #global_state_init: ::std::sync::atomic::AtomicBool =
            ::std::sync::atomic::AtomicBool::new(false);

        fn #global_state() -> &'static #state_type {
            use ::std::mem::MaybeUninit;
            use ::std::sync::{Mutex, Once, atomic::Ordering::SeqCst};

            static mut STATE: MaybeUninit<#state_type> = MaybeUninit::uninit();

            if #global_state_init.fetch_or(true, SeqCst) == false {
                unsafe {
                    STATE = MaybeUninit::new(Mutex::new(None));
                }
            }

            unsafe { &*STATE.as_ptr() }
        }

        #[no_mangle]
        #[doc(hidden)]
        pub extern "C" fn #load_bash_symbol(
            name: *const ::std::os::raw::c_char
        ) -> ::std::os::raw::c_int {
            const RETVAL_ERROR: ::std::os::raw::c_int = 0;
            ::std::panic::catch_unwind(|| {
                let mut lock = #store_access;
                let state = #constructor as Box<dyn ::bash_builtins::Builtin>;
                *lock = Some(state);
                1
            }).unwrap_or(RETVAL_ERROR)
        }

        #[no_mangle]
        #[doc(hidden)]
        pub extern "C" fn #unload_bash_symbol(
            name: *const ::std::os::raw::c_char
        ) {
            let _ = ::std::panic::catch_unwind(|| {
                #global_state_init.store(false, ::std::sync::atomic::Ordering::SeqCst);

                match #global_state().lock() {
                    Ok(mut lock) => { *lock = None },

                    Err(poison) => {
                        // If the mutex is poisoned we don't trust the state of
                        // the builtin. In this case the old value is leaked.
                        let old_state = poison.into_inner().take();
                        ::std::mem::forget(old_state);
                    },
                };
            });
        }

        extern "C" fn #builtin_func(
            word_list: *const ::bash_builtins::ffi::WordList
        ) -> ::std::os::raw::c_int {
            const RETVAL_ERROR: ::std::os::raw::c_int = 1;

            ::std::panic::catch_unwind(|| {
                let mut lock = #store_access;
                let mut args = unsafe { ::bash_builtins::Args::new(word_list) };
                match (&mut *lock) {
                    Some(state) => {
                        match state.call(&mut args) {
                            Ok(()) => 0,

                            Err(e) => {
                                if e.print_on_return() {
                                    ::bash_builtins::error!("{}", e);
                                }

                                e.exit_code()
                            }
                        }
                    }

                    None => {
                        ::bash_builtins::log::error("builtin not initialized");
                        RETVAL_ERROR
                    }
                }
            }).unwrap_or(101) // exit code on panic!(), from Rust
        }
    };

    tokens.into()
}

mod args {
    //! Macro arguments.

    use syn::parse::{Parse, ParseStream, Result};
    use syn::{ExprPath, LitStr, Token};

    #[derive(Default)]
    pub(crate) struct MacroArgs {
        pub(crate) name: Option<LitStr>,
        pub(crate) create: Option<ExprPath>,
        pub(crate) try_create: Option<ExprPath>,
        pub(crate) short_doc: Option<LitStr>,
        pub(crate) long_doc: Option<LitStr>,
    }

    mod kw {
        syn::custom_keyword!(name);
        syn::custom_keyword!(create);
        syn::custom_keyword!(try_create);
        syn::custom_keyword!(short_doc);
        syn::custom_keyword!(long_doc);
    }

    impl Parse for MacroArgs {
        fn parse(input: ParseStream) -> Result<Self> {
            let mut args = MacroArgs::default();

            while !input.is_empty() {
                let lookahead = input.lookahead1();

                macro_rules! args {
                    ($key:ident $($keys:ident)*) => {
                        if lookahead.peek(kw::$key) {
                            input.parse::<kw::$key>()?;
                            input.parse::<Token![=]>()?;
                            args.$key = Some(input.parse()?);
                        } else {
                            args!($($keys)*);
                        }
                    };

                    () => {
                        return Err(lookahead.error());
                    }
                }

                args!(name create try_create short_doc long_doc);

                if !input.is_empty() {
                    input.parse::<Token![,]>()?;
                }
            }

            Ok(args)
        }
    }
}

mod strings {
    //! Helper functions to manage string values.

    use quote::quote;

    /// Convert a string literal to its C-string equivalent.
    ///
    /// The value can't contain a NULL character.
    pub(crate) fn to_cstr(text: &str) -> proc_macro2::TokenStream {
        if text.contains('\0') {
            panic!("{:?} must not contain nul bytes", text);
        }

        quote! { concat!(#text, "\0").as_ptr().cast() }
    }

    /// Convert a string to an array of C strings.
    ///
    /// The left margin used to indent the text in the source code is removed.
    pub(crate) fn to_c_arrays(text: &str) -> proc_macro2::TokenStream {
        let text = text.trim_start_matches('\n').trim_end();

        let left_margin = text
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| line.chars().take_while(|c| *c == ' ').count())
            .min()
            .unwrap_or_default();

        let lines = text
            .lines()
            .map(|line| to_cstr(line.get(left_margin..).unwrap_or_default()));

        quote! {
            (&[
                #(#lines,)*
                ::std::ptr::null()
            ]).as_ptr()
        }
    }
}
