[![Build Status](https://github.com/idanarye/bevy-tnua/workflows/CI/badge.svg)](https://github.com/idanarye/bevy-tnua/actions)
[![Latest Version](https://img.shields.io/crates/v/bevy-tnua.svg)](https://crates.io/crates/bevy-tnua)
[![Rust Documentation](https://img.shields.io/badge/nightly-rustdoc-blue.svg)](https://idanarye.github.io/bevy-tnua/)
[![Rust Documentation](https://img.shields.io/badge/stable-rustdoc-purple.svg)](https://docs.rs/bevy-tnua/)

# Tnua - A Character Controller for [Bevy](https://bevyengine.org/).

Tnua ("motion" in Hebrew) is a floating character controller, which means that instead of constantly touching the ground the character floats above it, which makes many aspects of the motion control simpler.

Tnua can use [Rapier](https://rapier.rs/) or [Avian](https://github.com/Jondolf/avian) (formerly XPBD), and supports both the 2D and 3D versions of both with integration crates:
* For Rapier 2D, add the [bevy-tnua-rapier2d](https://crates.io/crates/bevy-tnua-rapier2d) crate.
* For Rapier 3D, add the [bevy-tnua-rapier3d](https://crates.io/crates/bevy-tnua-rapier3d) crate.
* For Avian 2D, add the [bevy-tnua-avian2d](https://crates.io/crates/bevy-tnua-avian2d) crate.
* For Avian 3D, add the [bevy-tnua-avian3d](https://crates.io/crates/bevy-tnua-avian3d) crate.
* XPBD is still supported with [bevy-tnua-xpbd2d](https://crates.io/crates/bevy-tnua-xpbd2d) and [bevy-tnua-xpbd3d](https://crates.io/crates/bevy-tnua-xpbd3d), but users are encouraged to migrate to Avian
* Third party integration crates. Such crates should depend on [bevy-tnua-physics-integration-layer](https://crates.io/crates/bevy-tnua-physics-integration-layer) and not the main bevy-tnua crate.

Note that:

* **Both** integration crate (`bevy-tnua-<physics-backend>`) and the main `bevy-tnua` crate are required, and that the main plugin from both crates should be added.
* If you use a physics backend with double precision (like Avian with the `f64` flag), you need to add the `f64` flag to all the Tnua crates. This applies to double precision data that gets defined by the physics backend - Bevy itself will still use single precision, and this is the precision the position and rotation will use.

## Features

* Supports both 2D and 3D versions of [Rapier](https://rapier.rs/) and [Avian](https://github.com/Jondolf/avian)
* Running
* Jumping
* Crouching
* Variable height jumping
* Coyote time
* Jump buffer
* Running up/down slopes/stairs
* Tilt correction
* Moving platforms
* Rotating platforms
* Animation helpers (not the animation itself, but Tnua has facilities that help deciding which animation to play)
* [Jump/fall through platforms](https://github.com/idanarye/bevy-tnua/wiki/Jump-fall-Through-Platforms)
* Air actions
* [Obstacle actions](https://github.com/idanarye/bevy-tnua/wiki/Using-the-obstacles-radar):
  * Wall sliding (and jumping)
  * Climbing

## Demos:

* 2D Platformer:
  [Rapier](https://idanarye.github.io/bevy-tnua/demos/platformer_2d-rapier),
  [Avian](https://idanarye.github.io/bevy-tnua/demos/platformer_2d-avian),
  [Avian (f64 version)](https://idanarye.github.io/bevy-tnua/demos/platformer_2d-avian-64)
* 3D Platformer:
  [Rapier](https://idanarye.github.io/bevy-tnua/demos/platformer_3d-rapier),
  [Avian](https://idanarye.github.io/bevy-tnua/demos/platformer_3d-avian),
  [Avian (f64 version)](https://idanarye.github.io/bevy-tnua/demos/platformer_3d-avian-64)
* 3D Shooter:
  [Rapier](https://idanarye.github.io/bevy-tnua/demos/shooter_like-rapier),
  [Avian](https://idanarye.github.io/bevy-tnua/demos/shooter_like-avian),
  [Avian (f64 version)](https://idanarye.github.io/bevy-tnua/demos/shooter_like-avian-64)

The basis and actions in the demos can be tweaked with a GUI. They are initialized to the `Default::default()` provided in Tnua, with the following exceptions:

* `TnuaBuiltinWalk::desired_velocity` defaults to the zero vector, but when the user walks the character it is set to a vector of length 20.0 (40.0 in the 2D demo)
* `TnuaBuiltinWalk::float_height` is set to 2.0 even though it defaults to 0.0. User code should always set the float height based on the model's geometrics.
* `TnuaBuiltinWalk::max_slope` is set to $\frac{\pi}{4}$ even though it defaults to $\frac{\pi}{2}$ (which disables the slipping behavior, since this is the slope angle of a wall)
* `TnuaBuiltinJump::height` is set to 4.0 even though it defaults to 0.0. User code should always set the jump height based on the game's requirements (a jump action of zero height is useless)
* `TnuaBuiltinCrouch::float_offset` is set to -0.9 even though it defaults to 0.0. Just like `float_height`, this value should always be set by user code based on the model's geometric.
* `TnuaBuiltinDash::displacement` defaults to 0.0, but when the user inputs the command to dash it gets set to a vector of length 10.0.

### Running the Demos Locally

```sh
$ cargo run --bin <demo-name> --features <physics-backend>
```

Where `<demo-name>` is the name of the demo and `<physics-backend>` is either `rapier2d`, `rapier3d`, `avian2d` or `avian3d`. Make sure to match the dimensionality of the backend (2D or 3D) to that of the demo. For example, to run the 3D platformer with Avian, use this:

```sh
$ cargo run --bin platformer_3d --features avian3d
```

### Interesting Parts of the Demo Code

(**NOTE**: [the examples](examples/) are simpler than the demos, so if there is an example that shows something it'll be easier to read that)

* Check out [the demos' entry points](demos/src/bin/) to see how the plugins and the player character entities are being set.
* Check out [the character control systems](demos/src/character_control_systems/) to see how to control the character's motion and special movement actions.
* Check out [the character animating systems](demos/src/character_animating_systems/) to see how to use information from Tnua for character animation.

## Versions

Tnua is broken into different crates that update separately, so this is broken into multiple tables. The version of bevy-tnua-physics-integration-layer must be the same for both the main bevy-tnua crate and the integration crates.

### Main

| bevy | bevy-tnua-physics-integration-layer | bevy-tnua  |
|------|-------------------------------------|------------|
| 0.15 | 0.6                                 | 0.22       |
| 0.15 | 0.5                                 | 0.21       |
| 0.14 | 0.4                                 | 0.19-0.20  |
| 0.13 | 0.3                                 | 0.16-0.18  |
| 0.13 | 0.2                                 | 0.15       |
| 0.12 | 0.1                                 | 0.13-0.14  |

### Rapier integration

| bevy | bevy-tnua-physics-integration-layer | bevy-tnua-rapier | bevy_rapier |
|------|-------------------------------------|------------------|-------------|
| 0.15 | 0.6                                 | 0.10             | 0.29        |
| 0.15 | 0.5                                 | 0.8, 0.9         | 0.28        |
| 0.14 | 0.4                                 | 0.7              | 0.27        |
| 0.13 | 0.3                                 | 0.6              | 0.26        |
| 0.13 | 0.3                                 | 0.4, 0.5         | 0.25        |
| 0.13 | 0.2                                 | 0.3              | 0.25        |
| 0.12 | 0.1                                 | 0.2              | 0.24        |
| 0.12 | 0.1                                 | 0.1              | 0.23        |

### Avian integration

| bevy | bevy-tnua-physics-integration-layer | bevy-tnua-avian | avian |
|------|-------------------------------------|-----------------|-------|
| 0.15 | 0.6                                 | 0.3             | 0.2   |
| 0.15 | 0.5                                 | 0.2             | 0.2   |
| 0.14 | 0.4                                 | 0.1             | 0.1   |

### XPBD integration

Will not receive any more updates since the bevy_xpbd project has been rebranded as "Avian".

| bevy | bevy-tnua-physics-integration-layer | bevy-tnua-xpbd | bevy_xpbd |
|------|-------------------------------------|----------------|-----------|
| 0.14 | 0.4                                 | 0.5            | 0.5       |
| 0.13 | 0.3                                 | 0.3, 0.4       | 0.4       |
| 0.13 | 0.2                                 | 0.2            | 0.4       |
| 0.12 | 0.1                                 | 0.1            | 0.3       |

### Pre-split

| bevy | bevy-tnua  | bevy_rapier |
|------|------------|-------------|
| 0.12 | 0.12       | 0.23        |
| 0.11 | 0.8 - 0.11 | 0.22        |
| 0.10 | 0.1 - 0.7  | 0.21        |

## Reference Material

The following were used for coding the math and physics of Tnua:

* "Floating capsule" and running mechanics:
  * https://youtu.be/qdskE8PJy6Q
* Jumping mechanics:
  * https://youtu.be/hG9SzQxaCm8
  * https://youtu.be/eeLPL3Y9jjA

## Alternatives

* [bevy_mod_wanderlust](https://github.com/PROMETHIA-27/bevy_mod_wanderlust) - the original inspiration for this mod, and where I got the floating capsule video from. I ended up creating my own plugin because bevy_mod_wanderlust does not support 2D.
* [Rapier itself has a character controller](https://rapier.rs/docs/user_guides/bevy_plugin/character_controller). It's not a floating character controller, but it's integrated with the physics engine itself and uses that privilege to work out some of the problems the floating model is used to address.

## License

Licensed under either of

 * Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any
additional terms or conditions.
