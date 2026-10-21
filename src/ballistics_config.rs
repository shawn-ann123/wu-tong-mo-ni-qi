use bevy::prelude::*;
use crossbeam_channel::{Receiver, unbounded};
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Resource, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct BallisticsConfig {
    /// 17 mm projectile muzzle velocity (m/s).
    pub muzzle_speed_mps: f32,
    /// 17 mm projectile mass (kg).
    pub projectile_mass_kg: f32,
    /// 17 mm projectile diameter (m).
    pub projectile_diameter_m: f32,
    /// Dart launch velocity (m/s).
    pub dart_speed_mps: f32,
    /// Dart mass (kg).
    pub dart_mass_kg: f32,
    /// Camera sensor offset in the gimbal-node frame (m).
    pub camera_offset: [f32; 3],
}

impl Default for BallisticsConfig {
    fn default() -> Self {
        Self {
            muzzle_speed_mps: 25.0,
            projectile_mass_kg: 0.0032,
            projectile_diameter_m: 0.017,
            dart_speed_mps: 17.0,
            dart_mass_kg: 0.25,
            camera_offset: [-0.002, 0.196, -0.06],
        }
    }
}

#[derive(Resource)]
struct BallisticsConfigWatcher {
    _watcher: RecommendedWatcher,
    receiver: Receiver<Result<Event, notify::Error>>,
}

pub struct BallisticsConfigPlugin;

impl Plugin for BallisticsConfigPlugin {
    fn build(&self, app: &mut App) {
        let config = BallisticsConfig::load().unwrap_or_default();

        // Hot-reload watcher
        let (tx, rx) = unbounded();
        let watcher_result = RecommendedWatcher::new(
            move |res| {
                let _ = tx.send(res);
            },
            notify::Config::default(),
        );

        if let Ok(mut watcher) = watcher_result {
            let path = ballistics_path();
            if let Err(e) = watcher.watch(path.as_path(), RecursiveMode::NonRecursive) {
                warn!("ballistics.yaml watch error: {}", e);
            } else {
                info!("ballistics.yaml hot-reload enabled: {}", path.display());
                app.insert_resource(BallisticsConfigWatcher {
                    _watcher: watcher,
                    receiver: rx,
                });
                app.add_systems(Update, ballistics_hot_reload);
            }
        }

        app.insert_resource(config);
    }
}

fn ballistics_hot_reload(
    mut config: ResMut<BallisticsConfig>,
    watcher: Option<Res<BallisticsConfigWatcher>>,
) {
    let Some(watcher) = watcher else { return };
    while let Ok(Ok(event)) = watcher.receiver.try_recv() {
        if event.kind.is_modify() {
            match BallisticsConfig::load() {
                Ok(new_config) => {
                    info!("ballistics.yaml reloaded");
                    *config = new_config;
                }
                Err(e) => warn!("ballistics.yaml reload failed: {}", e),
            }
        }
    }
}

impl BallisticsConfig {
    fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let path = ballistics_path();
        let content = std::fs::read_to_string(&path)?;
        Ok(serde_yaml::from_str(&content)?)
    }
}

fn ballistics_path() -> PathBuf {
    if let Some(path) = std::env::var_os("DAEDALUS_BALLISTICS")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .filter(|p| p.is_file())
    {
        return path;
    }

    [
        std::env::current_dir()
            .ok()
            .map(|p| p.join("ballistics.yaml")),
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|parent| parent.join("ballistics.yaml"))),
        Some(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("ballistics.yaml")),
    ]
    .into_iter()
    .flatten()
    .find(|p| p.is_file())
    .unwrap_or_else(|| PathBuf::from("ballistics.yaml"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shipped_ballistics_yaml_deserializes() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("ballistics.yaml");
        let yaml = std::fs::read_to_string(path).expect("ballistics.yaml is shipped");
        let config: BallisticsConfig = serde_yaml::from_str(&yaml).expect("valid yaml");
        assert!(config.muzzle_speed_mps > 0.0);
        assert!(config.dart_speed_mps > 0.0);
        assert_eq!(config.camera_offset.len(), 3);
    }
}
