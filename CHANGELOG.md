# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](http://semver.org/spec/v2.0.0.html).

NOTE: Subcrates have their own changelogs: [bevy-tnua-physics-integration-layer](physics-integration-layer/CHANGELOG.md), [bevy-tnua-rapier](rapier3d/CHANGELOG.md), [bevy-tnua-avian](avian3d/CHANGELOG.md).

## [Unreleased]
### Added
- A `TnuaBuiltinKnockback` action for applying knockback that will not be
  nullified even with very high walk acceleration settings (see
  https://github.com/idanarye/bevy-tnua/issues/30)

### Changed
- Instead of fixating it to positive Y, Tnua now calculates the up direction to
  be the reverse of the gravity direction (see see
  https://github.com/idanarye/bevy-tnua/issues/40)
- [**BREAKING**] API changes:
  - (only relevant for custom basis/actions) The `up_direction` of
    `TnuaBasisContext` and `TnuaActionContext` is now a field instead of a
    method.

## 0.19.0 - 2024-07-05
### Changed
- Upgrade to Bevy 0.14.

## 0.18.0 - 2024-05-18
### Added
- `max_slope` field for `TnuaBuiltinWalk` to make the character treat too steep
  slopes as walls.

## 0.17.0 - 2024-05-07
### Removed
- [**BREAKING**] `TnuaBuiltinWalk` no longer has an `up` field. The up
  direction is fixed to `Direction3d::Y` (up until now, it problably wouln't
  work well with other up directions anyway). This has some other implications,
  which are mostly internal:
  - `DynamicBasis::up_direction()` has been removed. Actions should take their
    up direction from the new `TnuaActionContext::up_direction()`.
    `TnuaBasisContext` also got an `up_direction()` method, for the same
    purpose. For now, they always point up.
  - `TnuaBuiltinWalk::standing_offset` is now a vector instead of a number (it
    was easier to make it that way)

### Added
- Make the `bevy_tnua::util` module public. It contains two helper utilities:
  - `SegmentedJumpInitialVelocityCalculator` for calculating the initial
    velocity required for a jump with varying gravity.
  - `rotation_arc_around_axis` for calculating a character's rotation.
- Re-export `bevy_tnua_physics_integration_layer::math` as `bevy_tnua::math`.

## 0.16.0 - 2024-04-02
### Added
- `f64` flag to run in double precision mod (used by the XPBD backend)
- Allow plugins to register their systems in different schedules. See the
  [migration guide](MIGRATION-GUIDES.md#migrating-to-tnua-016).

## 0.15.0 - 2024-02-24
### Changed
- Upgrade to Bevy 0.13.

## 0.14.2 - 2024-02-02
### Fixed
- Use boost for stopping in `TnuaBuiltinWalk` (fixes
  https://github.com/idanarye/bevy-tnua/issues/39)

## 0.14.1 - 2024-01-14
### Fixed
- Use a proper `OR` syntax for the dual license.

## 0.14.0 - 2024-01-01
### Added
- `is_airborne` method for `TnuaController`.
- `get_count_mut` and `reset_count` methods for `TnuaSimpleAirActionsCounter`.

### Changed
- Use external forces instead of boosting the velocity for movement (fixes
  https://github.com/idanarye/bevy-tnua/issues/34)

### Fixed
- Expose `DynamicBasis` and `DynamicAction`. This is mostly so that they'd
  appear in the docs.

## 0.13.0 - 2023-11-13
### Changed
- [**BREAKING**] Split the physics integration to separate crates. See the
  [migration guide](MIGRATION-GUIDES.md#migrating-to-tnua-013).

### Added
- [XPBD](https://github.com/Jondolf/bevy_xpbd) support (with separate crates)

## 0.12.0 - 2023-11-09
### Changed
- Upgrade Bevy to 0.12

## 0.11.0 - 2023-10-21
### Fixed
- [**BREAKING**] Fix typo `dynaimc_basis` -> `dynamic_basis`.

### Added
- Ability to start a jump while the air.
- A simple dash action - `TnuaBuiltinDash`. Also air-able.
- Utilities for tracking the air actions, so that games can limit how many (and
  which) air actions a character can perform. See `TnuaAirActionsTracker` and
  `TnuaSimpleAirActionsCounter`.

## 0.10.0 - 2023-10-16
### Changed
- [**BREAKING**] Big refactor which completely breaks the API. See the
  [migration guide](MIGRATION-GUIDES.md#migrating-to-tnua-010). The main
  changes are:
  - Instead of having a `TnuaPlatformerControls`, Tnua now has `TnuaController`
    which can be fed a _basis_ and (optionally) an _action_. The basis controls
    the basic floating and walking abound, while the action can be a jump - but
    also all other kinds of movement actions.
  - Instead of `TnuaPlatformerConfig`, the configuration is fed to the basis
    and the action on every frame.
- Turn direction no longer defaults to the walk direction. If it is not passed
  to the `TnuaBuiltinWalk` basis, the character will not turn.
- Crouching is done via an action - `TnuaBuiltinCrouch`.
- Replace `TnuaKeepCrouchingBelowObstacles` with `TnuaCrouchEnforcer`. This is
  not just a name change - their semantics are also different.

### Removed
- There is no longer `forward`. It was only needed before because Tnua needed
  to turn the character's "forward" in the movement direction. Instead, the
  forward direction is always assumed to be negative Z - even if this is not
  the real forward direction of the sprite/model.
- Manual turning. Now that Tnua does not make the character turn in the walk
  direction by default, there is no longer need to redirect that output in games with 2D physics.

## 0.9.0 - 2023-08-17
### Changed
- `TnuaKeepCrouchingBelowObstacles` now also prevent jumping while crouched
  below an obstacle (fixes https://github.com/idanarye/bevy-tnua/issues/27)

## 0.8.0 - 2023-07-11
### Changed
- Upgrade Bevy to 0.11.

## 0.7.0 - 2023-06-11
### Changed
- Physics backend plugins are now in charge of preventing `TnuaSystemSet` from
  running while the physics backend is paused. Users no longer need to do it.

### Added
- `TnuaToggle` for temporarily disabling Tnua for specific entities.

## 0.6.1 - 2023-06-04
### Fixed
- Fix jump shortening not working with takeoff gravity

## 0.6.0 - 2023-06-05
### Added
- Jump/fall through platforms.

## 0.5.0 - 2023-06-02
### Changed
- [**BREAKING**] Tnua now requires additional Rapier components -
  `ExternalForce` and `ReadMassProperties`. For convenience,
  `TnuaRapier2dIOBundle`/`TnuaRapier3dIOBundle` were added. It contains these
  new components, plus `Velocity` (which was already required)
- `TnuaMotor` now has `boost` and `acceleration` for both linear and angular
  components of the motor.
- Rename `jump_start_extra_gravity` to `upslope_jump_extra_gravity`.

### Added
- Settings to add extra gravity during jump takeoff.

## 0.4.0 - 2023-05-21
### Added
- `float_height_offset` control for crouching. Also add:
  - `height_change_impulse_for_duration` and `height_change_impulse_limit`
    settings for controling a boost that would be added for crouching and
    getting back up.
  - `standing_offset` field to `TnuaPlatformerAnimatingOutput` to assist in
    applying crouching/crawling animation.
  - `TnuaKeepCrouchingBelowObstacles` component for preventing the character
    from standing up under a too-low ceiling.
### Changed
- Update proximity sensors in parallel.

## 0.3.0 - 2023-05-13
### Changed
- [**BREAKING**] Removed `TnuaPlatformerBundle::new_with_config`. Users should
  use this instead:
  ```rust
  cmd.insert(TnuaPlatformerBundle {
      config: TnuaPlatformerConfig {
          // ...
      },
      ..Default::default()
  });
  ```
- The character no longer automatically jumps repeatedly when the jump button
  is held. This behavior, though, can be replicated by setting
  `held_jump_cooldown` to `Some(0.0)`.

### Fixed
- Apply additional impulse when moving platform changes velocity to prevent
  https://github.com/idanarye/bevy-tnua/issues/13.
- Fix ray(/shape)cast hitting a wall when the character squeezes into it.

### Added
- `jump_peak_prevention_at_upward_velocity` and
  `jump_peak_prevention_extra_gravity` settings for shortening the time a
  character floats at the peak of the jump.
- `jump_input_buffer_time` setting for jump input buffering - pressing the jump
  button before the character can actually jump.
- `held_jump_cooldown` setting for automatically jumping when the jump button
  is held.

## 0.2.2 - 2023-04-15
### Added
- Bevy `Reflect`ion for `TnuaPlatformerConfig` (and `TnuaFreeFallBehavior`)

## 0.2.1 - 2023-04-03
### Fixed
- Make ray/shape-casts ignore `Sensor`s.

## 0.2.0 - 2023-03-20
### Changed
- [**BREAKING**] `spring_dampening` now gets divided by the frame duration, to
  avoid weird effects from unstable framerate. This means that the proper
  numbers for it should be greatly reduced -  for example in the 3D example it
  was reduced from 60.0 to 1.2.
- Better document how to shapecast and why it is needed. Also make it the
  default for the examples.


## 0.1.0 - 2023-03-16
### Added
- Running
- Jumping
- Variable height jumping
- Coyote time
- Running up/down slopes/stairs
- Tilt correction
- Moving platforms
- Rotating platforms
- Animation helpers (not the animation itself, but Tnua has facilities that help deciding which animation to play)
