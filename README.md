[![Build Status](https://github.com/idanarye/bevy-tnua/workflows/CI/badge.svg)](https://github.com/idanarye/bevy-tnua/actions)
[![Latest Version](https://img.shields.io/crates/v/bevy-tnua.svg)](https://crates.io/crates/bevy-tnua)
[![Rust Documentation](https://img.shields.io/badge/nightly-rustdoc-blue.svg)](https://idanarye.github.io/bevy-tnua/)
[![Rust Documentation](https://img.shields.io/badge/stable-rustdoc-purple.svg)](https://docs.rs/bevy-tnua/)

# Tnua - A Character Controller for [Bevy](https://bevyengine.org/).

Tnua ("motion" in Hebrew) is a floating character controller, which means that instead of constantly touching the ground the character floats above it, which makes many aspects of the motion control simpler.

Tnua can use [Rapier](https://rapier.rs/) or [XPBD](https://github.com/Jondolf/bevy_xpbd), and supports both the 2D and 3D versions of both with integration crates:
* For Rapier 2D, add the [bevy-tnua-rapier2d](https://crates.io/crates/bevy-tnua-rapier2d) crate.
* For Rapier 3D, add the [bevy-tnua-rapier3d](https://crates.io/crates/bevy-tnua-rapier3d) crate.
* For XPBD 2D, add the [bevy-tnua-xpbd2d](https://crates.io/crates/bevy-tnua-xpbd2d) crate.
* For XPBD 3D, add the [bevy-tnua-xpbd3d](https://crates.io/crates/bevy-tnua-xpbd3d) crate.
* Third party integration crates. Such crates should depend on [bevy-tnua-physics-integration-layer](https://crates.io/crates/bevy-tnua-physics-integration-layer) and not the main bevy-tnua crate.

Note that **both** integration crate (`bevy-tnua-<physics-backend>`) and the main `bevy-tnua` crate are required, and that the main plugin from both crates should be added.

## Features

* Supports both 2D and 3D versions of [Rapier](https://rapier.rs/) and [XPBD](https://github.com/Jondolf/bevy_xpbd)
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

## Demos:

| Demo          | Rapier                                                          | XPBD                                                          |
|---------------|-----------------------------------------------------------------|---------------------------------------------------------------|
| 2D Platformer | https://idanarye.github.io/bevy-tnua/demos/platformer_2d-rapier | https://idanarye.github.io/bevy-tnua/demos/platformer_2d-xpbd |
| 3D Platformer | https://idanarye.github.io/bevy-tnua/demos/platformer_3d-rapier | https://idanarye.github.io/bevy-tnua/demos/platformer_3d-xpbd |

### Running the Demos Locally

```sh
$ cargo run --bin <demo-name> --features <physics-backend>
```

Where `<demo-name>` is the name of the demo and `<physics-backend>` is either `rapier2d`, `rapier3d`, `xpbd2d` or `xpbd3d`. Make sure to match the dimensionality of the backend (2D or 3D) to that of the demo. For example, to run the 3D platformer with XPBD, use this:

```sh
$ cargo run --bin platformer_3d --features xpbd3d
```

### Interesting Parts of the Demo Code

* Check out [the demos' entry points](demos/src/bin/) to see how the plugins and the player character entities are being set.
* Check out [the character control systems](demos/src/character_control_systems/) to see how to control the character's motion and special movement actions.
* Check out [the character animating systems](demos/src/character_animating_systems/) to see how to use information from Tnua for character animation.

## Versions

Tnua is broken into different crates that update separately, so this is broken into multiple tables. The version of bevy-tnua-physics-integration-layer must be the same for both the main bevy-tnua crate and the integration crates.

### Main

| bevy | bevy-tnua-physics-integration-layer | bevy-tnua  |
|------|-------------------------------------|------------|
| 0.13 | 0.2                                 | 0.15       |
| 0.12 | 0.1                                 | 0.13-0.14  |

### Rapier integration

| bevy | bevy-tnua-physics-integration-layer | bevy-tnua-rapier | bevy_rapier |
|------|-------------------------------------|------------------|-------------|
| 0.13 | 0.2                                 | 0.3              | 0.25        |
| 0.12 | 0.1                                 | 0.2              | 0.24        |
| 0.12 | 0.1                                 | 0.1              | 0.23        |

### XPBD integration

| bevy | bevy-tnua-physics-integration-layer | bevy-tnua-xpbd | bevy_xpbd |
|------|-------------------------------------|----------------|-----------|
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
