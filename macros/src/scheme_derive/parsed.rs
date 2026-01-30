use convert_case::{Case, Casing};
use quote::{quote, quote_spanned};
use syn::Ident;

use crate::util::{AttrArg, StaticError, ident_with_suffix};

#[derive(Debug)]
pub struct ParsedScheme<'a> {
    pub vis: &'a syn::Visibility,
    pub scheme_name: &'a syn::Ident,
    pub config_struct_name: syn::Ident,
    pub action_discriminant_name: syn::Ident,
    pub action_state_enum_name: syn::Ident,
    pub basis: syn::Type,
    pub serde: Option<proc_macro2::Span>,
    pub config_ext: Option<syn::Type>,
    pub commands: Vec<ParsedCommand<'a>>,
}

impl<'a> ParsedScheme<'a> {
    pub fn new(ast: &'a syn::DeriveInput, data_enum: &'a syn::DataEnum) -> syn::Result<Self> {
        let attr_on_enum = AttrOnEnum::new(ast)?;
        Ok(Self {
            vis: &ast.vis,
            scheme_name: &ast.ident,
            config_struct_name: ident_with_suffix(&ast.ident, "Config"),
            action_discriminant_name: ident_with_suffix(&ast.ident, "ActionDiscriminant"),
            action_state_enum_name: ident_with_suffix(&ast.ident, "ActionState"),
            basis: attr_on_enum.basis,
            serde: attr_on_enum.serde,
            config_ext: attr_on_enum.config_ext,
            commands: data_enum
                .variants
                .iter()
                .map(ParsedCommand::new)
                .collect::<Result<Vec<_>, _>>()?,
        })
    }

    pub fn gen_serde_clauses_if_set(&self) -> (proc_macro2::TokenStream, proc_macro2::TokenStream) {
        if self.serde.is_some() {
            self.gen_serde_clauses_always()
        } else {
            (quote! {}, quote! {})
        }
    }

    pub fn gen_serde_clauses_always(&self) -> (proc_macro2::TokenStream, proc_macro2::TokenStream) {
        let span = self.serde.unwrap_or_else(proc_macro2::Span::call_site);
        (
            quote_spanned! {span=>
                bevy_tnua::serde::Serialize, bevy_tnua::serde::Deserialize,
            },
            quote_spanned! {span=>
                #[serde(crate = "bevy_tnua::serde")]
            },
        )
    }
}

#[derive(Debug)]
struct AttrOnEnum {
    basis: syn::Type,
    serde: Option<proc_macro2::Span>,
    config_ext: Option<syn::Type>,
}

impl AttrOnEnum {
    fn new(ast: &syn::DeriveInput) -> syn::Result<Self> {
        let mut basis: Option<syn::Type> = None;
        let mut serde: Option<proc_macro2::Span> = None;
        let mut config_ext: Option<syn::Type> = None;
        for arg in AttrArg::iter_in_list_attributes(&ast.attrs, "scheme")? {
            match arg.name().to_string().as_str() {
                "basis" => {
                    arg.already_set_if(basis.is_some())?;
                    basis = Some(arg.key_value()?.parse_value()?);
                }
                "serde" => {
                    arg.apply_flag_to_field(
                        &mut serde,
                        "serializing and deserializing the entire state",
                    )?;
                }
                "config_ext" => {
                    arg.already_set_if(config_ext.is_some())?;
                    config_ext = Some(arg.key_value()?.parse_value()?);
                }
                _ => Err(arg.unknown_parameter())?,
            }
        }
        Ok(Self {
            basis: basis.ok_or(StaticError::CallSite(
                "Scheme is missing basis (`#[scheme(basis = ...)])`",
            ))?,
            serde,
            config_ext,
        })
    }
}

/// A variant in the scheme.
///
/// Note: this is called "command" instead of "action" because while a command have one action,
/// multiple commands may use the same action and also a command may have things beside the action.
#[derive(Debug)]
pub struct ParsedCommand<'a> {
    pub command_name: &'a syn::Ident,
    pub action_type: &'a syn::Type,
    pub command_name_snake: syn::Ident,
    pub payloads: Vec<ParsedPayload<'a>>,
}

impl<'a> ParsedCommand<'a> {
    pub fn new(variant: &'a syn::Variant) -> syn::Result<Self> {
        let fields_unnamed = match &variant.fields {
            syn::Fields::Named(_) => Err(StaticError::Spanned(
                variant,
                "Struct variants not allowed in a scheme - only tuple variants",
            ))?,
            syn::Fields::Unnamed(fields_unnamed) => fields_unnamed,
            syn::Fields::Unit => Err(StaticError::Spanned(
                variant,
                "Unit variants not allowed in a scheme - only tuple variants",
            ))?,
        };
        let mut it = fields_unnamed.unnamed.iter();
        let action_type = it
            .next()
            .ok_or(StaticError::Spanned(variant, "Missing action type"))?;
        let payloads = it.map(ParsedPayload::new).collect::<Result<_, _>>()?;
        Ok(Self {
            command_name: &variant.ident,
            action_type: &action_type.ty,
            command_name_snake: Ident::new(
                &variant.ident.to_string().to_case(Case::Snake),
                variant.ident.span(),
            ),
            payloads,
        })
    }
}

#[derive(Debug)]
pub struct ParsedPayload<'a> {
    pub payload_type: &'a syn::Type,
    pub modify_basis_config: Option<proc_macro2::Span>,
    // pub modify_action_config: Option<proc_macro2::Span>,
}

impl<'a> ParsedPayload<'a> {
    pub fn new(field: &'a syn::Field) -> syn::Result<Self> {
        let mut modify_basis_config = None;
        // let mut modify_action_config = None;

        for arg in AttrArg::iter_in_list_attributes(&field.attrs, "scheme")? {
            match arg.name().to_string().as_str() {
                "modify_basis_config" => {
                    arg.apply_flag_to_field(&mut modify_basis_config, "modifying basis config")?
                }
                // "modify_action_config" => arg.apply_flag_to_field(&mut modify_action_config, "modifying action config")?,
                _ => Err(arg.unknown_parameter())?,
            }
        }
        Ok(Self {
            payload_type: &field.ty,
            modify_basis_config,
            // modify_action_config,
        })
    }
}
