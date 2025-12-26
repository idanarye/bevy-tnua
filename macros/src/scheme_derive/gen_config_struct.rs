use proc_macro2::TokenStream;
use quote::quote;

use crate::ParsedScheme;

pub fn generate_config_struct(parsed: &ParsedScheme) -> syn::Result<TokenStream> {
    let ParsedScheme {
        vis,
        scheme_name,
        config_struct_name,
        ..
    } = parsed;
    Ok(quote! {
        #[derive(Asset, TypePath)]
        #vis struct #config_struct_name {
            basis: <Tnua2BuiltinWalk as Tnua2Basis>::Config,
            jump: <Tnua2BuiltinJump as Tnua2Action<Tnua2BuiltinWalk>>::Config,
            crouch: <Tnua2BuiltinCrouch as Tnua2Action<Tnua2BuiltinWalk>>::Config,
        }

        impl TnuaSchemeConfig for #config_struct_name {
            type Scheme = #scheme_name;

            fn basis_config(&self) -> &<Tnua2BuiltinWalk as Tnua2Basis>::Config {
                &self.basis
            }
        }
    })
}
