use proc_macro2::TokenStream;
use quote::quote;

use crate::ParsedScheme;
use crate::scheme_derive::gen_action_discriminant::generate_action_discriminant;
use crate::scheme_derive::gen_action_state_enum::generate_action_state_enum;
use crate::scheme_derive::gen_config_struct::generate_config_struct;

pub fn generate_scheme_derive(parsed: &ParsedScheme) -> syn::Result<TokenStream> {
    let all_generated_items = [
        generate_main_trait(parsed)?,
        generate_config_struct(parsed)?,
        generate_action_discriminant(parsed)?,
        generate_action_state_enum(parsed)?,
    ];
    Ok(quote! {
        #(#all_generated_items)*
    })
}

fn generate_main_trait(parsed: &ParsedScheme) -> syn::Result<TokenStream> {
    let ParsedScheme {
        vis: _,
        scheme_name,
        config_struct_name,
        action_discriminant_name,
        action_state_enum_name,
        basis,
        commands,
    } = parsed;

    let num_variants: syn::Index = commands.len().into();
    let command_names = commands.iter().map(|c| c.command_name).collect::<Vec<_>>();
    let command_names_snake = commands
        .iter()
        .map(|c| &c.command_name_snake)
        .collect::<Vec<_>>();

    Ok(quote! {
        impl TnuaScheme for #scheme_name {
            type Basis = #basis;
            type Config = #config_struct_name;
            type ActionDiscriminant = #action_discriminant_name;
            type ActionStateEnum = #action_state_enum_name;

            const NUM_VARIANTS: usize = #num_variants;

            fn discriminant(&self) -> #action_discriminant_name {
                match self {
                    #(
                        Self::#command_names(_) => #action_discriminant_name::#command_names,
                    )*
                }
            }

            fn into_action_state_variant(self, config: &#config_struct_name) -> #action_state_enum_name {
                match self {
                    #(
                        Self::#command_names(action) => {
                            #action_state_enum_name::#command_names(Tnua2ActionState::new(action, &config.#command_names_snake))
                        }
                    )*
                }
            }

            fn update_in_action_state_enum(
                self,
                action_state_enum: &mut #action_state_enum_name,
            ) -> UpdateInActionStateEnumResult<Self> {
                match (self, action_state_enum) {
                    #(
                        (Self::#command_names(action), #action_state_enum_name::#command_names(state)) => {
                            state.update_input(action);
                            UpdateInActionStateEnumResult::Success
                        }
                    )*
                    #[allow(unreachable_patterns)]
                    (this, _) => UpdateInActionStateEnumResult::WrongVariant(this),
                }
            }

            fn initiation_decision(
                &self,
                config: &#config_struct_name,
                ctx: Tnua2ActionContext<Self::Basis>,
                being_fed_for: &Stopwatch,
            ) -> bevy_tnua::TnuaActionInitiationDirective {
                match self {
                    #(
                        Self::#command_names(action) => action.initiation_decision(&config.#command_names_snake, ctx, being_fed_for),
                    )*
                }
            }
        }
    })
}
