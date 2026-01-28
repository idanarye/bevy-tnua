use syn::Error;
use syn::spanned::Spanned;

use crate::util::{AttrArg, StaticError};

#[derive(Debug)]
pub struct ParsedActionSlots<'a> {
    pub slots_name: &'a syn::Ident,
    pub scheme: syn::Ident,
    pub ending_actions: Vec<syn::Ident>,
    pub slots: Vec<ParsedSlot<'a>>,
}

impl<'a> ParsedActionSlots<'a> {
    pub fn new(ast: &'a syn::DeriveInput) -> syn::Result<Self> {
        let struct_data = match &ast.data {
            syn::Data::Struct(data_struct) => data_struct,
            syn::Data::Enum(_) => {
                return Err(Error::new(
                    ast.span(),
                    "TnuaActionSlots is not supported for enums - only for structs",
                ));
            }
            syn::Data::Union(_) => {
                return Err(Error::new(
                    ast.span(),
                    "TnuaActionSlots is not supported for unions - only for structs",
                ));
            }
        };

        let mut scheme: Option<syn::Ident> = None;
        let mut ending_actions: Vec<syn::Ident> = Vec::new();

        for arg in AttrArg::iter_in_list_attributes(&ast.attrs, "slots")? {
            match arg.name().to_string().as_str() {
                "scheme" => {
                    arg.already_set_if(scheme.is_some())?;
                    scheme = Some(arg.key_value()?.parse_value()?);
                }
                "ending" => {
                    ending_actions.extend(arg.sub_attr()?.args::<syn::Ident>()?);
                }
                _ => Err(arg.unknown_parameter())?,
            }
        }

        let slots = struct_data
            .fields
            .iter()
            .map(|field| {
                // AttrArg may be more flexible than what we may need right now, but we may want more
                // complex parameters in the future.
                let mut actions = Vec::new();
                for arg in AttrArg::iter_in_list_attributes(&field.attrs, "slots")? {
                    let AttrArg::Flag(action) = arg else {
                        return Err(syn::Error::new_spanned(
                            arg,
                            "Only action variant names are supported here",
                        ));
                    };
                    actions.push(action);
                }
                if actions.is_empty() {
                    return Err(syn::Error::new_spanned(
                        field,
                        "Slot with no actions assigned to it",
                    ));
                }
                Ok(ParsedSlot {
                    counter_name: field
                        .ident
                        .as_ref()
                        .expect("struct fields always have ident"),
                    actions,
                })
            })
            .collect::<Result<Vec<_>, syn::Error>>()?;

        Ok(Self {
            slots_name: &ast.ident,
            scheme: scheme.ok_or(StaticError::CallSite(
                "Action slots is missing scheme (`#[slots(scheme = ...)])`)",
            ))?,
            ending_actions,
            slots,
        })
    }
}

#[derive(Debug)]
pub struct ParsedSlot<'a> {
    pub counter_name: &'a syn::Ident,
    pub actions: Vec<syn::Ident>,
}
