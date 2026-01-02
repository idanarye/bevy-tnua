use proc_macro2::TokenStream;
use quote::quote;

use crate::ParsedScheme;

pub fn generate_action_discriminant(parsed: &ParsedScheme) -> syn::Result<TokenStream> {
    let ParsedScheme {
        vis,
        action_discriminant_name,
        commands,
        ..
    } = parsed;
    let command_names = commands.iter().map(|c| c.command_name).collect::<Vec<_>>();
    let variant_indices = commands
        .iter()
        .enumerate()
        .map(|(i, _)| syn::Index::from(i));

    let (serde_derives, serde_attr) = parsed.gen_serde_clauses_always();

    Ok(quote! {
        #[derive(Debug, Copy, Clone, PartialEq, Eq, #serde_derives)]
        #serde_attr
        #vis enum #action_discriminant_name {
            #(
                #command_names,
            )*
        }

        impl bevy_tnua::TnuaActionDiscriminant for #action_discriminant_name {
            fn variant_idx(&self) -> usize {
                match self {
                    #(
                        Self::#command_names => #variant_indices,
                    )*
                    #[allow(unreachable_patterns)]
                    _ => unreachable!(),
                }
            }
        }
    })
}
