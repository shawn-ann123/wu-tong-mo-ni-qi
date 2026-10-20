use bevy::input::mouse::MouseButtonInput;
use bevy::input::ButtonState;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_inspector_egui::bevy_egui::{EguiContexts, egui};

use crate::components::{CameraMode, FollowingType};
use crate::robomaster::prelude::{
    Activation, ManualPowerRuneControlState, MechanismState, PowerRune, PowerRuneMechanism,
    RUNE_TARGET_COUNT, RuneMode, apply_power_rune_control, apply_scene_control_power_rune_state,
};
use crate::setup::{
    AutoAimSceneMode, AutoAimSceneState, ShootingRangeTarget, ShootingRangeTargetKind,
};

use super::range_control::{
    RangeEndpoint, RangeEndpointPick, RangeTargetMotionMode, ShootingRangeControlState,
};
use super::spawn_selection::{SpawnSelectionState, top_down_world_xz};
use super::DebugWindowState;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RuneUiAction {
    SetMode(Option<RuneMode>),
    CycleBlade(usize),
}

pub fn scene_mode_controls_panel(
    mut contexts: EguiContexts,
    scene_state: Res<AutoAimSceneState>,
    mut rune_control: ResMut<ManualPowerRuneControlState>,
    mut runes: Query<(&mut PowerRune, &mut PowerRuneMechanism)>,
    mut range_control: ResMut<ShootingRangeControlState>,
    mut camera_mode: ResMut<CameraMode>,
    targets: Query<&ShootingRangeTarget>,
    spawn_selection: Res<SpawnSelectionState>,
    debug_window: Res<DebugWindowState>,
) {
    let Some(camera_entity) = debug_window.camera_entity else {
        return;
    };
    let Ok(ctx) = contexts.ctx_for_entity_mut(camera_entity) else {
        return;
    };

    let scene = scene_state.current;
    let show_rune = matches!(scene, AutoAimSceneMode::Armor | AutoAimSceneMode::Energy);
    let show_range = matches!(
        scene,
        AutoAimSceneMode::Armor | AutoAimSceneMode::ShootingRange
    );

    let rune_snapshot = runes
        .iter()
        .next()
        .map(|(_, mechanism)| mechanism.state().clone());
    let rune_available = rune_snapshot.is_some();
    let rune_mode = rune_control
        .active_mode()
        .or_else(|| rune_snapshot.as_ref().map(MechanismState::mode));
    let override_enabled = rune_control.override_enabled();
    let endpoint_pick = range_control.endpoint_pick;
    let spawn_selection_active = spawn_selection.active;
    let target_online = |kind: ShootingRangeTargetKind| {
        targets.iter().any(|target| target.kind == kind)
    };

    let mut rune_action: Option<RuneUiAction> = None;
    let mut pick_request: Option<RangeEndpointPick> = None;
    let mut cancel_pick = false;

    egui::Window::new("Mode Controls")
        .resizable(false)
        .collapsible(true)
        .default_pos(egui::pos2(12.0, 160.0))
        .default_width(470.0)
        .show(ctx, |ui| {
            ui.set_min_width(440.0);

            if show_rune {
                rune_controls(
                    ui,
                    rune_snapshot.as_ref(),
                    rune_mode,
                    override_enabled,
                    rune_available,
                    &mut rune_action,
                );
            }

            if show_rune && show_range {
                ui.separator();
            }

            if show_range {
                ui.label(egui::RichText::new("Shooting Range").strong());
                if spawn_selection_active {
                    ui.label("Spawn selection (F1) is active; endpoint picking is disabled.");
                }

                for kind in [ShootingRangeTargetKind::Armor3, ShootingRangeTargetKind::Armor1] {
                    ui.add_space(2.0);
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(kind.label()).strong());
                        ui.monospace(if target_online(kind) {
                            "online"
                        } else {
                            "not in scene"
                        });
                    });

                    let settings = range_control.settings_mut(kind);
                    ui.horizontal(|ui| {
                        ui.label("motion");
                        ui.selectable_value(
                            &mut settings.mode,
                            RangeTargetMotionMode::Stationary,
                            "stop",
                        );
                        ui.selectable_value(
                            &mut settings.mode,
                            RangeTargetMotionMode::Linear,
                            "linear",
                        );
                        ui.selectable_value(
                            &mut settings.mode,
                            RangeTargetMotionMode::Spin,
                            "spin",
                        );
                        ui.selectable_value(
                            &mut settings.mode,
                            RangeTargetMotionMode::LinearAndSpin,
                            "linear+spin",
                        );
                    });
                    ui.add(
                        egui::Slider::new(&mut settings.linear_speed_mps, 0.0..=6.0)
                            .text("linear velocity m/s"),
                    );
                    ui.add(
                        egui::Slider::new(&mut settings.spin_deg_s, -720.0..=720.0)
                            .text("angular velocity deg/s"),
                    );

                    if let Some(pick) = endpoint_pick.filter(|pick| pick.kind == kind) {
                        ui.horizontal(|ui| {
                            ui.label(format!(
                                "picking endpoint {}: click the main window, Esc cancels",
                                endpoint_label(pick.endpoint)
                            ));
                            if ui.small_button("cancel pick").clicked() {
                                cancel_pick = true;
                            }
                        });
                    }

                    for endpoint in [RangeEndpoint::A, RangeEndpoint::B] {
                        let value = match endpoint {
                            RangeEndpoint::A => settings.endpoint_a,
                            RangeEndpoint::B => settings.endpoint_b,
                        };
                        ui.horizontal(|ui| {
                            ui.monospace(format!("{}:", endpoint_label(endpoint)));
                            match value {
                                Some(position) => {
                                    ui.monospace(format!("x {:.2}  z {:.2}", position.x, position.z));
                                }
                                None => {
                                    ui.monospace("not set");
                                }
                            }
                            if ui
                                .add_enabled(
                                    !spawn_selection_active,
                                    egui::Button::new("pick on map"),
                                )
                                .clicked()
                            {
                                pick_request = Some(RangeEndpointPick {
                                    kind,
                                    endpoint,
                                });
                            }
                            if value.is_some() && ui.small_button("clear").clicked() {
                                match endpoint {
                                    RangeEndpoint::A => settings.endpoint_a = None,
                                    RangeEndpoint::B => settings.endpoint_b = None,
                                }
                            }
                        });
                    }

                    match (settings.endpoint_a, settings.endpoint_b) {
                        (Some(a), Some(b)) => {
                            let span = Vec3::new(b.x - a.x, 0.0, b.z - a.z).length();
                            ui.horizontal(|ui| {
                                ui.label(format!(
                                    "A->B span {span:.2} m (endpoints replace direction/span)"
                                ));
                                if ui.small_button("clear path").clicked() {
                                    settings.endpoint_a = None;
                                    settings.endpoint_b = None;
                                }
                            });
                        }
                        (None, None) => {}
                        _ => {
                            ui.label("pick both endpoints to use the endpoint path");
                        }
                    }
                }
            }

            if !show_rune && !show_range {
                ui.label(format!("No mode controls for {}.", scene.label()));
            }
        });

    match rune_action {
        Some(RuneUiAction::SetMode(mode)) => {
            if mode.is_some() {
                apply_power_rune_control(mode, &mut rune_control, &mut runes);
            } else {
                apply_scene_control_power_rune_state(None, &[], &[], &mut rune_control, &mut runes);
            }
        }
        Some(RuneUiAction::CycleBlade(index)) => {
            if let Some(state) = rune_snapshot.as_ref() {
                let mode = rune_control.active_mode().unwrap_or_else(|| state.mode());
                let mut states = state.target_states();
                states[index] = next_blade_activation(states[index]);
                let pending = target_indices(&states, |activation| {
                    activation == Activation::Activating
                });
                let activated = target_indices(&states, |activation| {
                    matches!(
                        activation,
                        Activation::Activated | Activation::Completed
                    )
                });
                apply_scene_control_power_rune_state(
                    Some(mode),
                    &pending,
                    &activated,
                    &mut rune_control,
                    &mut runes,
                );
            }
        }
        None => {}
    }

    if cancel_pick {
        range_control.endpoint_pick = None;
        camera_mode.0 = FollowingType::Robot;
    } else if let Some(pick) = pick_request {
        range_control.endpoint_pick = Some(pick);
        camera_mode.0 = FollowingType::TopDown;
    }
}

