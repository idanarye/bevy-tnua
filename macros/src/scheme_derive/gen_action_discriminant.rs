use proc_macro2::TokenStream;
use quote::quote;

use crate::ParsedScheme;

pub fn generate_action_discriminant(parsed: &ParsedScheme) -> syn::Result<TokenStream> {
    let ParsedScheme {
        vis,
        action_discriminant_name,
        ..
    } = parsed;
    Ok(quote! {
        #[derive(Debug, Copy, Clone, PartialEq, Eq)]
        #vis enum #action_discriminant_name {
            Jump,
            Crouch,
        }

        impl Tnua2ActionDiscriminant for #action_discriminant_name {
            fn variant_idx(&self) -> usize {
                match self {
                    Self::Jump => 0,
                    Self::Crouch => 1,
                }
            }
        }
    })
}
