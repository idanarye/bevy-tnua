#!/bin/bash

(
    retrospective-crate-version-tagging detect \
        --crate-name bevy-tnua \
        --changelog-path CHANGELOG.md \
        --tag-prefix v \
        --title-prefix "Main Crate"
    retrospective-crate-version-tagging detect \
        --crate-name bevy-tnua-physics-integration-layer \
        --changelog-path physics-integration-layer/CHANGELOG.md \
        --tag-prefix physics-integration-layer-v \
        --title-prefix "Physics Integration Layer"
    retrospective-crate-version-tagging detect \
        --crate-name bevy-tnua-avian3d \
        --changelog-path avian3d/CHANGELOG.md \
        --tag-prefix avian-v \
        --title-prefix "Avian Integration"
    retrospective-crate-version-tagging detect \
        --crate-name bevy-tnua-rapier3d \
        --changelog-path rapier3d/CHANGELOG.md \
        --tag-prefix rapier-v \
        --title-prefix "Rapier Integration"
) | retrospective-crate-version-tagging create-releases
