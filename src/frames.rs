//! Coordinate frame conventions used by the gimbal and camera.
//!
//! ## Gimbal node (云台系)
//!
//! The `GIMBAL` entity uses Bevy convention: +X right, +Y up, +Z backward.
//! At identity rotation the gimbal faces -Z (forward). Movement, muzzle and
//! display follow this convention.
//!
//! | User label | Gimbal node axis | Bevy vec3          |
//! |:-----------|:-----------------|:-------------------|
//! | Forward    | -Z               | `-Vec3::Z`         |
//! | Left       | -X               | `-Vec3::X`         |
//! | Up         | +Y               | `Vec3::Y`          |
//!
//! ## Camera optical frame (相机系)
//!
//! Sensor convention: +X right, +Y down, +Z forward (depth).
//! [`gimbal_from_camera_optical`] is the camera-to-gimbal extrinsic. The Bevy
//! camera entity has +X right / +Y up and views along -Z, differing from the
//! optical frame by a 180° rotation about X; composing them gives
//! [`camera_from_gimbal`], which is identity in this model. The optical axis
//! therefore always follows `GIMBAL_FORWARD`.

use bevy::prelude::*;

/// Forward direction in the gimbal node frame (Bevy -Z).
pub const GIMBAL_FORWARD: Vec3 = Vec3::NEG_Z;

/// Left direction in the gimbal node frame (Bevy -X).
pub const GIMBAL_LEFT: Vec3 = Vec3::NEG_X;

/// Up direction in the gimbal node frame (Bevy +Y).
pub const GIMBAL_UP: Vec3 = Vec3::Y;

/// Rotation mapping vectors from the camera optical frame to the Bevy gimbal node.
///
/// Logical gimbal frame: +X forward, +Y left, +Z up.
/// Bevy gimbal node: +X right, +Y up, +Z backward.
/// Camera optical frame: +X right, +Y down, +Z forward.
///
/// Expressed in gimbal-node axes the columns are
/// `[right, down, forward] = [+X, -Y, -Z]`, i.e. a 180° rotation about X:
/// optical forward (+Z_c) maps to `GIMBAL_FORWARD` (-Z_g), optical down (+Y_c)
/// maps to `-GIMBAL_UP`, and optical right (+X_c) maps to `-GIMBAL_LEFT`.
pub fn gimbal_from_camera_optical() -> Quat {
    Quat::from_mat3(&Mat3::from_cols(
        -GIMBAL_LEFT,   // camera +X (right)
        -GIMBAL_UP,     // camera +Y (down)
        GIMBAL_FORWARD, // camera +Z (forward)
    ))
}

/// Rotation from the *gimbal node* frame to the *Bevy Camera entity* frame.
///
/// The Bevy camera has +X right / +Y up and views along -Z, while the optical
/// frame has +Y down and +Z forward. The two differ by a 180° rotation about X,
/// so composing the optical extrinsic with that flip yields identity in this
/// model: `camera_transform.rotation = gimbal_world_rotation`.
pub fn camera_from_gimbal() -> Quat {
    gimbal_from_camera_optical() * Quat::from_rotation_x(std::f32::consts::PI)
}

/// Camera lens offset in the gimbal node frame — position of the camera
/// sensor relative to the gimbal origin.  Default matches `CAM_DIRECTION` in
/// `vehicle.glb`.  Override via `ballistics.yaml` camera_offset.
pub const DEFAULT_CAMERA_OFFSET_GIMBAL: Vec3 = Vec3::new(-0.002, 0.196, -0.06);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gimbal_forward_is_neg_z() {
        assert_eq!(GIMBAL_FORWARD, Vec3::NEG_Z);
        assert_eq!(GIMBAL_LEFT, Vec3::NEG_X);
        assert_eq!(GIMBAL_UP, Vec3::Y);
    }

    #[test]
    fn camera_from_gimbal_looks_along_gimbal_forward() {
        let q = camera_from_gimbal();
        // Camera looks along its local -Z -> should map to GIMBAL_FORWARD (-Z_g)
        let look_dir = q * Vec3::NEG_Z;
        assert!(look_dir.distance(GIMBAL_FORWARD) < 1e-5);
        // Camera up (+Y_c) should map to GIMBAL_UP
        let up_dir = q * Vec3::Y;
        assert!(up_dir.distance(GIMBAL_UP) < 1e-5);
        // Camera right (+X_c) should map to -GIMBAL_LEFT (+X_g)
        let right_dir = q * Vec3::X;
        assert!(right_dir.distance(Vec3::X) < 1e-5);
    }

    #[test]
    fn optical_extrinsic_maps_forward_down_and_right() {
        let q = gimbal_from_camera_optical();
        // Optical forward (+Z) -> gimbal forward.
        assert!((q * Vec3::Z).distance(GIMBAL_FORWARD) < 1e-5);
        // Optical down (+Y) -> gimbal down (-up).
        assert!((q * Vec3::Y).distance(-GIMBAL_UP) < 1e-5);
        // Optical right (+X) -> gimbal right (-left).
        assert!((q * Vec3::X).distance(-GIMBAL_LEFT) < 1e-5);
    }

    #[test]
    fn frames_are_right_handed() {
        let cross = GIMBAL_FORWARD.cross(GIMBAL_LEFT);
        assert!(cross.distance(GIMBAL_UP) < 1e-5);
    }
}
