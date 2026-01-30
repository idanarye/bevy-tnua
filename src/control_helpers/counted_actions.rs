use std::marker::PhantomData;

pub use bevy_tnua_macros::TnuaActionSlots;

use bevy::ecs::schedule::{InternedScheduleLabel, ScheduleLabel};
use bevy::prelude::*;
#[cfg(feature = "serialize")]
use serde::{Deserialize, Serialize};

use crate::basis_capabilities::TnuaBasisWithGround;
use crate::controller::TnuaActionFlowStatus;
use crate::{
    TnuaActionDiscriminant, TnuaBasisAccess, TnuaController, TnuaScheme, TnuaUserControlsSystems,
};

/// See [the derive macro](bevy_tnua_macros::TnuaActionSlots).
pub trait TnuaActionSlots: 'static + Send + Sync {
    /// The scheme who's actions are assigned to the slots.
    type Scheme: TnuaScheme;

    /// A state where all the slot counters are zeroed.
    const ZEROES: Self;

    /// Decision what to do when an action starts.
    fn rule_for(
        action: <Self::Scheme as TnuaScheme>::ActionDiscriminant,
    ) -> TnuaActionCountingActionRule;

    /// Get a mutable reference to the counter of the action's slot.
    ///
    /// Note that not all actions have slots. For actions not assigned to any slot, this will
    /// return `None`.
    fn get_mut(
        &mut self,
        action: <Self::Scheme as TnuaScheme>::ActionDiscriminant,
    ) -> Option<&mut usize>;

    /// Get the value of the counter of the action's slot.
    ///
    /// Note that not all actions have slots. For actions not assigned to any slot, this will
    /// return `None`.
    fn get(&self, action: <Self::Scheme as TnuaScheme>::ActionDiscriminant) -> Option<usize>;
}

/// An helper for tracking whether or not the character is in a situation when actions are counted.
///
/// This is a low level construct. Prefer using [`TnuaActionsCounter`], which uses this internally.
#[derive(Default, Debug)]
#[cfg_attr(feature = "serialize", derive(Serialize, Deserialize))]
pub enum TnuaActionCountingStatus {
    CountActions,
    #[default]
    ActionsAreFree,
}

/// The result of [`TnuaActionCountingStatus::update()`].
#[derive(Debug, Clone, Copy)]
pub enum TnuaActionCountingUpdate<D: TnuaActionDiscriminant> {
    /// Nothing of interest happened this update.
    NoChange,

    /// The character has just started a duration where the counted actions are limited, without
    /// performing a counted action.
    ///
    /// e.g.: for air actions, this could mean stepping off a cliff into a free fall.
    CountingActivated,

    /// The character has just started a duration where the counted actions are limited by
    /// performing a counted action.
    ///
    /// e.g.: for air actions, this could mean jumping from the ground.
    CountingActivatedByAction(D),

    /// The character has just started a counted action this frame, when counted actions are
    /// already limited.
    ///
    /// e.g.: for air actions, this could mean doing an air jump.
    CountedActionStarted(D),

    /// The character has just finished a counted action this frame, but counted actions are still
    /// limited.
    ///
    /// e.g.: for air actions, this could mean finishing a dash while still in the air.
    ActionFinishedStillCounting,

    /// The duration where the counted actions are limited has ended for the character.
    ///
    /// e.g.: for air actions, this could mean the character has landed.
    CountingEnded,
}

/// A decision, defined by [`TnuaActionSlots`], regarding an individual action.
pub enum TnuaActionCountingActionRule {
    /// This action needs to be counted.
    ///
    /// Only return this for actions that are assigned to a slot.
    Counted,
    /// This action does not participate in the action counting.
    Uncounted,
    /// This action ends the counting, even if otherwise the condition for that is not met.
    ///
    /// For example - when counting air actions, performing a wall slide action would reset the
    /// counters even though the character is not "grounded".
    EndingCount,
}

