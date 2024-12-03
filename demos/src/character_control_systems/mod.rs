pub mod info_dumpeing_systems;
pub mod platformer_control_systems;
mod querying_helpers;
mod spatial_ext_facade;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dimensionality {
    Dim2,
    Dim3,
}
