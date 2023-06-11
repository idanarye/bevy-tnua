# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](http://semver.org/spec/v2.0.0.html).

## [Unreleased]

## 0.7.0 - 2023-06-11
### Changed
- Physics backend plugins are now in charge of preventing `TnuaSystemSet` from
  running while the physiscs backend is paused. Users no longer need to do it.

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
