use proc_macro2::TokenStream;
use syn::{DeriveInput, parse::Error, parse_macro_input, spanned::Spanned};

use self::scheme_derive::codegen::generate_scheme_derive;
use self::scheme_derive::parsed::ParsedScheme;

mod scheme_derive;
#[allow(unused)]
mod util;

#[proc_macro_derive(TnuaScheme, attributes(scheme))]
pub fn derive_tnua_scheme(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match impl_derive_tnua_scheme(&input) {
        Ok(output) => output.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

fn impl_derive_tnua_scheme(ast: &syn::DeriveInput) -> Result<TokenStream, Error> {
    Ok(match &ast.data {
        syn::Data::Struct(_) => {
            return Err(Error::new(
                ast.span(),
                "TnuaScheme is not supported for structs - only for enums",
            ));
        }
        syn::Data::Enum(data_enum) => {
            let parsed = ParsedScheme::new(ast, data_enum)?;
            generate_scheme_derive(&parsed)?
        }
        syn::Data::Union(_) => {
            return Err(Error::new(
                ast.span(),
                "TnuaScheme is not supported for unions - only for enums",
            ));
        }
    })
    //syn::Data::Struct(_) => Err
    //syn::Fields::Named(fields) => struct_info::StructInfo::new(ast, fields.named.iter())?.derive()?,
    //syn::Fields::Unnamed(_) => return Err(Error::new(ast.span(), "TypedBuilder is not supported for tuple structs")),
    //syn::Fields::Unit => return Err(Error::new(ast.span(), "TypedBuilder is not supported for unit structs")),
    //},
    //syn::Data::Enum(_) => return Err(Error::new(ast.span(), "TypedBuilder is not supported for enums")),
    //syn::Data::Union(_) => return Err(Error::new(ast.span(), "TypedBuilder is not supported for unions")),
}
