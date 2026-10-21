use avian3d::prelude::*;
use bevy::prelude::*;
use core::f32::consts::PI;

use crate::ballistics_config::BallisticsConfig;
use crate::components::{
    Controlled, DartLaunch, DartProjectile, DartSetting, GameLayer, Infantry, InfantryChassis,
    InfantryGimbal, ProjectileCooldown, ProjectileLifetime, ProjectileSetting,
};
use crate::config::SimulationConfig;
use crate::frames::GIMBAL_FORWARD;
use crate::robomaster::prelude::{Armor, ArmorRoot, Projectile};
use crate::statistic::ProjectileStatistics;
use crate::telemetry::{
    PendingAutoAimShotContext, ProjectileImpactRecorded, ProjectileKinematics, ProjectileTelemetry,
    ProjectileTrace, ShotSource, nearest_armor_target, nearest_outpost_armor_target,
};

pub fn setup_projectile(
    mut commands: Commands,
    ballistics: Res<BallisticsConfig>,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(ProjectileSetting(
        meshes.add(Sphere::new(ballistics.projectile_diameter_m / 2.0)),
        materials.add(StandardMaterial {
            base_color: Color::srgba(0.132866, 1.0, 0.132869, 0.85),
            emissive: LinearRgba::new(0.132866, 1.0, 0.132869, 0.85),
            emissive_exposure_weight: -1.0,
            alpha_mode: AlphaMode::Opaque,
            ..default()
        }),
    ));
    commands.insert_resource(DartSetting(
        asset_server.load(GltfAssetLabel::Scene(0).from_asset("DART.glb")),
    ));
}

pub fn projectile_launch(
    time: Res<Time>,
    mut cooldown: ResMut<ProjectileCooldown>,
    mut stats: ResMut<ProjectileStatistics>,
    mut telemetry: ResMut<ProjectileTelemetry>,
    pending_auto_aim: Option<Res<PendingAutoAimShotContext>>,
    config: Res<SimulationConfig>,
    ballistics: Res<BallisticsConfig>,
    mut commands: Commands,
    setting: Res<ProjectileSetting>,
    infantry: Single<(&LinearVelocity, &AngularVelocity), (With<Infantry>, With<Controlled>)>,
    gimbal: Single<
        (&GlobalTransform, &InfantryGimbal),
        (With<Controlled>, Without<InfantryChassis>),
    >,
) {
    cooldown.tick(time.delta());
    if !cooldown.is_finished() {
        if pending_auto_aim.is_some() {
            commands.remove_resource::<PendingAutoAimShotContext>();
        }
        return;
    }
    cooldown.reset();

    let auto_aim = pending_auto_aim.as_deref().copied();
    if auto_aim.is_some() {
        commands.remove_resource::<PendingAutoAimShotContext>();
    }

    // Muzzle velocity follows the gimbal frame +X (forward). In the Bevy-aligned
    // gimbal node frame that is -Z, exposed as GIMBAL_FORWARD.
    let direction = gimbal
        .0
        .rotation()
        .mul_vec3(GIMBAL_FORWARD)
        .normalize_or_zero();
    if direction == Vec3::ZERO {
        return;
    }
    let vel = infantry.0.0 + direction * ballistics.muzzle_speed_mps;
    // Spawn at the gimbal origin, not the camera optical axis.
    let position = gimbal.0.translation();
    let (gimbal_yaw, gimbal_pitch, _) = gimbal.0.rotation().to_euler(EulerRot::YXZ);
    stats.increase_launch();
    let trace = ProjectileTrace {
        id: telemetry.next_projectile_id(),
        source: if auto_aim.is_some() {
            ShotSource::AutoAim
        } else {
            ShotSource::Manual
        },
        target_mode: std::env::var("DAEDALUS_AUTO_AIM_MODE").unwrap_or_else(|_| "armor".into()),
        launch_index: stats.launch_count,
        launched_at_s: time.elapsed_secs_f64(),
        position_m: position,
        velocity_mps: vel,
        direction,
        gimbal_yaw_deg: gimbal_yaw.to_degrees(),
        gimbal_pitch_deg: gimbal_pitch.to_degrees(),
        auto_aim,
    };
    telemetry.write_launch(&trace);
    commands.spawn((
        RigidBody::Dynamic,
        Collider::sphere(ballistics.projectile_diameter_m / 2.0),
        Mass(ballistics.projectile_mass_kg),
        Friction::new(config.projectile.friction),
        Restitution::new(0.3),
        LinearDamping(config.projectile.linear_damping),
        GameLayer::projectile_collision_layers(true),
        Mesh3d(setting.0.clone()),
        MeshMaterial3d(setting.1.clone()),
        LinearVelocity(vel),
        AngularVelocity(infantry.1.0),
        Transform::IDENTITY.with_translation(position),
        ProjectileLifetime(Timer::from_seconds(
            config.projectile.lifetime,
            TimerMode::Once,
        )),
        Projectile,
        trace,
    ));
    info!("Projectile launched; total launches={}", stats.launch_count);
}

