use bevy::prelude::Resource;
use serde::Serialize;

use crate::robomaster::prelude::Team;

pub const RUNE_EFFECTIVE_RADIUS_M: f32 = 0.150;
const RUNE_RING_WIDTH_M: f32 = RUNE_EFFECTIVE_RADIUS_M / 10.0;

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct BigRuneScore {
    pub team: &'static str,
    pub run_id: u64,
    pub run_active: bool,
    pub activated_arms: u8,
    pub has_hit: bool,
    pub average_ring: f32,
    pub last_ring: u8,
    pub last_radius_mm: u16,
    pub last_target: i8,
    #[serde(skip)]
    ring_total: u32,
}

impl BigRuneScore {
    const fn new(team: &'static str) -> Self {
        Self {
            team,
            run_id: 0,
            run_active: false,
            activated_arms: 0,
            has_hit: false,
            average_ring: 0.0,
            last_ring: 0,
            last_radius_mm: 0,
            last_target: -1,
            ring_total: 0,
        }
    }

    fn begin_run(&mut self) {
        self.run_id = self.run_id.saturating_add(1);
        self.run_active = true;
        self.clear_hits();
    }

    fn clear_hits(&mut self) {
        self.activated_arms = 0;
        self.has_hit = false;
        self.average_ring = 0.0;
        self.last_ring = 0;
        self.last_radius_mm = 0;
        self.last_target = -1;
        self.ring_total = 0;
    }

    fn record_hit(&mut self, target: usize, radius_m: f32) -> Option<u8> {
        let ring = ring_for_radius(radius_m)?;
        if !self.run_active {
            self.begin_run();
        }
        self.activated_arms = self.activated_arms.saturating_add(1);
        self.has_hit = true;
        self.ring_total = self.ring_total.saturating_add(u32::from(ring));
        self.average_ring = self.ring_total as f32 / f32::from(self.activated_arms);
        self.last_ring = ring;
        self.last_radius_mm = (radius_m * 1000.0).round().clamp(0.0, 150.0) as u16;
        self.last_target = i8::try_from(target).unwrap_or(-1);
        Some(ring)
    }
}

#[derive(Resource, Clone, Debug)]
pub struct BigRuneScores {
    red: BigRuneScore,
    blue: BigRuneScore,
}

impl Default for BigRuneScores {
    fn default() -> Self {
        Self {
            red: BigRuneScore::new("red"),
            blue: BigRuneScore::new("blue"),
        }
    }
}

impl BigRuneScores {
    pub fn get(&self, team: Team) -> &BigRuneScore {
        match team {
            Team::Red => &self.red,
            Team::Blue => &self.blue,
        }
    }

    fn get_mut(&mut self, team: Team) -> &mut BigRuneScore {
        match team {
            Team::Red => &mut self.red,
            Team::Blue => &mut self.blue,
        }
    }

    pub fn sync_rule_run(&mut self, team: Team, active: bool) {
        let score = self.get_mut(team);
        match (active, score.run_active) {
            (true, false) => score.begin_run(),
            (false, true) => score.run_active = false,
            _ => {}
        }
    }

    pub fn clear_non_large(&mut self, team: Team) {
        let score = self.get_mut(team);
        score.run_active = false;
        score.clear_hits();
    }

    pub fn record_hit(&mut self, team: Team, target: usize, radius_m: f32) -> Option<u8> {
        self.get_mut(team).record_hit(target, radius_m)
    }

    pub fn reset_all(&mut self) {
        *self = Self::default();
    }
}

pub fn ring_for_radius(radius_m: f32) -> Option<u8> {
    if !radius_m.is_finite() || !(0.0..=RUNE_EFFECTIVE_RADIUS_M).contains(&radius_m) {
        return None;
    }
    let band = (radius_m / RUNE_RING_WIDTH_M).floor().min(9.0) as u8;
    Some(10 - band)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn radius_maps_to_ten_equal_rings() {
        assert_eq!(ring_for_radius(0.0), Some(10));
        assert_eq!(ring_for_radius(0.0149), Some(10));
        assert_eq!(ring_for_radius(0.0151), Some(9));
        assert_eq!(ring_for_radius(0.1499), Some(1));
        assert_eq!(ring_for_radius(0.1500), Some(1));
        assert_eq!(ring_for_radius(0.1501), None);
    }

    #[test]
    fn new_rule_run_resets_the_previous_average() {
        let mut scores = BigRuneScores::default();
        scores.sync_rule_run(Team::Red, true);
        assert_eq!(scores.record_hit(Team::Red, 2, 0.0), Some(10));
        assert_eq!(scores.record_hit(Team::Red, 3, 0.149), Some(1));
        assert_eq!(scores.get(Team::Red).average_ring, 5.5);

        scores.sync_rule_run(Team::Red, false);
        assert_eq!(scores.get(Team::Red).average_ring, 5.5);
        scores.sync_rule_run(Team::Red, true);
        assert_eq!(scores.get(Team::Red).average_ring, 0.0);
        assert_eq!(scores.get(Team::Red).activated_arms, 0);
        assert_eq!(scores.get(Team::Red).run_id, 2);
    }
}
