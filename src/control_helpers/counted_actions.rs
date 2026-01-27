use std::marker::PhantomData;

use bevy::ecs::schedule::{InternedScheduleLabel, ScheduleLabel};
use bevy::prelude::*;

use crate::basis_capabilities::TnuaBasisWithGround;
use crate::controller::TnuaActionFlowStatus;
use crate::{
    TnuaActionDiscriminant, TnuaBasisAccess, TnuaController, TnuaScheme, TnuaUserControlsSystems,
};

pub trait TnuaActionSlots: 'static + Send + Sync + Default {
    type Scheme: TnuaScheme;

    fn get_mut(
        &mut self,
        action: <Self::Scheme as TnuaScheme>::ActionDiscriminant,
    ) -> Option<&mut usize>;
    fn get(&self, action: <Self::Scheme as TnuaScheme>::ActionDiscriminant) -> Option<usize>;
}

#[derive(Default)]
pub enum TnuaActionCountingStatus {
    CountActions,
    #[default]
    ActionsAreFree,
}

/// The result of [`TnuaActionCountingStatus::update()`].
#[derive(Debug, Clone, Copy)]
pub enum TnuaActionCountingUpdate<D: TnuaActionDiscriminant> {
    /// Nothing of interest happened this frame.
    NoChange,

    /// The character has just started a free fall this frame.
    CountingActivated,

    CountingActivatedByAction(D),

    /// The character has just started an air action this frame.
    CountedActionStarted(D),

    /// The character has just finished an air action this frame, and is still in the air.
    ActionFinishedStillCounting,

    /// The character has just landed this frame.
    CountingEnded,
}

pub enum TnuaActionCountingActionRule {
    Counted,
    Uncounted,
    ResttingCount,
}

impl TnuaActionCountingStatus {
    fn update<S: TnuaScheme>(
        &mut self,
        controller: &TnuaController<S>,
        status_for_basis: impl FnOnce(&TnuaBasisAccess<S::Basis>) -> TnuaActionCountingStatus,
        rule_for_action: impl FnOnce(S::ActionDiscriminant) -> TnuaActionCountingActionRule,
    ) -> TnuaActionCountingUpdate<S::ActionDiscriminant> {
        match controller.action_flow_status() {
            TnuaActionFlowStatus::NoAction => {
                self.update_based_on_basis(controller, status_for_basis)
            }
            TnuaActionFlowStatus::ActionOngoing(action_discriminant) => {
                match rule_for_action(*action_discriminant) {
                    TnuaActionCountingActionRule::Counted => match self {
                        Self::CountActions => TnuaActionCountingUpdate::NoChange,
                        Self::ActionsAreFree => {
                            *self = Self::CountActions;
                            TnuaActionCountingUpdate::CountingActivatedByAction(
                                *action_discriminant,
                            )
                        }
                    },
                    TnuaActionCountingActionRule::Uncounted => {
                        self.update_based_on_basis(controller, status_for_basis)
                    }
                    TnuaActionCountingActionRule::ResttingCount => match self {
                        Self::CountActions => {
                            *self = Self::ActionsAreFree;
                            TnuaActionCountingUpdate::CountingEnded
                        }
                        Self::ActionsAreFree => TnuaActionCountingUpdate::NoChange,
                    },
                }
            }
            TnuaActionFlowStatus::ActionStarted(action_discriminant)
            | TnuaActionFlowStatus::Cancelled {
                old: _,
                new: action_discriminant,
            } => match rule_for_action(*action_discriminant) {
                TnuaActionCountingActionRule::Counted => match self {
                    Self::CountActions => {
                        TnuaActionCountingUpdate::CountedActionStarted(*action_discriminant)
                    }
                    Self::ActionsAreFree => {
                        *self = Self::CountActions;
                        TnuaActionCountingUpdate::CountingActivatedByAction(*action_discriminant)
                    }
                },
                TnuaActionCountingActionRule::Uncounted => {
                    self.update_based_on_basis(controller, status_for_basis)
                }
                TnuaActionCountingActionRule::ResttingCount => {
                    *self = Self::ActionsAreFree;
                    TnuaActionCountingUpdate::CountingEnded
                }
            },
            TnuaActionFlowStatus::ActionEnded(_) => {
                let result = self.update_based_on_basis(controller, status_for_basis);
                match self {
                    TnuaActionCountingStatus::CountActions => {
                        TnuaActionCountingUpdate::ActionFinishedStillCounting
                    }
                    TnuaActionCountingStatus::ActionsAreFree => result,
                }
            }
        }
    }