pub fn projectile_aerodynamics(
    config: Res<SimulationConfig>,
    ballistics: Res<BallisticsConfig>,
    mut projectiles: Query<Forces, (With<Projectile>, Without<DartProjectile>)>,
) {
    let aero = &config.projectile.aerodynamics;
    if !aero.enabled {
        return;
    }

    let diameter = ballistics.projectile_diameter_m;
    if diameter <= 0.0 {
        return;
    }
    let air_density = aero.air_density.max(0.0);
    let drag_coefficient = aero.drag_coefficient.max(0.0);
    if air_density == 0.0 || drag_coefficient == 0.0 {
        return;
    }

    let area = PI * (diameter * 0.5).powi(2);
    let wind = Vec3::new(aero.wind[0], aero.wind[1], aero.wind[2]);
    let k = 0.5 * air_density * drag_coefficient * area;

    for mut forces in projectiles.iter_mut() {
        let v_rel = forces.linear_velocity() - wind;
        let speed = v_rel.length();
        if speed <= 1e-3 {
            continue;
        }
        forces.apply_force(-k * speed * v_rel);
    }
}

pub fn dart_launch(
    mut commands: Commands,
    config: Res<SimulationConfig>,
    ballistics: Res<BallisticsConfig>,
    mut stats: ResMut<ProjectileStatistics>,
    setting: Res<DartSetting>,
    launchers: Query<&GlobalTransform, With<DartLaunch>>,
) {
    info!("Dart launch requested");
    const DART_FORWARD: Vec3 = Vec3::Y;
    const DART_MODEL_FORWARD: Vec3 = Vec3::NEG_Y;
    const DART_SPAWN_OFFSET_M: f32 = 0.00;

    let Ok(launcher) = launchers.single() else {
        info!("Dart launch skipped: expected exactly one launcher");
        return;
    };

    let direction = launcher
        .rotation()
        .mul_vec3(DART_FORWARD)
        .normalize_or_zero();
    if direction == Vec3::ZERO {
        return;
    }

    stats.increase_launch();

    let transform =
        Transform::from_translation(launcher.translation() + direction * DART_SPAWN_OFFSET_M)
            .with_rotation(
                launcher.rotation() * Quat::from_rotation_arc(DART_MODEL_FORWARD, DART_FORWARD),
            );
    let voxel = |size| {
        ColliderConstructorHierarchy::new(ColliderConstructor::VoxelizedTrimeshFromMesh {
            voxel_size: size,
            fill_mode: FillMode::FloodFill {
                detect_cavities: true,
            },
        })
        .with_default_layers(GameLayer::projectile_collision_layers(true))
    };
    commands.spawn((
        RigidBody::Dynamic,
        voxel(0.005),
        Mass(ballistics.dart_mass_kg),
        Friction::new(config.projectile.friction),
        Restitution::new(0.55),
        LinearDamping(config.projectile.linear_damping),
        GameLayer::projectile_collision_layers(true),
        WorldAssetRoot(setting.0.clone()),
        transform,
        LinearVelocity(direction * ballistics.dart_speed_mps),
        ProjectileLifetime(Timer::from_seconds(
            config.projectile.lifetime,
            TimerMode::Once,
        )),
        Projectile,
        DartProjectile,
    ));
    info!("Dart launched; total launches={}", stats.launch_count);
}

