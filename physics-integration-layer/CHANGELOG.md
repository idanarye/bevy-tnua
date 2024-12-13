# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](http://semver.org/spec/v2.0.0.html).

## [Unreleased]
### Added
- `TnuaObstacleRadar` component for detecting nearby colliders.
- `TnuaSpatialExt` trait for allowing physics backend integration crates to
  offer spatial queries in user systems. This is mostly so that the main
  crate can offer helpers (like `TnuaRadarLens`) that do more complex things
  with these queries.
- The `TnuaVelChange::calc_boost` helper method.

## 0.5.0 - 2024-12-13
### Changed
- Upgrade to Bevy 0.15.

## 0.4.0 - 2024-07-05
### Changed
- Upgrade to Bevy 0.14.

## 0.3.0 - 2024-04-02
### Added
- `f64` flag to run in double precision mod (used by the XPBD backend)

## 0.2.0 - 2024-02-24
### Changed
- Upgrade to Bevy 0.13.

## 0.1.1 - 2024-01-14
### Fixed
- Use a proper `OR` syntax for the dual license.

## 0.1.0 - 2023-11-13
### Changed
- Splitted out of the main bevy-tnua crate, so that physics backend integration
  crates can depend on it (and not on the main crate)
