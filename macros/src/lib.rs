use proc_macro2::TokenStream;
use syn::{DeriveInput, parse::Error, parse_macro_input, spanned::Spanned};

use self::action_slots_derive::codegen::generate_action_slots_derive;
use self::action_slots_derive::parsed::ParsedActionSlots;
use self::scheme_derive::codegen::generate_scheme_derive;
use self::scheme_derive::parsed::ParsedScheme;

mod action_slots_derive;
mod scheme_derive;
#[allow(unused)]
mod util;

/// Make an enum a control scheme for a Tnua character controller.
///
/// This implements the `TnuaScheme` trait for the enum, and also generates the following structs
/// required for implementing it (replace `{name}` with the name of the control scheme enum):
///
/// * `{name}Config` - a struct with the configuration of the basis and all the actions.
/// * `{name}ActionDiscriminant` - an enum mirroring the control scheme, except all the variants
///   are units.
/// * `{name}ActionState` - an enum mirroring the control scheme, except instead of just the input
///   types each variant contains a `TnuaActionState` which holds the input, configuration and
///   memory of the action.
///
/// The enum itself **must** have a `#[scheme(basis = ...)]` attribute that specifies the basis of
/// the control scheme (typically `TnuaBuiltinWalk`). The following additional parameters are
/// allowed on that `scheme` attribute on the enum:
///
/// * `#[scheme(serde)]` - derive Serialize and Deserialize on the generated action state enum.
///   * This is mostly useful with (and will probably fail without) the `serialize` feature enabled
///     on the bevy-tnua crate.
///   * The control scheme enum itself will not get these derives automatically - that derive will
///     need to be added manually.
///   * With these, and with the `serialize` feature enabled, the `TnuaController` and
///     `TnuaGhostOverwrites` of the control scheme will also be serializable and deserializable -
///     allowing networking libraries to synchronize them between machines.
///   * Even without this setting and without the `serialize` feature on the bevy-tnua crate, the
///     generated configuration struct and the action discriminant enum will still get these
///     derives.
///
/// Each variant **must** be a tuple variant, where the first element of the tuple is the action,
/// followed by zero or more payloads.
///
/// Payloads are ignored by Tnua itself - they are for the user systems to keep track of data
/// related to the actions - except when they are annotated by `#[scheme(modify_basis_config)]`.
/// Such payloads will modify the configuration when the action they are part of is in effect.
///
/// Example:
///
/// ```ignore
/// #[derive(TnuaScheme)]
/// #[scheme(basis = TnuaBuiltinWalk)]
/// pub enum ControlScheme {
///     Jump(TnuaBuiltinJump),
///     Crouch(
///         TnuaBuiltinCrouch,
///         // While this action is in effect, `SlowDownWhileCrouching` will change the
///         // `TnuaBuiltinWalkConfig` to reduce character speed.
///         #[scheme(modify_basis_config)] SlowDownWhileCrouching,
///     ),
///     WallSlide(
///         TnuaBuiltinWallSlide,
///         // This payload has is ignored by Tnua, but user code can use it to tell which wall
///         // the character is sliding on.
///         Entity,
///     ),
/// }
/// ```
///
#[proc_macro_derive(TnuaScheme, attributes(scheme))]
pub fn derive_tnua_scheme(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match impl_derive_tnua_scheme(&input) {
        Ok(output) => output.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

fn impl_derive_tnua_scheme(ast: &syn::DeriveInput) -> Result<TokenStream, Error> {
    Ok(match &ast.data {
        syn::Data::Struct(_) => {
            return Err(Error::new(
                ast.span(),
                "TnuaScheme is not supported for structs - only for enums",
            ));
        }
        syn::Data::Enum(data_enum) => {
            let parsed = ParsedScheme::new(ast, data_enum)?;
            generate_scheme_derive(&parsed)?
        }
        syn::Data::Union(_) => {
            return Err(Error::new(
                ast.span(),
                "TnuaScheme is not supported for unions - only for enums",
            ));
        }
    })
}

/// Define the behavior of action that can be performed a limited amount of times during certain
/// durations (e.g. air actions)
///
/// This macro must be defined on a struct with a `#[slots(scheme = ...)]` attribute on the struct
/// itself, pointing to a [`TnuaScheme`] that the slots belong to.
///
/// Each field of the struct must have the type [`usize`], and have a `#[slots(...)]` attribute on
/// it listing the actions (variants of the scheme enum) belonging to that slot.
///
/// Not all actions need to be assigned to slots, but every slot needs at least one action assigned
/// to it.
///
/// A single action must not be assigned to more than one slot, but a single slot is allowed to
/// have multiple actions (`#[slots(Action1, Action2, ...)]`)
///
/// The main attribute on the struct can also have a `#[slots(ending(...))]` parameter, listing
/// actions that end the counting. This is used to signal that the counting should start anew after
/// these actions, even if the regular conditions for terminating and re-starting the counting
/// don't occur. For example - when counting air actions, a wall slide should end the counting so
/// that after jumping from it'd be a new air duration and the player could air-jump again even if
/// they've exhausted all the air jumps before the wall slide.
///
/// Example:
///
/// ```ignore
/// #[derive(Debug, TnuaActionSlots)]
/// #[slots(scheme = ControlScheme, ending(WallSlide))]
/// pub struct DemoControlSchemeAirActions {
///     #[slots(Jump)]
///     jump: usize,
///     #[slots(Dash)]
///     dash: usize,
///     // Other actions, like `Crouch`
/// }
/// ```
#[proc_macro_derive(TnuaActionSlots, attributes(slots))]
pub fn derive_tnua_action_slots(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match impl_derive_tnua_action_slots(&input) {
        Ok(output) => output.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

fn impl_derive_tnua_action_slots(ast: &syn::DeriveInput) -> Result<TokenStream, Error> {
    generate_action_slots_derive(&ParsedActionSlots::new(ast)?)
}