pub fn shooting_range_endpoint_pick(
    mut state: ResMut<ShootingRangeControlState>,
    mut camera_mode: ResMut<CameraMode>,
    mut clicks: MessageReader<MouseButtonInput>,
    windows: Query<(Entity, &Window), With<PrimaryWindow>>,
    targets: Query<(&ShootingRangeTarget, &Transform)>,
    spawn_selection: Res<SpawnSelectionState>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    let Some(pick) = state.endpoint_pick else {
        return;
    };

    if spawn_selection.active {
        state.endpoint_pick = None;
        return;
    }

    if keyboard.just_pressed(KeyCode::Escape) {
        state.endpoint_pick = None;
        camera_mode.0 = FollowingType::Robot;
        return;
    }

    if camera_mode.0 != FollowingType::TopDown {
        camera_mode.0 = FollowingType::TopDown;
    }

    let Ok((primary_window, window)) = windows.single() else {
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        return;
    };

    for event in clicks.read() {
        if event.button != bevy::input::mouse::MouseButton::Left
            || event.state != ButtonState::Pressed
            || event.window != primary_window
        {
            continue;
        }

        let ground = top_down_world_xz(cursor, window.size());
        let y = targets
            .iter()
            .find(|(target, _)| target.kind == pick.kind)
            .map(|(_, transform)| transform.translation.y)
            .unwrap_or(1.0);
        state.set_endpoint(pick.kind, pick.endpoint, Vec3::new(ground.x, y, ground.y));
        state.endpoint_pick = None;
        camera_mode.0 = FollowingType::Robot;
        info!(
            "Set {} endpoint {} to ({:.2}, {:.2}).",
            pick.kind.label(),
            endpoint_label(pick.endpoint),
            ground.x,
            ground.y
        );
        break;
    }
}

