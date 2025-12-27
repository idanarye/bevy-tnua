use proc_macro2::TokenStream;
use quote::quote;

use crate::ParsedScheme;

pub fn generate_config_struct(parsed: &ParsedScheme) -> syn::Result<TokenStream> {
    let ParsedScheme {
        vis,
        scheme_name,
        config_struct_name,
        basis,
        commands,
        ..
    } = parsed;
    let command_names_snake = commands
        .iter()
        .map(|c| &c.command_name_snake)
        .collect::<Vec<_>>();
    let action_types = commands.iter().map(|c| c.action_type).collect::<Vec<_>>();
    Ok(quote! {
        #[derive(Asset, TypePath)]
        #vis struct #config_struct_name {
            basis: <#basis as Tnua2Basis>::Config,
            #(
                #command_names_snake: <#action_types as Tnua2Action<#basis>>::Config,
            )*
        }

        impl TnuaSchemeConfig for #config_struct_name {
            type Scheme = #scheme_name;

            fn basis_config(&self) -> &<#basis as Tnua2Basis>::Config {
                &self.basis
            }
        }
    })
}
