use crate::util::ident_with_suffix;

#[derive(Debug)]
pub struct ParsedScheme<'a> {
    pub vis: &'a syn::Visibility,
    pub scheme_name: &'a syn::Ident,
    pub config_struct_name: syn::Ident,
    pub action_discriminant_name: syn::Ident,
    pub action_state_enum_name: syn::Ident,
}

impl<'a> ParsedScheme<'a> {
    pub fn new(ast: &'a syn::DeriveInput, _data_enum: &'a syn::DataEnum) -> syn::Result<Self> {
        Ok(Self {
            vis: &ast.vis,
            scheme_name: &ast.ident,
            config_struct_name: ident_with_suffix(&ast.ident, "Config"),
            action_discriminant_name: ident_with_suffix(&ast.ident, "ActionDiscriminant"),
            action_state_enum_name: ident_with_suffix(&ast.ident, "ActionStateEnum"),
        })
    }
}
