use proc_macro2::TokenStream;
use quote::{ToTokens, quote, quote_spanned};

use crate::ParsedScheme;

use super::parsed::ParsedCommand;

pub fn generate_action_state(parsed: &ParsedScheme) -> syn::Result<TokenStream> {
    let ParsedScheme {
        vis,
        action_discriminant_name,
        action_state_enum_name,
        basis,
        commands,
        ..
    } = parsed;
    let command_names = commands.iter().map(|c| c.command_name).collect::<Vec<_>>();
    let action_types = commands.iter().map(|c| c.action_type).collect::<Vec<_>>();
    let payload_types = commands
        .iter()
        .map(|c| c.payloads.iter().map(|p| p.payload_type).collect())
        .collect::<Vec<Vec<_>>>();

    let modify_basis_config_branches = commands.iter().map(gen_modify_basis_config_branch);

    Ok(quote! {
        #[derive(bevy_tnua::serde::Serialize, bevy_tnua::serde::Deserialize)]
        #[serde(crate = "bevy_tnua::serde")]
        #vis enum #action_state_enum_name {
            #(
                #command_names(bevy_tnua::action_state::TnuaActionState<#action_types, #basis>, #(#payload_types,)*),
            )*
        }

        impl bevy_tnua::TnuaActionState for #action_state_enum_name {
            type Basis = #basis;
            type Discriminant = #action_discriminant_name;

            fn discriminant(&self) -> #action_discriminant_name {
                match self {
                    #(
                        Self::#command_names(_, ..) => #action_discriminant_name::#command_names,
                    )*
                }
            }

            fn interface(
                &self,
            ) -> &dyn bevy_tnua::action_state::TnuaActionStateInterface<Self::Basis> {
                match self {
                    #(
                        Self::#command_names(state, ..) => state,
                    )*
                }
            }

            fn interface_mut(
                &mut self,
            ) -> &mut dyn bevy_tnua::action_state::TnuaActionStateInterface<Self::Basis> {
                match self {
                    #(
                        Self::#command_names(state, ..) => state,
                    )*
                }
            }

            fn modify_basis_config(&self, basis_config: &mut <Self::Basis as bevy_tnua::TnuaBasis>::Config) {
                match self {
                    #(#modify_basis_config_branches)*
                }
            }
        }
    })
}

fn gen_modify_basis_config_branch(command: &ParsedCommand) -> TokenStream {
    let command_name = &command.command_name;
    let relevant_payload_binds_or_none = command
        .payloads
        .iter()
        .enumerate()
        .map(|(i, p)| {
            p.modify_basis_config
                .map(|span| syn::Ident::new(&format!("payload_{i}"), span))
        })
        .collect::<Vec<_>>();
    let payload_binds_or_black_holes = relevant_payload_binds_or_none.iter().map(|payload_bind| {
        if let Some(payload_bind) = payload_bind {
            payload_bind.to_token_stream()
        } else {
            syn::PatWild {
                attrs: Default::default(),
                underscore_token: Default::default(),
            }
            .to_token_stream()
        }
    });
    let basis_config_modification_statements =
        relevant_payload_binds_or_none
            .iter()
            .flatten()
            .map(|payload_bind| {
                quote_spanned! {payload_bind.span()=>
                    bevy_tnua::TnuaConfigModifier::modify_config(#payload_bind, basis_config);
                }
            });
    quote! {
        Self::#command_name(_, #(#payload_binds_or_black_holes,)*) => {
            #(#basis_config_modification_statements)*
        }
    }
}
