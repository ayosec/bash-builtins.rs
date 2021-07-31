//! Implementation of the `BuiltinOptions` derive macro.

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{quote, ToTokens};
use std::collections::HashSet;
use std::os::raw::c_int;
use syn::spanned::Spanned;

struct VariantOption {
    option: char,
    name: syn::Ident,
    argument_type: Option<syn::Type>,
}

pub(crate) fn macro_impl(args: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(args as syn::DeriveInput);

    // For each variant in the enum we generate a match branch. It checks
    // the result from `getops()`, and converts the argument (if any) to
    // the expected type.

    let variants = match parse_variants(&input) {
        Ok(v) => v,
        Err(e) => return e.into_compile_error().into(),
    };

    let match_variants = variants.iter().map(|variant| {
        let option = variant.option as c_int;
        let var_name = &variant.name;

        let parser = match &variant.argument_type {
            None => {
                quote! { Ok(Self::#var_name) }
            }

            Some(argument_type) => {
                quote! {
                    <#argument_type as ::bash_builtins::convert::FromWordPointer>::extract_value(arg)
                        .map(Self::#var_name)
                }
            }
        };

        quote! {
            #option => { #parser }
        }
    });

    // Build the string argument for `getopt()`.
    //
    // If an option has an argument, `getopt()` expects either a ':' (for
    // required arguments) or a ';' (otherwise) after the option character.
    //
    // To select the extra character we use the constant `OPTSTR_ARGUMENT`,
    // associated to the `FromWordPointer` instance. For `Option<T>` this
    // constant is `;`, and for everything else it is ':'.
    //
    // The associated constant allows us to detect the `Option` type even if
    // the user renames it in their code.
    let options_string = {
        let mut opts = Vec::new();

        for variant in &variants {
            let opt_byte = variant.option as u8;
            opts.push(quote! { #opt_byte });

            if let Some(argument_type) = &variant.argument_type {
                let argument_type = remove_lifetimes(argument_type);

                opts.push(quote! {
                    <#argument_type as ::bash_builtins::convert::FromWordPointer>::OPTSTR_ARGUMENT
                });
            }
        }

        opts.push(quote! { 0 });
        opts
    };

    let options_string_len = options_string.len();

    // Add '__bash_builtin__cstr to the generic parameters.
    //
    // This lifetime is used to bound the `CStr` instances to the `&mut Args`
    // variable received in `Builtin::call`.
    let generics_ext = {
        let mut generics = input.generics.clone();

        let lifetime = syn::Lifetime::new("'__bash_builtin__cstr", Span::call_site());
        generics
            .params
            .push(syn::GenericParam::Lifetime(syn::LifetimeDef::new(lifetime)));

        for lt in input.generics.lifetimes() {
            let where_clause = generics
                .where_clause
                .get_or_insert_with(|| syn::WhereClause {
                    where_token: syn::token::Where {
                        span: Span::call_site(),
                    },
                    predicates: syn::punctuated::Punctuated::new(),
                });

            let pred = format!("'__bash_builtin__cstr: '{}", lt.lifetime.ident);
            where_clause.predicates.push(syn::parse_str(&pred).unwrap());
        }

        generics
    };

    // Generate the parser.

    let (_, ty_generics, _) = input.generics.split_for_impl();
    let (impl_generics, _, where_clause) = generics_ext.split_for_impl();

    let type_name = &input.ident;

    let tokens = quote! {
        impl #impl_generics ::bash_builtins::BuiltinOptions<'__bash_builtin__cstr> for #type_name #ty_generics
        #where_clause
        {
            fn options() -> &'static [u8] {
                const OPTIONS: [u8; #options_string_len] = [ #(#options_string,)* ];
                &OPTIONS[..]
            }

            fn from_option(
                opt: ::std::os::raw::c_int,
                arg: Option<&'__bash_builtin__cstr ::std::ffi::CStr>,
            ) -> ::bash_builtins::Result<Self> {
                match opt {
                    #(#match_variants,)*

                    _ =>  {
                        ::bash_builtins::log::show_usage();
                        return Err(::bash_builtins::Error::Usage);
                    },
                }
            }
        }
    };

    tokens.into()
}

/// Parse the macro input to extract variants data.
fn parse_variants(input: &syn::DeriveInput) -> Result<Vec<VariantOption>, syn::Error> {
    let mut found_options = HashSet::new();

    let data = match &input.data {
        syn::Data::Enum(d) => d,
        _ => return Err(syn::Error::new(input.span(), "expected an enum")),
    };

    data.variants
        .iter()
        .map(|v| parse_variant(v, &mut found_options))
        .collect()
}

fn parse_variant(
    variant: &syn::Variant,
    found_options: &mut HashSet<char>,
) -> Result<VariantOption, syn::Error> {
    let name = variant.ident.clone();

    macro_rules! err {
        ($err:expr) => {
            return Err(syn::Error::new(variant.span(), $err))
        };
    }

    let option = variant
        .attrs
        .iter()
        .find(|attr| attr.path.is_ident("opt"))
        .ok_or_else(|| syn::Error::new(variant.span(), "missing #[opt = '…'] attribute"))
        .and_then(|attr| attr.parse_meta())
        .and_then(|meta| {
            let value = match meta {
                syn::Meta::NameValue(value) => value,
                _ => err!("invalid #[opt] attribute"),
            };

            let opt = match value.lit {
                syn::Lit::Char(lit) => lit.value(),
                _ => err!("#[opt = '…'] requires a character"),
            };

            if !opt.is_ascii_alphanumeric() {
                err!("#[opt] requires an ASCII alphanumeric character");
            }

            if !found_options.insert(opt) {
                err!(format!("duplicated option '{}'", opt));
            }

            Ok(opt)
        })?;

    let argument_type = match &variant.fields {
        syn::Fields::Unit => None,

        syn::Fields::Unnamed(fields) => {
            let mut fields = fields.unnamed.iter();
            let field = fields.next().expect("empty Unnamed fields");

            if fields.next().is_some() {
                err!("Options must have only one argument");
            }

            Some(field.ty.clone())
        }

        syn::Fields::Named(_) => err!("Named fields are not supported"),
    };

    Ok(VariantOption {
        option,
        name,
        argument_type,
    })
}

/// Replace every `'lifetime` with a `'_`.
///
/// The conversion is done using find-and-replace against the text
/// representation of the type. Not very robust, but good enough for
/// the first version.
fn remove_lifetimes(ty: &syn::Type) -> proc_macro2::TokenStream {
    let input = ty.to_token_stream().to_string();

    let mut output = String::with_capacity(input.len());
    let mut in_lifetime = false;

    for ch in input.chars() {
        if in_lifetime {
            in_lifetime = !ch.is_whitespace();
        } else if ch == '\'' {
            output.push_str("'_ ");
            in_lifetime = true;
        } else {
            output.push(ch);
        }
    }

    output.parse().unwrap()
}