    fn update_based_on_basis<S: TnuaScheme>(
        &mut self,
        controller: &TnuaController<S>,
        status_for_basis: impl FnOnce(&TnuaBasisAccess<S::Basis>) -> TnuaActionCountingStatus,
    ) -> TnuaActionCountingUpdate<S::ActionDiscriminant> {
        let Ok(basis_access) = controller.basis_access() else {
            return TnuaActionCountingUpdate::NoChange;
        };
        // TODO: replace the air-related enums with new ones
        match (&self, status_for_basis(&basis_access)) {
            (Self::CountActions, Self::CountActions) => TnuaActionCountingUpdate::NoChange,
            (Self::CountActions, Self::ActionsAreFree) => {
                *self = Self::ActionsAreFree;
                TnuaActionCountingUpdate::CountingEnded
            }
            (Self::ActionsAreFree, Self::CountActions) => {
                TnuaActionCountingUpdate::CountingActivated
            }
            (Self::ActionsAreFree, Self::ActionsAreFree) => TnuaActionCountingUpdate::NoChange,
        }
    }
}

#[derive(Deref, DerefMut, Component)]
pub struct TnuaActionsCounter<S: TnuaActionSlots> {
    counting_status: TnuaActionCountingStatus,
    current_action: Option<(<S::Scheme as TnuaScheme>::ActionDiscriminant, usize)>,
    #[deref]
    slots: S,
}

impl<S: TnuaActionSlots + Default> Default for TnuaActionsCounter<S> {
    fn default() -> Self {
        Self {
            counting_status: Default::default(),
            current_action: None,
            slots: Default::default(),
        }
    }
}

impl<S: TnuaActionSlots> TnuaActionsCounter<S>
// TODO: get rid of these requirements
where
    <S::Scheme as TnuaScheme>::Basis: TnuaBasisWithGround,
{
    /// Call this every frame, at the schedule of [`TnuaControllerPlugin`], to track the actions.
    pub fn update(&mut self, controller: &TnuaController<S::Scheme>) {
        let update = self.counting_status.update(
            controller,
            |basis| {
                if <<S::Scheme as TnuaScheme>::Basis as TnuaBasisWithGround>::is_airborne(basis) {
                    TnuaActionCountingStatus::CountActions
                } else {
                    TnuaActionCountingStatus::ActionsAreFree
                }
            },
            |action| {
                if self.slots.get(action).is_some() {
                    TnuaActionCountingActionRule::Counted
                } else {
                    TnuaActionCountingActionRule::Uncounted
                }
            },
        );

        match update {
            TnuaActionCountingUpdate::NoChange => {}
            TnuaActionCountingUpdate::CountingActivated => {
                // The free fall is considered the first action
                self.current_action = None;
                // self.air_actions_count += 1;
            }
            // TODO: should these two have different meaning?
            TnuaActionCountingUpdate::CountingActivatedByAction(action_discriminant)
            | TnuaActionCountingUpdate::CountedActionStarted(action_discriminant) => {
                let slot = self
                    .slots
                    .get_mut(action_discriminant)
                    .expect("Should only get AirActionStarted for air actions");
                self.current_action = Some((action_discriminant, *slot));
                *slot += 1;
            }
            TnuaActionCountingUpdate::ActionFinishedStillCounting => {
                self.current_action = None;
            }
            TnuaActionCountingUpdate::CountingEnded => {
                self.current_action = None;
                self.slots = Default::default();
            }
        }
    }

    pub fn count_for(&self, action: <S::Scheme as TnuaScheme>::ActionDiscriminant) -> usize {
        if let Some((current_action, actions)) = self.current_action
            && current_action == action
        {
            return actions;
        }
        self.slots.get(action).unwrap_or_default() // TODO - return None?
    }
}

pub struct TnuaAirActionsPlugin<S: TnuaActionSlots> {
    schedule: InternedScheduleLabel,
    _phantom: PhantomData<S>,
}

impl<S: TnuaActionSlots> TnuaAirActionsPlugin<S> {
    pub fn new(schedule: impl ScheduleLabel) -> Self {
        Self {
            schedule: schedule.intern(),
            _phantom: PhantomData,
        }
    }
}

impl<S: TnuaActionSlots> Plugin for TnuaAirActionsPlugin<S>
where
    <S::Scheme as TnuaScheme>::Basis: TnuaBasisWithGround,
{
    fn build(&self, app: &mut App) {
        app.register_required_components::<TnuaController<S::Scheme>, TnuaActionsCounter<S>>();
        app.add_systems(
            self.schedule,
            actions_counter_update_system::<S>.in_set(TnuaUserControlsSystems),
        );
    }
}

fn actions_counter_update_system<S: TnuaActionSlots>(
    mut query: Query<(&mut TnuaActionsCounter<S>, &TnuaController<S::Scheme>)>,
) where
    <S::Scheme as TnuaScheme>::Basis: TnuaBasisWithGround,
{
    for (mut counter, controller) in query.iter_mut() {
        counter.update(controller);
    }
}
