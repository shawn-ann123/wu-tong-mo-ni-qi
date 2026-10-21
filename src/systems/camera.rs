use bevy::input::mouse::MouseMotion;
use bevy::prelude::*;

use crate::ballistics_config::BallisticsConfig;
use crate::components::{
    CameraMode, Controlled, FollowingType, Infantry, InfantryGimbal, MainCamera,
};
use crate::config::SimulationConfig;
use crate::frames::camera_from_gimbal;

pub fn following_controls(mut mode: ResMut<CameraMode>, keyboard: Res<ButtonInput<KeyCode>>) {
    if keyboard.just_pressed(KeyCode::F3) {
        mode.0 = match mode.0 {
            FollowingType::Free => FollowingType::Robot,
            FollowingType::Robot => FollowingType::ThirdPerson,
            FollowingType::ThirdPerson => FollowingType::TopDown,
            FollowingType::TopDown => FollowingType::Free,
        };
    }
}

pub fn update_camera_follow(
    camera_query: Single<(&mut Transform, &MainCamera), Without<Controlled>>,
    infantry: Single<&Transform, (With<Infantry>, With<Controlled>)>,
    gimbal: Single<&Transform, (With<Controlled>, With<InfantryGimbal>)>,
    ballistics: Res<BallisticsConfig>,
    mode: Res<CameraMode>,
) {
    let gimbal_transform = gimbal.into_inner();
    let (mut camera_transform, camera_offset) = camera_query.into_inner();

    match mode.0 {
        FollowingType::Robot => {
            // Camera-to-gimbal extrinsic: the camera is mounted on the gimbal.
            // Rotation maps the gimbal frame to the Bevy camera frame so the
            // optical axis (+Z optical) follows logical gimbal +X, represented
            // by -Z in the Bevy gimbal node.
            let gimbal_world_rotation = infantry.rotation * gimbal_transform.rotation;
            let cam_offset_gimbal = Vec3::from_array(ballistics.camera_offset);
            camera_transform.translation =
                infantry.translation + gimbal_world_rotation * cam_offset_gimbal;
            camera_transform.rotation = gimbal_world_rotation * camera_from_gimbal();
        }
        FollowingType::ThirdPerson => {
            let base_transform = infantry.into_inner();
            let offset = base_transform.rotation * camera_offset.follow_offset;
            camera_transform.translation = base_transform.translation + offset;
            camera_transform.look_at(base_transform.translation, Vec3::Y);
        }
        FollowingType::Free | FollowingType::TopDown => {}
    }
}

pub fn freecam_controls(
    time: Res<Time>,
    mode: Res<CameraMode>,
    config: Res<SimulationConfig>,
    mut mouse_motion_events: MessageReader<MouseMotion>,
    keyboard: Res<ButtonInput<KeyCode>>,
    camera_query: Single<&mut Transform, (With<MainCamera>, Without<Infantry>)>,
) {
    if mode.0 != FollowingType::Free {
        return;
    }

    let delta = time.delta_secs();
    let mut camera_transform = camera_query.into_inner();

    let mut mouse_delta = Vec2::ZERO;
    for event in mouse_motion_events.read() {
        mouse_delta += event.delta;
    }

    if mouse_delta != Vec2::ZERO {
        let (yaw, pitch, roll) = camera_transform.rotation.to_euler(EulerRot::YXZ);

        let new_yaw = yaw - mouse_delta.x * config.camera.mouse_sensitivity;
        let new_pitch = (pitch - mouse_delta.y * config.camera.mouse_sensitivity).clamp(-1.4, 1.4);

        camera_transform.rotation = Quat::from_euler(EulerRot::YXZ, new_yaw, new_pitch, roll);
    }

    let speed = config.camera.free_move_speed * delta;
    let forward = camera_transform.forward();
    let right = camera_transform.right();
    let up = camera_transform.up();

    if keyboard.pressed(KeyCode::KeyW) {
        camera_transform.translation += forward * speed;
    }
    if keyboard.pressed(KeyCode::KeyS) {
        camera_transform.translation -= forward * speed;
    }
    if keyboard.pressed(KeyCode::KeyA) {
        camera_transform.translation -= right * speed;
    }
    if keyboard.pressed(KeyCode::KeyD) {
        camera_transform.translation += right * speed;
    }
    if keyboard.pressed(KeyCode::KeyN) {
        camera_transform.translation += up * speed;
    }
    if keyboard.pressed(KeyCode::KeyJ) {
        camera_transform.translation -= up * speed;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::robomaster::prelude::{INFANTRY_THREE_CONFIG, Team};

    #[test]
    fn robot_camera_follows_gimbal_forward_with_configured_offset() {
        let mut app = App::new();
        app.insert_resource(BallisticsConfig::default())
            .insert_resource(CameraMode(FollowingType::Robot))
            .add_systems(Update, update_camera_follow);

        let player_translation = Vec3::new(10.0, 0.0, -2.0);
        let player_rotation = Quat::from_rotation_y(0.5);
        let gimbal_local_rotation = Quat::from_euler(EulerRot::YXZ, 0.3, 0.2, 0.0);

        app.world_mut().spawn((
            Infantry::new(Team::Red, INFANTRY_THREE_CONFIG),
            Controlled,
            Transform::from_translation(player_translation).with_rotation(player_rotation),
        ));
        app.world_mut().spawn((
            Controlled,
            InfantryGimbal::default(),
            Transform::from_rotation(gimbal_local_rotation),
        ));
        app.world_mut().spawn((
            MainCamera {
                follow_offset: Vec3::new(0.0, 3.0, 2.0),
            },
            Transform::default(),
        ));
        app.update();

        let gimbal_world_rotation = player_rotation * gimbal_local_rotation;
        let offset = Vec3::from_array(BallisticsConfig::default().camera_offset);

        let mut query = app
            .world_mut()
            .query_filtered::<&Transform, With<MainCamera>>();
        let camera = query.single(app.world()).expect("camera exists");

        let expected_translation = player_translation + gimbal_world_rotation * offset;
        assert!(camera.translation.distance(expected_translation) < 1e-5);

        // Optical axis (Bevy camera forward = -Z) follows the gimbal forward.
        let expected_forward = (gimbal_world_rotation * crate::frames::GIMBAL_FORWARD).normalize();
        assert!(camera.forward().distance(expected_forward) < 1e-5);
        // Screen up follows the gimbal up.
        let expected_up = (gimbal_world_rotation * crate::frames::GIMBAL_UP).normalize();
        assert!(camera.up().distance(expected_up) < 1e-5);
    }
}
