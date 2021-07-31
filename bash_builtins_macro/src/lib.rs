//! Macros for the `bash_builtins` crate.

mod metadata_proc_macro;
mod options_derive_macro;

use proc_macro::TokenStream;

#[proc_macro]
#[doc = include_str!("doc/metadata_proc_macro.md")]
pub fn builtin_metadata(args: TokenStream) -> TokenStream {
    metadata_proc_macro::macro_impl(args)
}

#[proc_macro_derive(BuiltinOptions, attributes(opt))]
#[doc = include_str!("doc/options_derive_macro.md")]
pub fn derive_options(args: TokenStream) -> TokenStream {
    options_derive_macro::macro_impl(args)
}
