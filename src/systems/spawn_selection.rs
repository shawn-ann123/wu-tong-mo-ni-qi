use bevy::prelude::*;
use bevy::window::{CursorMoved, PrimaryWindow};
use bevy::input::mouse::MouseButtonInput;
use bevy::input::ButtonState;
use bevy_inspector_egui::bevy_egui::EguiContexts;
use bevy_inspector_egui::egui;

use crate::components::{CameraMode, Controlled, FollowingType, Infantry};
use super::DebugWindowState;

const TOP_DOWN_CAMERA_HEIGHT: f32 = 30.0;
const DEFAULT_SPAWN_Y: f32 = 0.5;

#[derive(Resource, Default)]
pub struct SpawnSelectionState {
    pub active: bool,
    pub pending_spawn: Option<Vec3>,
    pub input_x: String,
    pub input_y: String,
    pub input_z: String,
    pub click_to_apply: Option<Vec3>,
    pub last_cursor_pos: Option<Vec2>,
    pub primary_window_entity: Option<Entity>,
}

#[derive(Component)]
pub struct TopDownCamera;

/// Cache the primary window entity on startup
pub fn spawn_selection_init(
    windows: Query<Entity, With<PrimaryWindow>>,
    mut state: ResMut<SpawnSelectionState>,
) {
    if state.primary_window_entity.is_none() {
        if let Ok(entity) = windows.single() {
            state.primary_window_entity = Some(entity);
        }
    }
}

/// Track cursor position on the primary window via events
pub fn spawn_cursor_tracking(
    mut state: ResMut<SpawnSelectionState>,
    mut cursor_events: MessageReader<CursorMoved>,
) {
    let Some(primary_window) = state.primary_window_entity else {
        return;
    };
    for event in cursor_events.read() {
        if event.window == primary_window {
            state.last_cursor_pos = Some(event.position);
        }
    }
}

/// Detect mouse clicks on the primary window via events
pub fn spawn_click_detection(
    mut state: ResMut<SpawnSelectionState>,
    mut camera_mode: ResMut<CameraMode>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut click_events: MessageReader<MouseButtonInput>,
    windows: Query<&Window, With<PrimaryWindow>>,
) {
    if keyboard.just_pressed(KeyCode::F1) {
        state.active = !state.active;
        if state.active {
            camera_mode.0 = FollowingType::TopDown;
        } else {
            camera_mode.0 = FollowingType::Robot;
        }
    }

    if !state.active {
        return;
    }

    let Some(primary_window) = state.primary_window_entity else {
        return;
    };

    for event in click_events.read() {
        if event.button == bevy::input::mouse::MouseButton::Left
            && event.state == ButtonState::Pressed
            && event.window == primary_window
        {
            info!("Spawn click detected on primary window!");
            if let Some(cursor_pos) = state.last_cursor_pos {                let Ok(window) = windows.single() else {
                    return;
                };
                let window_size = window.size();
                let ndc_x = (cursor_pos.x / window_size.x) * 2.0 - 1.0;
                let ndc_y = 1.0 - (cursor_pos.y / window_size.y) * 2.0;

                let fov_rad = 45.0_f32.to_radians();
                let aspect = window_size.x / window_size.y;
                let half_h = TOP_DOWN_CAMERA_HEIGHT * (fov_rad / 2.0).tan();
                let half_w = half_h * aspect;

                let world_x = ndc_x * half_w;
                let world_z = -ndc_y * half_h;

                state.click_to_apply = Some(Vec3::new(world_x, DEFAULT_SPAWN_Y, world_z));
            }
        }
    }
}

/// Runs in DebugWindowContextPass — shows egui panel and applies pending spawns
pub fn spawn_selection_ui(
    mut contexts: EguiContexts,
    mut state: ResMut<SpawnSelectionState>,
    mut camera_mode: ResMut<CameraMode>,
    mut infantry_query: Query<&mut Transform, (With<Infantry>, With<Controlled>)>,
    debug_window: Res<DebugWindowState>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    if !state.active {
        return;
    }

    let Some(camera_entity) = debug_window.camera_entity else {
        return;
    };
    let Ok(ctx) = contexts.ctx_for_entity_mut(camera_entity) else {
        return;
    };

    if let Some(pos) = state.click_to_apply.take() {
        state.pending_spawn = Some(pos);
        state.input_x = format!("{:.2}", pos.x);
        state.input_y = format!("{:.2}", pos.y);
        state.input_z = format!("{:.2}", pos.z);
        if let Ok(mut transform) = infantry_query.single_mut() {
            transform.translation = pos;
        }
    }

    egui::Window::new("Spawn Point Selection")
        .resizable(false)
        .default_width(300.0)
        .show(ctx, |ui| {
            ui.heading("Click on main window to place, or enter coordinates");
            ui.separator();

            ui.horizontal(|ui| {
                ui.label("X:");
                ui.text_edit_singleline(&mut state.input_x);
            });
            ui.horizontal(|ui| {
                ui.label("Y:");
                ui.text_edit_singleline(&mut state.input_y);
            });
            ui.horizontal(|ui| {
                ui.label("Z:");
                ui.text_edit_singleline(&mut state.input_z);
            });

            ui.separator();

            if ui.button("Apply Coordinates").clicked() {
                if let (Ok(x), Ok(y), Ok(z)) = (
                    state.input_x.parse::<f32>(),
                    state.input_y.parse::<f32>(),
                    state.input_z.parse::<f32>(),
                ) {
                    let pos = Vec3::new(x, y, z);
                    state.pending_spawn = Some(pos);
                    if let Ok(mut transform) = infantry_query.single_mut() {
                        transform.translation = pos;
                    }
                }
            }

            if ui.button("Close (Esc)").clicked() || keyboard.just_pressed(KeyCode::Escape) {
                state.active = false;
                camera_mode.0 = FollowingType::Robot;
            }

            if let Some(pos) = state.pending_spawn {
                ui.separator();
                ui.label(format!("Current: ({:.2}, {:.2}, {:.2})", pos.x, pos.y, pos.z));
            }
        });
}

pub fn apply_spawn_position(
    mut state: ResMut<SpawnSelectionState>,
    mut infantry_query: Query<&mut Transform, (With<Infantry>, With<Controlled>)>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    if keyboard.just_pressed(KeyCode::Enter) {
        if let Some(pos) = state.pending_spawn.take() {
            if let Ok(mut transform) = infantry_query.single_mut() {
                transform.translation = pos;
                state.active = false;
            }
        }
    }
}

pub fn top_down_camera_system(
    mode: Res<CameraMode>,
    mut camera_query: Query<&mut Transform, (With<crate::components::MainCamera>, Without<Infantry>)>,
) {
    if mode.0 != FollowingType::TopDown {
        return;
    }

    let Ok(mut camera_transform) = camera_query.single_mut() else {
        return;
    };

    camera_transform.translation = Vec3::new(0.0, TOP_DOWN_CAMERA_HEIGHT, 0.0);
    camera_transform.rotation = Quat::from_euler(EulerRot::XZY, -std::f32::consts::FRAC_PI_2, 0.0, 0.0);
}
