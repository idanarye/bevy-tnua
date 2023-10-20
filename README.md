[![Build Status](https://github.com/idanarye/bevy-tnua/workflows/CI/badge.svg)](https://github.com/idanarye/bevy-tnua/actions)
[![Latest Version](https://img.shields.io/crates/v/bevy-tnua.svg)](https://crates.io/crates/bevy-tnua)
[![Rust Documentation](https://img.shields.io/badge/nightly-rustdoc-blue.svg)](https://idanarye.github.io/bevy-tnua/)
[![Rust Documentation](https://img.shields.io/badge/stable-rustdoc-purple.svg)](https://docs.rs/bevy-tnua/)

# Tnua - A Character Controller for [bevy_rapier](https://github.com/dimforge/bevy_rapier).

Tnua ("motion" in Hebrew) is a floating character controller, which means that instead of constantly touching the ground the character floats above it, which makes many aspects of the motion control simpler.

Tnua uses [Rapier](https://rapier.rs/), and supports both the 2D and 3D versions of it:

## Features

* Supports both 2D and 3D versions of [Rapier](https://rapier.rs/)
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

## Examples:

* https://idanarye.github.io/bevy-tnua/demos/platformer_2d
* https://idanarye.github.io/bevy-tnua/demos/platformer_3d

## Versions

| bevy | bevy-tnua  | bevy_rapier |
|------|------------|-------------|
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
