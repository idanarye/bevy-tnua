use bevy::prelude::*;
use bevy_tnua::{TnuaGhostSensor, TnuaObstacleRadar, TnuaProximitySensor};

use crate::ui::info::InfoSource;

pub fn character_control_info_dumping_system(
    mut query: Query<(
        &mut InfoSource,
        &TnuaProximitySensor,
        Option<&TnuaGhostSensor>,
        Option<&TnuaObstacleRadar>,
    )>,
    names_query: Query<&Name>,
) {
    for (mut info_source, sensor, ghost_sensor, obstacle_radar) in query.iter_mut() {
        if !info_source.is_active() {
            continue;
        }
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
                .blips
                .iter()
                .map(|(entity, blip)| {
                    let name = names_query
                        .get(*entity)
                        .ok()
                        .map(|name| name.to_string())
                        .unwrap_or_else(|| format!("{entity}"));
                    format!(
                        "{}\n\t{}\n\t{} to top\n\t{} to bottom",
                        name, blip.position, blip.to_top, blip.to_bottom
                    )
                })
                .collect::<Vec<_>>();
            obstacles.sort();
            info_source.label("Obstacle radar", obstacles.join("\n"));
        }
    }
}
