use proc_macro2::TokenStream;
use quote::quote;

use crate::ParsedScheme;

pub fn generate_action_state_enum(parsed: &ParsedScheme) -> syn::Result<TokenStream> {
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
    Ok(quote! {
        #vis enum #action_state_enum_name {
            #(
                #command_names(Tnua2ActionState<#action_types, #basis>),
            )*
        }

        impl Tnua2ActionStateEnum for #action_state_enum_name {
            type Basis = #basis;
            type Discriminant = #action_discriminant_name;

            fn discriminant(&self) -> #action_discriminant_name {
                match self {
                    #(
                        Self::#command_names(_) => #action_discriminant_name::#command_names,
                    )*
                }
            }

            fn interface(
                &self,
            ) -> &dyn bevy_tnua::schemes_action_state::Tnua2ActionStateInterface<Self::Basis> {
                match self {
                    #(
                        Self::#command_names(state) => state,
                    )*
                }
            }

            fn interface_mut(
                &mut self,
            ) -> &mut dyn bevy_tnua::schemes_action_state::Tnua2ActionStateInterface<Self::Basis> {
                match self {
                    #(
                        Self::#command_names(state) => state,
                    )*
                }
            }
        }
    })
}