impl TnuaActionCountingStatus {
    /// Call this every frame, in the same schedule as
    /// [`TnuaControllerPlugin`](crate::TnuaControllerPlugin), to track the scenario where the
    /// actions are counted.
    ///
    /// The predicates determine what to do based on the state of the current basis and - if an
    /// action just started - based on that action.
    ///
    /// This function both changes the [`TnuaActionCountingStatus`] and returns a
    /// [`TnuaActionCountingUpdate`] that can be used to decide how to update a more complex type
    /// (like [`TnuaActionsCounter`]) that does the actual action counting.
    pub fn update<S: TnuaScheme>(
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
                    TnuaActionCountingActionRule::EndingCount => match self {
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
                TnuaActionCountingActionRule::EndingCount => {
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
        match (&self, status_for_basis(&basis_access)) {
            (Self::CountActions, Self::CountActions) => TnuaActionCountingUpdate::NoChange,
            (Self::CountActions, Self::ActionsAreFree) => {
                *self = Self::ActionsAreFree;
                TnuaActionCountingUpdate::CountingEnded
            }
            (Self::ActionsAreFree, Self::CountActions) => {
                *self = Self::CountActions;
                TnuaActionCountingUpdate::CountingActivated
            }
            (Self::ActionsAreFree, Self::ActionsAreFree) => TnuaActionCountingUpdate::NoChange,
        }
    }
}

/// An helper for counting the actions in scenarios where actions can only be done a limited amount
/// of times. Mainly used for implementing air actions.
///
/// It's [`update`](Self::update) must be called every frame - even when the result is not used -
/// in the same schedule as [`TnuaControllerPlugin`](crate::TnuaControllerPlugin). For air actions,
/// this can usually be done with [`TnuaAirActionsPlugin`].
///
/// This type exposes the slots struct to allow manual interference with the counting, but the
/// actually checking of counters should use [`count_for`](Self::count_for) which also takes into
/// account the currently active action.
#[derive(Component)]
#[cfg_attr(feature = "serialize", derive(Serialize, Deserialize))]
pub struct TnuaActionsCounter<S: TnuaActionSlots> {
    counting_status: TnuaActionCountingStatus,
    #[cfg_attr(
        feature = "serialize",
        serde(bound(
            serialize = "<S::Scheme as TnuaScheme>::ActionDiscriminant: Serialize",
            deserialize = "<S::Scheme as TnuaScheme>::ActionDiscriminant: Deserialize<'de>",
        ))
    )]
    current_action: Option<(<S::Scheme as TnuaScheme>::ActionDiscriminant, usize)>,
    pub slots: S,
}

impl<S: TnuaActionSlots> Default for TnuaActionsCounter<S> {
    fn default() -> Self {
        Self {
            counting_status: Default::default(),
            current_action: None,
            slots: S::ZEROES,
        }
    }
}

impl<S: TnuaActionSlots> TnuaActionsCounter<S> {
    /// Call this every frame, at the schedule of
    /// [`TnuaControllerPlugin`](crate::TnuaControllerPlugin), to track the actions.
    ///
    /// The predicate and the [`TnuaActionSlots`] from the generic parameter define how the
    /// counters will get updated.
    pub fn update(
        &mut self,
        controller: &TnuaController<S::Scheme>,
        status_for_basis: impl FnOnce(
            &TnuaBasisAccess<<S::Scheme as TnuaScheme>::Basis>,
        ) -> TnuaActionCountingStatus,
    ) {
        let update = self
            .counting_status
            .update(controller, status_for_basis, S::rule_for);

        match update {
            TnuaActionCountingUpdate::NoChange => {}
            TnuaActionCountingUpdate::CountingActivated => {
                self.current_action = None;
                // No need to reset the slots - we can assume they are already at default
            }
            // TODO: should these two have different meaning?
            TnuaActionCountingUpdate::CountingActivatedByAction(action_discriminant) => {
                let slot = self
                    .slots
                    .get_mut(action_discriminant)
                    .expect("Should only get CountingActivatedByAction for air actions");
                self.current_action = Some((action_discriminant, *slot));
            }
            TnuaActionCountingUpdate::CountedActionStarted(action_discriminant) => {
                let slot = self
                    .slots
                    .get_mut(action_discriminant)
                    .expect("Should only get CountedActionStarted for air actions");
                *slot += 1;
                self.current_action = Some((action_discriminant, *slot));
            }
            TnuaActionCountingUpdate::ActionFinishedStillCounting => {
                self.current_action = None;
            }
            TnuaActionCountingUpdate::CountingEnded => {
                self.current_action = None;
                self.slots = S::ZEROES;
            }
        }
    }

