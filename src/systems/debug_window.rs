use bevy::prelude::*;
use bevy::camera::RenderTarget;
use bevy::ecs::schedule::ScheduleLabel;
use bevy::window::{WindowRef, WindowResolution};
use bevy_inspector_egui::bevy_egui::EguiMultipassSchedule;

#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct DebugWindowContextPass;

#[derive(Resource)]
pub struct DebugWindowState {
    pub window_entity: Option<Entity>,
    pub camera_entity: Option<Entity>,
    pub visible: bool,
}

impl Default for DebugWindowState {
    fn default() -> Self {
        Self {
            window_entity: None,
            camera_entity: None,
            visible: true,
        }
    }
}

#[derive(Component)]
pub struct DebugWindow;

#[derive(Component)]
pub struct DebugWindowCamera;

pub fn setup_debug_window(
    mut commands: Commands,
    mut state: ResMut<DebugWindowState>,
) {
    let window_id = commands
        .spawn((
            Window {
                title: "Daedalus Debug Panels".to_string(),
                resolution: WindowResolution::new(960, 720),
                position: WindowPosition::Automatic,
                ..default()
            },
            DebugWindow,
        ))
        .id();

    let camera_id = commands
        .spawn((
            EguiMultipassSchedule::new(DebugWindowContextPass),
            Camera2d::default(),
            Camera {
                order: 10,
                ..default()
            },
            RenderTarget::Window(WindowRef::Entity(window_id)),
            DebugWindowCamera,
        ))
        .id();

    state.window_entity = Some(window_id);
    state.camera_entity = Some(camera_id);
    info!("Debug window spawned: window={:?} camera={:?}", window_id, camera_id);
}

pub fn toggle_debug_window(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<DebugWindowState>,
    mut windows: Query<&mut Window, With<DebugWindow>>,
) {
    if keyboard.just_pressed(KeyCode::Backquote) {
        state.visible = !state.visible;
        for mut window in windows.iter_mut() {
            window.visible = state.visible;
        }
    }
}
