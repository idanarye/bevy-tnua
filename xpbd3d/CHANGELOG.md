# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](http://semver.org/spec/v2.0.0.html).

NOTE: This changelog is shared between bevy-tnua-xpbd2d and bevy-tnua-xpbd3d.

## [Unreleased]
### Changed
- Upgrade to Bevy 0.14.

## 0.4.0 - 2024-04-04
### Changed
- [**BREAKING**] Ray is always cast in the specified direction, regardless of
  the entity's rotation.

## 0.3.0 - 2024-04-02
### Changed
- `f64` flag to run in double precision mod.
- Allow `TnuaXpbd2dPlugin` and `TnuaXpbd3dPlugin` to register their systems in
  different schedules.

## 0.2.0 - 2024-02-24
### Changed
- Upgrade to Bevy 0.13 and bevy_xpbd 0.4.

## 0.1.1 - 2024-01-14
### Fixed
- Use a proper `OR` syntax for the dual license.

## 0.1.0 - 2023-11-13
### Added
- [bevy_xpbd](https://github.com/Jondolf/bevy_xpbd) support - both 2D and 3D.
