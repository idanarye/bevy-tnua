# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](http://semver.org/spec/v2.0.0.html).

NOTES:

* This changelog is shared between bevy-tnua-avian2d and bevy-tnua-avian3d.
* Avian used to be named bevy_xpbd. The old bevy-tnua-xpbd changelog can be seen [here](https://github.com/idanarye/bevy-tnua/blob/3cba881c8825633a8d8bdca1fe30e54500e655b8/xpbd3d/CHANGELOG.md).

## [Unreleased]
### Added
- Support for `TnuaObstacleRadar`.
- `TnuaSpatialExtAvian2d`/`TnuaSpatialExtAvian3d` - implementation for the
  `TnuaSpatialExt` trait.

## 0.2.0 - 2024-12-21
### Changed
- Upgrade to Bevy 0.15 and avian 0.2.

### Removed
- `TnuaAvian#dPlugin` no longer implements `Default`. Since Avian changed their
  default schedule from `PostUpdate` to `FixedPostUpdate`, user code that just
  uses the de-facto default `Update` will start having weird results. This
  forces the user to make a deliberate decision regarding which schedule to run.

## 0.1.1 - 2024-08-02
### Fixed
- Run `TnuaSystemSet` before `PhysicsStepSet::First` rather than
  `PhysicsStepSet::BroadPhase`. Apparently Avian changed how the sets are organized...

## 0.1.0 - 2024-07-06
### Added
- [avian](https://github.com/Jondolf/avian) (formerly bevy_xpbd) support - both 2D and 3D.
