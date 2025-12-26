use crate::util::{AttrArg, StaticError, ident_with_suffix};

#[derive(Debug)]
pub struct ParsedScheme<'a> {
    pub vis: &'a syn::Visibility,
    pub scheme_name: &'a syn::Ident,
    pub config_struct_name: syn::Ident,
    pub action_discriminant_name: syn::Ident,
    pub action_state_enum_name: syn::Ident,
    pub basis: syn::Ident,
}

impl<'a> ParsedScheme<'a> {
    pub fn new(ast: &'a syn::DeriveInput, _data_enum: &'a syn::DataEnum) -> syn::Result<Self> {
        let attr_on_enum = AttrOnEnum::new(ast)?;
        Ok(Self {
            vis: &ast.vis,
            scheme_name: &ast.ident,
            config_struct_name: ident_with_suffix(&ast.ident, "Config"),
            action_discriminant_name: ident_with_suffix(&ast.ident, "ActionDiscriminant"),
            action_state_enum_name: ident_with_suffix(&ast.ident, "ActionStateEnum"),
            basis: attr_on_enum.basis,
        })
    }
}

#[derive(Debug)]
struct AttrOnEnum {
    basis: syn::Ident,
}

impl AttrOnEnum {
    fn new(ast: &syn::DeriveInput) -> syn::Result<Self> {
        let mut basis: Option<syn::Ident> = None;
        for arg in AttrArg::iter_in_list_attributes(&ast.attrs, "scheme")? {
            match arg.name().to_string().as_str() {
                "basis" => {
                    arg.already_set_if(basis.is_some())?;
                    basis = Some(arg.key_value()?.parse_value()?);
                }
                _ => Err(arg.unknown_parameter())?,
            }
        }
        Ok(Self {
            basis: basis.ok_or(StaticError::CallSite(
                "Scheme is missing basis (`#[scheme(basis = ...)])`",
            ))?,
        })
    }
}
