use std::collections::HashMap;

use atterate::StaticError;
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

    let num_variants: syn::Index = commands
        .iter()
        .filter(|c| c.same_trigger.is_none())
        .count()
        .into();

    let command_names = commands.iter().map(|c| c.command_name).collect::<Vec<_>>();

    let mut same_trigger_targets: HashMap<&syn::Ident, Option<usize>> = commands
        .iter()
        .filter_map(|c| Some((c.same_trigger.as_ref()?, None)))
        .collect();
    if !same_trigger_targets.is_empty() {
        for (i, cmd) in commands
            .iter()
            .filter(|c| c.same_trigger.is_none())
            .enumerate()
        {
            if let Some(target) = same_trigger_targets.get_mut(cmd.command_name) {
                *target = Some(i);
            }
        }
    }
    let same_trigger_targets: HashMap<&syn::Ident, usize> = same_trigger_targets
        .into_iter()
        .map(|(k, v)| Ok((k, v.ok_or(StaticError::Spanned(k, "Command variant does not exist in scheme, or exists but has a `same_trigger` of its own"))?)))
        .collect::<Result<_, syn::Error>>()?;

    let mut next_index = 0..;
    let variant_indices = commands.iter().map(|command| {
        let variant_index = if let Some(same_as) = command.same_trigger.as_ref() {
            same_trigger_targets[same_as]
        } else {
            next_index.next().expect("endless range should not end")
        };
        syn::Index::from(variant_index)
    });

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
            const NUM_FEED_STATUS_SLOTS: usize = #num_variants;

            fn feed_status_slot(&self) -> usize {
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