fn rune_controls(
    ui: &mut egui::Ui,
    snapshot: Option<&MechanismState>,
    mode: Option<RuneMode>,
    override_enabled: bool,
    available: bool,
    action: &mut Option<RuneUiAction>,
) {
    ui.label(egui::RichText::new("Power Rune").strong());
    ui.add_enabled_ui(available, |ui| {
        ui.horizontal(|ui| {
            if ui
                .selectable_label(override_enabled && mode.is_none(), "Off")
                .clicked()
            {
                *action = Some(RuneUiAction::SetMode(None));
            }
            if ui
                .selectable_label(mode == Some(RuneMode::Small), "Small Rune")
                .clicked()
            {
                *action = Some(RuneUiAction::SetMode(Some(RuneMode::Small)));
            }
            if ui
                .selectable_label(mode == Some(RuneMode::Large), "Large Rune")
                .clicked()
            {
                *action = Some(RuneUiAction::SetMode(Some(RuneMode::Large)));
            }
        });

        let Some(snapshot) = snapshot else {
            return;
        };

        ui.horizontal(|ui| {
            ui.label("blades");
            for (index, activation) in snapshot.target_states().iter().enumerate() {
                let (label, fill) = blade_style(*activation);
                let button = egui::Button::new(
                    egui::RichText::new(format!("{index}\n{label}"))
                        .color(egui::Color32::BLACK),
                )
                .fill(fill)
                .min_size(egui::vec2(58.0, 34.0));
                if ui.add(button).clicked() {
                    *action = Some(RuneUiAction::CycleBlade(index));
                }
            }
        });
        ui.horizontal(|ui| {
            ui.label(format!("state: {}", mechanism_label(snapshot)));
            ui.monospace(format!("mode: {}", rune_mode_label(snapshot.mode())));
        });
    });

    if !available {
        ui.label("Power rune is not spawned in this scene.");
    }
}

fn target_indices(
    states: &[Activation; RUNE_TARGET_COUNT],
    predicate: impl Fn(Activation) -> bool,
) -> Vec<usize> {
    states
        .iter()
        .enumerate()
        .filter_map(|(index, activation)| predicate(*activation).then_some(index))
        .collect()
}

fn next_blade_activation(activation: Activation) -> Activation {
    match activation {
        Activation::Deactivated => Activation::Activating,
        Activation::Activating => Activation::Activated,
        Activation::Activated | Activation::Completed => Activation::Deactivated,
    }
}

fn blade_style(activation: Activation) -> (&'static str, egui::Color32) {
    match activation {
        Activation::Deactivated => ("OFF", egui::Color32::from_rgb(122, 128, 134)),
        Activation::Activating => ("PENDING", egui::Color32::from_rgb(230, 170, 40)),
        Activation::Activated => ("ACTIVE", egui::Color32::from_rgb(64, 180, 92)),
        Activation::Completed => ("DONE", egui::Color32::from_rgb(70, 140, 230)),
    }
}

fn mechanism_label(state: &MechanismState) -> &'static str {
    match state {
        MechanismState::Inactive { .. } => "idle",
        MechanismState::Activating(_) => "running",
        MechanismState::Activated { .. } => "activated",
        MechanismState::Failed { .. } => "failed",
    }
}

fn rune_mode_label(mode: RuneMode) -> &'static str {
    match mode {
        RuneMode::Small => "small",
        RuneMode::Large => "large",
    }
}

fn endpoint_label(endpoint: RangeEndpoint) -> &'static str {
    match endpoint {
        RangeEndpoint::A => "A",
        RangeEndpoint::B => "B",
    }
}

#[cfg(test)]
mod tests {
    use super::next_blade_activation;
    use crate::robomaster::prelude::Activation;

    #[test]
    fn blade_button_cycles_through_three_states() {
        assert_eq!(
            next_blade_activation(Activation::Deactivated),
            Activation::Activating
        );
        assert_eq!(
            next_blade_activation(Activation::Activating),
            Activation::Activated
        );
        assert_eq!(
            next_blade_activation(Activation::Activated),
            Activation::Deactivated
        );
        assert_eq!(
            next_blade_activation(Activation::Completed),
            Activation::Deactivated
        );
    }
}
