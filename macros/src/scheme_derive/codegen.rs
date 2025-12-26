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
    } = parsed;
    Ok(quote! {
        impl TnuaScheme for #scheme_name {
            type Basis = Tnua2BuiltinWalk;
            type Config = #config_struct_name;
            type ActionDiscriminant = #action_discriminant_name;
            type ActionStateEnum = #action_state_enum_name;

            const NUM_VARIANTS: usize = 2;

            fn discriminant(&self) -> #action_discriminant_name {
                match self {
                    Self::Jump(_) => #action_discriminant_name::Jump,
                    Self::Crouch(_) => #action_discriminant_name::Crouch,
                }
            }

            fn into_action_state_variant(self, config: &#config_struct_name) -> #action_state_enum_name {
                match self {
                    Self::Jump(action) => {
                        #action_state_enum_name::Jump(Tnua2ActionState::new(action, &config.jump))
                    }
                    Self::Crouch(action) => {
                        #action_state_enum_name::Crouch(Tnua2ActionState::new(action, &config.crouch))
                    }
                }
            }

            fn update_in_action_state_enum(
                self,
                action_state_enum: &mut #action_state_enum_name,
            ) -> UpdateInActionStateEnumResult<Self> {
                match (self, action_state_enum) {
                    (Self::Jump(action), #action_state_enum_name::Jump(state)) => {
                        state.update_input(action);
                        UpdateInActionStateEnumResult::Success
                    }
                    (Self::Crouch(action), #action_state_enum_name::Crouch(state)) => {
                        state.update_input(action);
                        UpdateInActionStateEnumResult::Success
                    }
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
                    Self::Jump(action) => action.initiation_decision(&config.jump, ctx, being_fed_for),
                    Self::Crouch(action) => action.initiation_decision(&config.crouch, ctx, being_fed_for),
                }
            }
        }
    })
}
