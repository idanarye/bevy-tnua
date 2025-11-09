use bevy::{color::palettes::css, prelude::*};
use bevy_tnua::prelude::TnuaController;
use bevy_tnua::{
    math::AsF32, radar_lens::TnuaRadarLens, TnuaGhostSensor, TnuaObstacleRadar, TnuaProximitySensor,
};

use crate::ui::info::InfoSource;

use super::spatial_ext_facade::SpatialExtFacade;

#[allow(clippy::type_complexity)]
pub fn character_control_info_dumping_system(
    mut query: Query<(
        &mut InfoSource,
        &TnuaController,
        &TnuaProximitySensor,
        Option<&TnuaGhostSensor>,
        Option<&TnuaObstacleRadar>,
    )>,
    names_query: Query<&Name>,
) {
    for (mut info_source, controller, sensor, ghost_sensor, obstacle_radar) in query.iter_mut() {
        if !info_source.is_active() {
            continue;
        }
        info_source.label("Action", controller.action_name().unwrap_or_default());
        if let Some(sensor_output) = sensor.output.as_ref() {
            if let Ok(name) = names_query.get(sensor_output.entity) {
                info_source.label("Standing on", name.as_str());
            } else {
                info_source.label("Standing on", format!("{:?}", sensor_output.entity));
            }
        } else {
            info_source.label("Standing on", "<Nothing>");
        }
        if let Some(ghost_sensor) = ghost_sensor.as_ref() {
            let mut text = String::new();
            for hit in ghost_sensor.iter() {
                if !text.is_empty() {
                    text.push_str(", ");
                }
                if let Ok(name) = names_query.get(hit.entity) {
                    text.push_str(name.as_str());
                } else {
                    text.push_str(&format!("{:?}", hit.entity));
                }
            }
            info_source.label("Ghost sensor", text);
        }
        if let Some(obstacle_radar) = obstacle_radar.as_ref() {
            let mut obstacles = obstacle_radar
                .iter_blips()
                .map(|entity| {
                    names_query
                        .get(entity)
                        .ok()
                        .map(|name| name.to_string())
                        .unwrap_or_else(|| format!("{entity}"))
                })
                .collect::<Vec<_>>();
            obstacles.sort();
            info_source.label("Obstacle radar", obstacles.join("\n"));
        }
    }
}

pub fn character_control_radar_visualization_system(
    query: Query<&TnuaObstacleRadar>,
    spatial_ext: SpatialExtFacade,
    mut gizmos: Gizmos,
) {
    if true {
        // Don't show the gizmos
        return;
    }
    for obstacle_radar in query.iter() {
        let radar_lens = TnuaRadarLens::new(obstacle_radar, &spatial_ext);
        for blip in radar_lens.iter_blips() {
            let closest_point = blip.closest_point().get();
            gizmos.arrow(
                obstacle_radar.tracked_position().f32(),
                closest_point.f32(),
                css::PALE_VIOLETRED,
            );
        }
    }
}