pub fn cleanup_projectiles(
    time: Res<Time>,
    mut commands: Commands,
    telemetry: Res<ProjectileTelemetry>,
    mut projectiles: Query<(
        Entity,
        &mut ProjectileLifetime,
        Option<&Transform>,
        Option<&LinearVelocity>,
        Option<&ProjectileTrace>,
        Option<&ProjectileImpactRecorded>,
    )>,
    armors: Query<(Entity, &Armor, &GlobalTransform), With<ArmorRoot>>,
) {
    for (entity, mut lifetime, transform, velocity, trace, impact_recorded) in &mut projectiles {
        lifetime.tick(time.delta());
        if lifetime.is_finished() {
            if let Some(trace) = trace
                && impact_recorded.is_none()
            {
                let projectile = ProjectileKinematics::from_components(transform, velocity);
                let nearest = nearest_armor_target(projectile.position_m, &armors);
                let nearest_outpost = nearest_outpost_armor_target(projectile.position_m, &armors);
                telemetry.write_expired(trace, projectile, nearest, nearest_outpost);
            }
            commands.entity(entity).despawn();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::robomaster::prelude::{INFANTRY_THREE_CONFIG, Team};

    #[test]
    fn projectile_spawns_at_gimbal_origin_along_gimbal_forward() {
        let mut app = App::new();
        app.insert_resource(SimulationConfig::default())
            .insert_resource(BallisticsConfig::default())
            .insert_resource(ProjectileStatistics::default())
            .insert_resource(ProjectileTelemetry::from_env())
            .insert_resource(ProjectileSetting(
                Handle::<Mesh>::default(),
                Handle::<StandardMaterial>::default(),
            ))
            .insert_resource(ProjectileCooldown(Timer::from_seconds(
                0.0,
                TimerMode::Once,
            )))
            .init_resource::<Time>()
            .add_systems(Update, projectile_launch);

        let gimbal_translation = Vec3::new(1.0, 2.0, 3.0);
        let gimbal_rotation = Quat::from_euler(EulerRot::YXZ, 0.3, 0.2, 0.0);

        app.world_mut().spawn((
            Infantry::new(Team::Red, INFANTRY_THREE_CONFIG),
            Controlled,
            LinearVelocity(Vec3::ZERO),
            AngularVelocity(Vec3::ZERO),
        ));
        app.world_mut().spawn((
            Controlled,
            InfantryGimbal::default(),
            Transform::from_translation(gimbal_translation).with_rotation(gimbal_rotation),
            GlobalTransform::from(
                Transform::from_translation(gimbal_translation).with_rotation(gimbal_rotation),
            ),
        ));
        app.update();

        let mut query = app
            .world_mut()
            .query_filtered::<(&Transform, &LinearVelocity), With<Projectile>>();
        let projectiles: Vec<_> = query
            .iter(app.world())
            .map(|(transform, velocity)| (transform.translation, velocity.0))
            .collect();
        assert_eq!(projectiles.len(), 1);
        let (position, velocity) = projectiles[0];

        // Muzzle at the gimbal origin.
        assert!(position.distance(gimbal_translation) < 1e-5);
        // Muzzle direction along the gimbal forward axis (-Z in the node frame).
        let expected_direction = (gimbal_rotation * crate::frames::GIMBAL_FORWARD).normalize();
        let actual_direction = velocity.normalize();
        assert!(
            actual_direction.distance(expected_direction) < 1e-5,
            "direction={actual_direction:?} expected={expected_direction:?}"
        );
        assert!((velocity.length() - BallisticsConfig::default().muzzle_speed_mps).abs() < 1e-4);
    }

    #[test]
    fn dart_launch_spawns_dart_projectile() {
        let mut app = App::new();
        app.insert_resource(SimulationConfig::default())
            .insert_resource(BallisticsConfig::default())
            .insert_resource(ProjectileStatistics::default())
            .insert_resource(DartSetting(Handle::<WorldAsset>::default()))
            .add_systems(Update, dart_launch);

        app.world_mut()
            .spawn((DartLaunch, GlobalTransform::IDENTITY));
        app.update();

        let mut dart_query = app
            .world_mut()
            .query_filtered::<Entity, With<DartProjectile>>();
        assert_eq!(dart_query.iter(app.world()).count(), 1);
        assert_eq!(
            app.world().resource::<ProjectileStatistics>().launch_count,
            1
        );
    }
}
