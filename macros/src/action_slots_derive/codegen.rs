use proc_macro2::TokenStream;
use quote::quote;

use crate::ParsedActionSlots;
use crate::action_slots_derive::parsed::ParsedSlot;

pub fn generate_action_slots_derive(parsed: &ParsedActionSlots) -> syn::Result<TokenStream> {
    let ParsedActionSlots {
        slots_name,
        scheme,
        ending_actions,
        slots,
    } = parsed;

    // This type is a mouthful, and gets repeated alot
    let discriminant = quote!(<Self::Scheme as bevy_tnua::TnuaScheme>::ActionDiscriminant);

    let counted_actions = slots.iter().flat_map(|slot| slot.actions.iter());

    let get_mut_branches = slots.iter().map(|slot| {
        let ParsedSlot {
            counter_name,
            actions,
        } = slot;
        quote! {
            #(#discriminant::#actions)|* => Some(&mut self.#counter_name),
        }
    });

    let get_branches = slots.iter().map(|slot| {
        let ParsedSlot {
            counter_name,
            actions,
        } = slot;
        quote! {
            #(#discriminant::#actions)|* => Some(self.#counter_name),
        }
    });

    Ok(quote! {
        impl bevy_tnua::control_helpers::TnuaActionSlots for #slots_name {
            type Scheme = #scheme;

            fn rule_for(action: #discriminant) -> bevy_tnua::control_helpers::TnuaActionCountingActionRule {
                match action {
                    #(
                        #discriminant::#counted_actions => {
                            bevy_tnua::control_helpers::TnuaActionCountingActionRule::Counted
                        }
                    )*
                    #(
                        #discriminant::#ending_actions => {
                            bevy_tnua::control_helpers::TnuaActionCountingActionRule::EndingCount
                        }
                    )*
                    #[allow(unreachable_patterns)]
                    _ => bevy_tnua::control_helpers::TnuaActionCountingActionRule::Uncounted,
                }
            }

            fn get_mut(&mut self, action: #discriminant) -> Option<&mut usize> {
                match action {
                    #(#get_mut_branches)*
                    #[allow(unreachable_patterns)]
                    _ => None,
                }
            }

            fn get(&self, action: #discriminant) -> Option<usize> {
                match action {
                    #(#get_branches)*
                    #[allow(unreachable_patterns)]
                    _ => None,
                }
            }
        }
    })
}
