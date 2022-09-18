use bevy::prelude::*;

#[derive(Component, Debug)]
pub struct TnuaProximitySensor {
    pub cast_origin: Vec3,
    pub cast_direction: Vec3,
    pub cast_range: f32,
    pub entity: Option<Entity>,
    pub proximity: f32,
    pub normal: Vec3,
}

impl TnuaProximitySensor {
    pub fn new(origin: Vec3, direction: Vec3) -> Self {
        Self {
            cast_origin: origin,
            cast_direction: direction,
            cast_range: direction.length(),
            entity: None,
            proximity: std::f32::INFINITY,
            normal: Vec3::ZERO,
        }
    }

    pub fn update(&mut self, entity: Entity, proximity: f32, normal: Vec3) {
        self.entity = Some(entity);
        self.proximity = proximity;
        self.normal = normal;
    }

    pub fn clear(&mut self) {
        self.entity = None;
        self.proximity = std::f32::INFINITY;
        self.normal = Vec3::ZERO;
    }
}