    /// Calculate the "number" of an action.
    ///
    /// If actions are not currently being counted, this will return 0. Otherwise, it will return
    /// the number the requested action will be - meaning the first one in the counting duration
    /// will be numbered 1.
    ///
    /// If the specified action is currently running, this method will return the number of the
    /// currently running action, not the next action of the same variant. This is done so that
    /// user control systems will keep feeding it - with `allow_in_air: true` - for as long as the
    /// player holds the button. Note that this means that while the very action that triggered the
    /// counting (e.g. - jumping off the ground when counting air actions) is still active, its
    /// number will be 0 (even though action counting starts from 1, this action was from before
    /// the counting so it gets to be 0)
    ///
    /// Each slot gets counted separately. If the action does not belong to any slot, or if actions
    /// are not currently being counted, this returns 0.
    ///
    /// ```no_run
    /// # use bevy_tnua::prelude::*;
    /// # use bevy_tnua::control_helpers::{TnuaActionSlots, TnuaActionsCounter};
    /// # #[derive(TnuaScheme)] #[scheme(basis = TnuaBuiltinWalk)] enum ControlScheme {Jump(TnuaBuiltinJump)}
    /// # #[derive(TnuaActionSlots)] #[slots(scheme = ControlScheme)] struct AirActionSlots {#[slots(Jump)] jump: usize}
    /// # let mut controller = TnuaController::<ControlScheme>::default();
    /// let air_actions: TnuaActionsCounter<AirActionSlots>; // actually get this from a Query
    ///
    /// # air_actions = Default::default();
    /// controller.action(ControlScheme::Jump(TnuaBuiltinJump {
    ///     allow_in_air: air_actions.count_for(ControlSchemeActionDiscriminant::Jump)
    ///         // Allow one air jump - use <= instead of < because the first one in the air will
    ///         // be have its `count_for` return 1.
    ///         <= 1,
    ///     ..Default::default()
    /// }));
    /// ```
    pub fn count_for(&self, action: <S::Scheme as TnuaScheme>::ActionDiscriminant) -> usize {
        if let Some((current_action, actions)) = self.current_action
            && current_action == action
        {
            return actions;
        }
        let Some(slot_value) = self.slots.get(action) else {
            return 0; // non-counted action
        };
        match self.counting_status {
            TnuaActionCountingStatus::CountActions => slot_value + 1,
            TnuaActionCountingStatus::ActionsAreFree => slot_value,
        }
    }
}

/// Use the action slots definition to track air actions.
///
/// Must use the same schedule as the [`TnuaControllerPlugin`](crate::TnuaControllerPlugin).
///
/// Note that this will automatically make [`TnuaActionsCounter<S>`] a dependency component of the
/// [`TnuaController`] parametrized to `S`'s [`Scheme`](TnuaActionSlots::Scheme).
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
        counter.update(controller, |basis| {
            if <<S::Scheme as TnuaScheme>::Basis as TnuaBasisWithGround>::is_airborne(basis) {
                TnuaActionCountingStatus::CountActions
            } else {
                TnuaActionCountingStatus::ActionsAreFree
            }
        });
    }
}
