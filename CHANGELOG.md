# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](http://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
