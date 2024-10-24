use bevy::{ecs::system::SystemParam, prelude::*};

use bevy_tnua::math::Vector3;
use bevy_tnua::spatial_ext::TnuaSpatialExt;
use bevy_tnua_avian3d::TnuaSpatialExtAvian3d;

#[derive(SystemParam)]
pub struct SpatialExtFacade<'w, 's> {
    #[cfg(feature = "avian3d")]
    for_avian3d: TnuaSpatialExtAvian3d<'w, 's>,
}

#[allow(unreachable_code)]
impl<'w, 's> TnuaSpatialExt for SpatialExtFacade<'w, 's> {
    type ColliderData<'a> = ColliderDataFacade<'a, 'w, 's>
    // type ColliderData<'a> = ()
    where
        Self: 'a;

    fn fetch_collider_data(&self, entity: Entity) -> Option<Self::ColliderData<'_>> {
        Some(ColliderDataFacade {
            #[cfg(feature = "avian3d")]
            for_avian3d: self.for_avian3d.fetch_collider_data(entity)?,
        })
    }

    fn project_point(&'_ self, point: Vector3, collider_data: &Self::ColliderData<'_>) -> Vector3 {
        #[cfg(feature = "avian3d")]
        return self
            .for_avian3d
            .project_point(point, &collider_data.for_avian3d);

        panic!("Running without any physics backend configured");
    }
}

pub struct ColliderDataFacade<'a, 'w, 's>
where
    Self: 'a,
{
    #[cfg(feature = "avian3d")]
    for_avian3d: <TnuaSpatialExtAvian3d<'w, 's> as TnuaSpatialExt>::ColliderData<'a>,
}
