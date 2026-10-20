use bevy::prelude::*;
use bevy_inspector_egui::bevy_egui::EguiContexts;
use bevy_inspector_egui::egui;

use crate::components::{Controlled, Infantry, InfantryGimbal};
use super::DebugWindowState;

#[derive(Resource)]
pub struct TelemetryWindowState {
    pub enabled: bool,
}

impl Default for TelemetryWindowState {
    fn default() -> Self {
        Self { enabled: true }
    }
}

pub fn telemetry_window_system(
    mut contexts: EguiContexts,
    gimbal_query: Query<(&Transform, &InfantryGimbal), With<Controlled>>,
    chassis_query: Query<&Transform, (With<Infantry>, With<Controlled>)>,
    mut state: ResMut<TelemetryWindowState>,
    debug_window: Res<DebugWindowState>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    if keyboard.just_pressed(KeyCode::F4) {
        state.enabled = !state.enabled;
    }

    if !state.enabled {
        return;
    }

    let Some(camera_entity) = debug_window.camera_entity else {
        return;
    };
    let Ok(ctx) = contexts.ctx_for_entity_mut(camera_entity) else {
        return;
    };

    egui::Window::new("Vehicle Truth Values")
        .resizable(true)
        .default_width(380.0)
        .default_pos(egui::pos2(500.0, 380.0))
        .show(ctx, |ui| {
            ui.heading("Real-time Truth Table");
            ui.separator();

            egui::Grid::new("truth_table")
                .num_columns(2)
                .spacing([20.0, 4.0])
                .striped(true)
                .show(ui, |ui| {
                    // Chassis world position (always shown first)
                    if let Some(chassis_transform) = chassis_query.iter().next() {
                        let pos = chassis_transform.translation;
                        let (chassis_yaw, _, _) = chassis_transform.rotation.to_euler(EulerRot::YXZ);

                        ui.label("World Position X");
                        ui.monospace(format!("{:.4} m", pos.x));
                        ui.end_row();

                        ui.label("World Position Y");
                        ui.monospace(format!("{:.4} m", pos.y));
                        ui.end_row();

                        ui.label("World Position Z");
                        ui.monospace(format!("{:.4} m", pos.z));
                        ui.end_row();

                        ui.label("Chassis Yaw");
                        ui.monospace(format!("{:.4} rad  ({:.2}°)", chassis_yaw, chassis_yaw.to_degrees()));
                        ui.end_row();
                    }

                    ui.label("");
                    ui.label("");
                    ui.end_row();

                    // Gimbal data
                    if let Some((gimbal_transform, gimbal_data)) = gimbal_query.iter().next() {
                        let (yaw, pitch, _roll) = gimbal_transform.rotation.to_euler(EulerRot::YXZ);

                        ui.label("Gimbal Yaw");
                        ui.monospace(format!("{:.4} rad  ({:.2}°)", yaw, yaw.to_degrees()));
                        ui.end_row();

                        ui.label("Gimbal Pitch");
                        ui.monospace(format!("{:.4} rad  ({:.2}°)", pitch, pitch.to_degrees()));
                        ui.end_row();

                        ui.label("Local Yaw");
                        ui.monospace(format!("{:.4} rad", gimbal_data.local_yaw));
                        ui.end_row();

                        ui.label("Pitch (data)");
                        ui.monospace(format!("{:.4} rad", gimbal_data.pitch));
                        ui.end_row();
                    }
                });

            if gimbal_query.iter().next().is_none() && chassis_query.iter().next().is_none() {
                ui.label("No controlled vehicle found");
            }
        });
}
