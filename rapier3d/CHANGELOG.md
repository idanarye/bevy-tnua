# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](http://semver.org/spec/v2.0.0.html).

NOTE: This changelog is shared between bevy-tnua-rapier2d and bevy-tnua-rapier3d.

## [Unreleased]

## 0.8.0 - 2024-12-13
### Changed
- Upgrade to Bevy 0.15 and bevy_rapier 0.28.

## 0.7.0 - 2024-07-08
### Changed
- Upgrade to Bevy 0.14 and bevy_rapier 0.27.

## 0.6.0 - 2024-05-07
### Changed
- Upgrade to bevy_rapier 0.26.

## 0.5.0 - 2024-04-04
### Changed
- [**BREAKING**] Ray is always cast in the specified direction, regardless of
  the entity's rotation.

## 0.4.0 - 2024-04-02
### Changed
- Allow `TnuaRapier2dPlugin` and `TnuaRapier3dPlugin` to register their systems
  in different schedules.

## 0.3.0 - 2024-02-24
### Changed
- Upgrade to Bevy 0.13 and bevy_rapier 0.25.

## 0.2.0 - 2024-02-12
### Changed
- Upgrade bevy_rapier to 0.24.

## 0.1.1 - 2024-01-14
### Fixed
- Use a proper `OR` syntax for the dual license.

## 0.1.0 - 2023-11-13
### Changed
- Splitted out of the main bevy-tnua crate.
