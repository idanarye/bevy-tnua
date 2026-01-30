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
        config_ext,
        ..
    } = parsed;
    let command_names_snake = commands
        .iter()
        .map(|c| &c.command_name_snake)
        .collect::<Vec<_>>();
    let action_types = commands.iter().map(|c| c.action_type).collect::<Vec<_>>();
    let config_ext = config_ext.iter();
    Ok(quote! {
        #[derive(bevy::prelude::Asset, bevy::prelude::TypePath, bevy_tnua::serde::Serialize, bevy_tnua::serde::Deserialize)]
        #[serde(crate = "bevy_tnua::serde")]
        #vis struct #config_struct_name {
            #vis basis: <#basis as bevy_tnua::TnuaBasis>::Config,
            #(
                #vis #command_names_snake: <#action_types as bevy_tnua::TnuaAction<#basis>>::Config,
            )*
            #(
                #vis ext: #config_ext,
            )*
        }

        impl bevy_tnua::TnuaSchemeConfig for #config_struct_name {
            type Scheme = #scheme_name;

            fn basis_config(&self) -> &<#basis as bevy_tnua::TnuaBasis>::Config {
                &self.basis
            }
        }
    })
}
