use std::time::Duration;

use crate::TnuaActionLifecycleDirective;
use crate::TnuaActionLifecycleStatus;
use crate::TnuaMotor;
use crate::schemes_action_state::Tnua2ActionStateInterface;
use bevy::prelude::*;
use bevy_tnua_physics_integration_layer::data_for_backends::TnuaProximitySensor;
use bevy_tnua_physics_integration_layer::data_for_backends::TnuaRigidBodyTracker;

use crate::TnuaBasisContext;
use crate::math::*;

pub trait TnuaScheme: 'static + Send + Sync + Sized {
    type Basis: Tnua2Basis;
    type Config: TnuaSchemeConfig<Scheme = Self> + Asset;
    type ActionStateEnum: Tnua2ActionStateEnum<Basis = Self::Basis>;

    const NUM_VARIANTS: usize;

    fn variant_idx(&self) -> usize;

    fn is_same_action_as(&self, other: &Self) -> bool {
        self.variant_idx() == other.variant_idx()
    }

    fn into_action_state_variant(self, config: &Self::Config) -> Self::ActionStateEnum;

    fn update_in_action_state_enum(
        self,
        action_state_enum: &mut Self::ActionStateEnum,
    ) -> UpdateInActionStateEnumResult<Self>;
}

pub enum UpdateInActionStateEnumResult<S: TnuaScheme> {
    Success,
    WrongVariant(S),
}

pub trait TnuaSchemeConfig {
    type Scheme: TnuaScheme<Config = Self>;

    fn basis_config(&self) -> &<<Self::Scheme as TnuaScheme>::Basis as Tnua2Basis>::Config;
}

pub trait Tnua2Basis: Default + 'static + Send + Sync {
    type Config: Clone;
    type Memory: Send + Sync + Default;

    fn apply(
        &self,
        config: &Self::Config,
        memory: &mut Self::Memory,
        ctx: TnuaBasisContext,
        motor: &mut TnuaMotor,
    );

    fn proximity_sensor_cast_range(&self, config: &Self::Config, memory: &Self::Memory) -> Float;
}

pub trait Tnua2Action<B: Tnua2Basis>: 'static + Send + Sync {
    type Config: Clone;
    type Memory: Send + Sync + Default;

    fn apply(
        &self,
        config: &Self::Config,
        memory: &mut Self::Memory,
        ctx: Tnua2ActionContext<B>,
        lifecycle_status: TnuaActionLifecycleStatus,
        motor: &mut TnuaMotor,
    ) -> TnuaActionLifecycleDirective;

    #[allow(unused_variables)]
    fn influence_basis(
        &self,
        config: &Self::Config,
        memory: &Self::Memory,
        ctx: TnuaBasisContext,
        basis_input: &B,
        basis_config: &B::Config,
        basis_memory: &mut B::Memory,
    ) {
    }
}

pub trait Tnua2ActionStateEnum: 'static + Send + Sync {
    type Basis: Tnua2Basis;

    fn variant_idx(&self) -> usize;
    fn interface(&self) -> &dyn Tnua2ActionStateInterface<Self::Basis>;
    fn interface_mut(&mut self) -> &mut dyn Tnua2ActionStateInterface<Self::Basis>;
}

#[derive(Clone)]
pub struct Tnua2BasisAccess<'a, B: Tnua2Basis> {
    pub input: &'a B,
    pub config: &'a B::Config,
    pub memory: &'a B::Memory,
}

pub struct Tnua2ActionContext<'a, B: Tnua2Basis> {
    /// The duration of the current frame.
    pub frame_duration: Float,

    /// A sensor that collects data about the rigid body from the physics backend.
    pub tracker: &'a TnuaRigidBodyTracker,

    /// A sensor that tracks the distance of the character's center from the ground.
    pub proximity_sensor: &'a TnuaProximitySensor,

    /// The direction considered as "up".
    pub up_direction: Dir3,

    /// An accessor to the basis.
    pub basis: &'a Tnua2BasisAccess<'a, B>,
}

impl<'a, B: Tnua2Basis> Tnua2ActionContext<'a, B> {
    /// "Downgrade" to a basis context.
    ///
    /// This is useful for some helper methods of [the concrete basis and its
    /// state](Self::concrete_basis) that require a basis context.
    pub fn as_basis_context(&self) -> TnuaBasisContext<'a> {
        TnuaBasisContext {
            frame_duration: self.frame_duration,
            tracker: self.tracker,
            proximity_sensor: self.proximity_sensor,
            up_direction: self.up_direction,
        }
    }

    pub fn frame_duration_as_duration(&self) -> Duration {
        Duration::from_secs_f64(self.frame_duration.into())
    }
}
