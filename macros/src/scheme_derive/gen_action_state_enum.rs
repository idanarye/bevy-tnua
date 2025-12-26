use proc_macro2::TokenStream;
use quote::quote;

use crate::ParsedScheme;

pub fn generate_action_state_enum(parsed: &ParsedScheme) -> syn::Result<TokenStream> {
    let ParsedScheme {
        vis,
        action_discriminant_name,
        action_state_enum_name,
        ..
    } = parsed;
    Ok(quote! {
        #vis enum #action_state_enum_name {
            Jump(Tnua2ActionState<Tnua2BuiltinJump, Tnua2BuiltinWalk>),
            Crouch(Tnua2ActionState<Tnua2BuiltinCrouch, Tnua2BuiltinWalk>),
        }

        impl Tnua2ActionStateEnum for #action_state_enum_name {
            type Basis = Tnua2BuiltinWalk;
            type Discriminant = #action_discriminant_name;

            fn discriminant(&self) -> #action_discriminant_name {
                match self {
                    Self::Jump(_) => #action_discriminant_name::Jump,
                    Self::Crouch(_) => #action_discriminant_name::Crouch,
                }
            }

            fn interface(
                &self,
            ) -> &dyn bevy_tnua::schemes_action_state::Tnua2ActionStateInterface<Self::Basis> {
                match self {
                    Self::Jump(state) => state,
                    Self::Crouch(state) => state,
                }
            }

            fn interface_mut(
                &mut self,
            ) -> &mut dyn bevy_tnua::schemes_action_state::Tnua2ActionStateInterface<Self::Basis> {
                match self {
                    Self::Jump(state) => state,
                    Self::Crouch(state) => state,
                }
            }
        }
    })
}
