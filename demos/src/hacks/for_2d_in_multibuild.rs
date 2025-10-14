use bevy::prelude::*;

/// The CI builds the WASM demos with both 2D and 3D variations, but since we don't load the 3D
/// plugins of the physics backend crates in this demo some of resources required for some of the
/// systems are not reigstered. This registers them, so that the systems are not broken.
pub struct Register3dResourcesInThe2dDemos;

impl Plugin for Register3dResourcesInThe2dDemos {
    fn build(&self, #[allow(unused)] app: &mut App) {
        #[cfg(feature = "avian3d")]
        app.add_message::<avian3d::prelude::CollisionStart>();
        #[cfg(feature = "rapier3d")]
        app.add_message::<bevy_rapier3d::prelude::CollisionEvent>();
    }
}
