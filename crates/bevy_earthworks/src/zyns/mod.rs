//! Zyns gamification system for earthworks simulation.
//!
//! Provides time-based rewards and achievements for player actions.

pub mod ui;

pub use ui::{ZynsUiPlugin, ZynsUiState};

use bevy_app::prelude::*;
use bevy_asset::prelude::*;
use bevy_ecs::prelude::*;
use bevy_math::Vec3;
use bevy_mesh::Mesh;
use bevy_pbr::StandardMaterial;
use bevy_reflect::Reflect;
use bevy_time::Time;

use crate::effects::{spawn_achievement_particles, EffectsConfig};
use crate::machines::MachineEvent;
use crate::plan::PlanStepEvent;
use crate::scoring::SimulationScore;
use crate::terrain::TerrainModifiedEvent;

/// Plugin for the Zyns gamification system.
pub struct ZynsPlugin;

impl Plugin for ZynsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ZynsWallet>()
            .init_resource::<ZynsConfig>()
            .init_resource::<AchievementTracker>()
            .add_message::<ZynsEarnedEvent>()
            .add_message::<AchievementUnlockedEvent>()
            .add_plugins(ZynsUiPlugin)
            .add_systems(
                Update,
                (
                    zyns_reward_system,
                    achievement_check_system,
                    achievement_particle_system,
                ),
            );
    }
}

/// Configuration for Zyns rewards.
#[derive(Resource, Clone, Debug, Reflect)]
pub struct ZynsConfig {
    /// Base Zyns per cubic meter excavated.
    pub zyns_per_volume: f32,
    /// Time bonus multiplier (applied when under target time).
    pub time_bonus_multiplier: f32,
    /// Target seconds per cubic meter for time bonus.
    pub target_seconds_per_volume: f32,
    /// Efficiency bonus multiplier (based on SimulationScore.efficiency).
    pub efficiency_bonus_multiplier: f32,
    /// Zyns awarded per completed operation.
    pub zyns_per_operation: u32,
}

impl Default for ZynsConfig {
    fn default() -> Self {
        Self {
            zyns_per_volume: 10.0,            // 10 Zyns per m³
            time_bonus_multiplier: 2.0,       // 2x bonus for fast completion
            target_seconds_per_volume: 5.0,   // 5 seconds per m³ target
            efficiency_bonus_multiplier: 1.5, // 1.5x for high efficiency
            zyns_per_operation: 5,            // 5 Zyns per operation
        }
    }
}

/// Resource tracking the player's Zyns balance.
#[derive(Resource, Default, Clone, Debug, Reflect)]
pub struct ZynsWallet {
    /// Current Zyns balance.
    pub balance: u64,
    /// Total Zyns earned this session.
    pub total_earned: u64,
    /// Zyns earned from volume operations.
    pub from_volume: u64,
    /// Zyns earned from time bonuses.
    pub from_time_bonus: u64,
    /// Zyns earned from achievements.
    pub from_achievements: u64,
    /// Recent transaction for UI animation.
    pub last_earned: u32,
    /// Time since last earning (for animation fadeout).
    pub last_earned_timer: f32,
}

impl ZynsWallet {
    /// Adds Zyns to the wallet.
    pub fn earn(&mut self, amount: u32, source: ZynsSource) {
        self.balance += amount as u64;
        self.total_earned += amount as u64;
        self.last_earned = amount;
        self.last_earned_timer = 0.0;

        match source {
            ZynsSource::Volume | ZynsSource::Operation => self.from_volume += amount as u64,
            ZynsSource::TimeBonus => self.from_time_bonus += amount as u64,
            ZynsSource::Achievement => self.from_achievements += amount as u64,
        }
    }

    /// Resets the wallet.
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// Source of Zyns earnings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ZynsSource {
    /// Earned from excavation/fill volume.
    Volume,
    /// Earned from time bonus.
    TimeBonus,
    /// Earned from achievement.
    Achievement,
    /// Earned from completing an operation.
    Operation,
}

/// Event emitted when Zyns are earned.
#[derive(Message, Clone, Debug)]
pub struct ZynsEarnedEvent {
    /// Amount of Zyns earned.
    pub amount: u32,
    /// Source of the earnings.
    pub source: ZynsSource,
    /// Optional position for particle effects (world coordinates).
    pub position: Option<Vec3>,
}

/// Achievement definition.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Reflect)]
pub enum Achievement {
    /// Excavated total volume milestone.
    VolumeExcavated(u32), // 100, 500, 1000, 5000 m³
    /// Completed operations milestone.
    OperationsCompleted(u32), // 10, 50, 100, 500
    /// Maintained high efficiency.
    EfficiencyMaster, // 90%+ efficiency for 60 seconds
    /// Speed demon - completed operation in record time.
    SpeedDemon, // Any operation under 50% target time
    /// First excavation.
    FirstDig,
}

impl Achievement {
    /// Returns the Zyns reward for this achievement.
    pub fn reward(&self) -> u32 {
        match self {
            Achievement::VolumeExcavated(v) => match v {
                100 => 50,
                500 => 200,
                1000 => 500,
                5000 => 2000,
                _ => 25,
            },
            Achievement::OperationsCompleted(n) => match n {
                10 => 25,
                50 => 100,
                100 => 250,
                500 => 1000,
                _ => 10,
            },
            Achievement::EfficiencyMaster => 500,
            Achievement::SpeedDemon => 100,
            Achievement::FirstDig => 10,
        }
    }

    /// Returns display name for the achievement.
    pub fn name(&self) -> &'static str {
        match self {
            Achievement::VolumeExcavated(100) => "Novice Excavator",
            Achievement::VolumeExcavated(500) => "Skilled Operator",
            Achievement::VolumeExcavated(1000) => "Master Mover",
            Achievement::VolumeExcavated(5000) => "Earth Shaper",
            Achievement::VolumeExcavated(_) => "Volume Milestone",
            Achievement::OperationsCompleted(10) => "Getting Started",
            Achievement::OperationsCompleted(50) => "Busy Bee",
            Achievement::OperationsCompleted(100) => "Workhorse",
            Achievement::OperationsCompleted(500) => "Machine Master",
            Achievement::OperationsCompleted(_) => "Operations Milestone",
            Achievement::EfficiencyMaster => "Efficiency Master",
            Achievement::SpeedDemon => "Speed Demon",
            Achievement::FirstDig => "First Dig",
        }
    }
}

/// Event emitted when an achievement is unlocked.
#[derive(Message, Clone, Debug)]
pub struct AchievementUnlockedEvent {
    /// The achievement that was unlocked.
    pub achievement: Achievement,
    /// Zyns reward amount.
    pub reward: u32,
}

/// Resource tracking unlocked achievements and progress.
#[derive(Resource, Default, Clone, Debug, Reflect)]
pub struct AchievementTracker {
    /// Set of unlocked achievements.
    #[reflect(ignore)]
    pub unlocked: bevy_platform::collections::HashSet<Achievement>,
    /// Time spent at high efficiency (for `EfficiencyMaster`).
    pub high_efficiency_time: f32,
    /// Whether first dig has been tracked.
    pub first_dig_done: bool,
}

/// System that calculates and awards Zyns based on events.
fn zyns_reward_system(
    time: Res<Time>,
    config: Res<ZynsConfig>,
    score: Res<SimulationScore>,
    mut wallet: ResMut<ZynsWallet>,
    mut terrain_events: MessageReader<TerrainModifiedEvent>,
    mut machine_events: MessageReader<MachineEvent>,
    mut step_events: MessageReader<PlanStepEvent>,
    mut zyns_events: MessageWriter<ZynsEarnedEvent>,
) {
    // Update animation timer
    wallet.last_earned_timer += time.delta_secs();

    // Process terrain events for volume-based rewards
    for event in terrain_events.read() {
        let volume = event.volume_changed.abs() as f32 * 0.0283; // voxels to m³
        let base_zyns = (volume * config.zyns_per_volume) as u32;

        if base_zyns > 0 {
            // Calculate time bonus
            let time_per_volume = if volume > 0.0 {
                score.elapsed_time / (score.volume_excavated + score.volume_filled).max(0.1)
            } else {
                config.target_seconds_per_volume
            };

            let time_bonus = if time_per_volume < config.target_seconds_per_volume {
                let bonus_ratio = 1.0 - (time_per_volume / config.target_seconds_per_volume);
                (base_zyns as f32 * bonus_ratio * config.time_bonus_multiplier) as u32
            } else {
                0
            };

            // Apply efficiency bonus
            let efficiency_bonus = if score.efficiency > 0.8 {
                (base_zyns as f32 * (score.efficiency - 0.8) * config.efficiency_bonus_multiplier)
                    as u32
            } else {
                0
            };

            let total = base_zyns + time_bonus + efficiency_bonus;
            wallet.earn(base_zyns, ZynsSource::Volume);

            if time_bonus > 0 {
                wallet.earn(time_bonus, ZynsSource::TimeBonus);
            }

            zyns_events.write(ZynsEarnedEvent {
                amount: total,
                source: ZynsSource::Volume,
                position: None, // Could add chunk position here
            });
        }
    }

    // Process machine events for operation rewards
    for event in machine_events.read() {
        match event {
            MachineEvent::CompletedExcavating { .. } | MachineEvent::CompletedDumping { .. } => {
                wallet.earn(config.zyns_per_operation, ZynsSource::Operation);
                zyns_events.write(ZynsEarnedEvent {
                    amount: config.zyns_per_operation,
                    source: ZynsSource::Operation,
                    position: None,
                });
            }
            _ => {}
        }
    }

    // Process plan step events
    for event in step_events.read() {
        if matches!(event.result, crate::plan::StepResult::Success) {
            // Small bonus for successful plan steps
            wallet.earn(2, ZynsSource::Operation);
        }
    }
}

/// System that checks for and awards achievements.
fn achievement_check_system(
    time: Res<Time>,
    score: Res<SimulationScore>,
    mut wallet: ResMut<ZynsWallet>,
    mut tracker: ResMut<AchievementTracker>,
    mut terrain_events: MessageReader<TerrainModifiedEvent>,
    mut achievement_events: MessageWriter<AchievementUnlockedEvent>,
    mut zyns_events: MessageWriter<ZynsEarnedEvent>,
) {
    // Check for first dig
    if !tracker.first_dig_done && terrain_events.read().next().is_some() {
        tracker.first_dig_done = true;
        let achievement = Achievement::FirstDig;
        if tracker.unlocked.insert(achievement.clone()) {
            let reward = achievement.reward();
            wallet.earn(reward, ZynsSource::Achievement);
            achievement_events.write(AchievementUnlockedEvent {
                achievement: achievement.clone(),
                reward,
            });
            zyns_events.write(ZynsEarnedEvent {
                amount: reward,
                source: ZynsSource::Achievement,
                position: None,
            });
        }
    }

    // Check volume milestones
    let total_volume = (score.volume_excavated + score.volume_filled) as u32;
    for threshold in [100, 500, 1000, 5000] {
        if total_volume >= threshold {
            let achievement = Achievement::VolumeExcavated(threshold);
            if tracker.unlocked.insert(achievement.clone()) {
                let reward = achievement.reward();
                wallet.earn(reward, ZynsSource::Achievement);
                achievement_events.write(AchievementUnlockedEvent {
                    achievement: achievement.clone(),
                    reward,
                });
                zyns_events.write(ZynsEarnedEvent {
                    amount: reward,
                    source: ZynsSource::Achievement,
                    position: None,
                });
            }
        }
    }

    // Check operations milestones
    for threshold in [10, 50, 100, 500] {
        if score.operations_completed >= threshold {
            let achievement = Achievement::OperationsCompleted(threshold);
            if tracker.unlocked.insert(achievement.clone()) {
                let reward = achievement.reward();
                wallet.earn(reward, ZynsSource::Achievement);
                achievement_events.write(AchievementUnlockedEvent {
                    achievement: achievement.clone(),
                    reward,
                });
                zyns_events.write(ZynsEarnedEvent {
                    amount: reward,
                    source: ZynsSource::Achievement,
                    position: None,
                });
            }
        }
    }

    // Check efficiency master (90%+ for 60 seconds)
    if score.efficiency >= 0.9 {
        tracker.high_efficiency_time += time.delta_secs();
        if tracker.high_efficiency_time >= 60.0 {
            let achievement = Achievement::EfficiencyMaster;
            if tracker.unlocked.insert(achievement.clone()) {
                let reward = achievement.reward();
                wallet.earn(reward, ZynsSource::Achievement);
                achievement_events.write(AchievementUnlockedEvent {
                    achievement: achievement.clone(),
                    reward,
                });
                zyns_events.write(ZynsEarnedEvent {
                    amount: reward,
                    source: ZynsSource::Achievement,
                    position: None,
                });
            }
        }
    } else {
        tracker.high_efficiency_time = 0.0;
    }
}

/// System that spawns particle effects for achievements.
fn achievement_particle_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    effects_config: Res<EffectsConfig>,
    earthworks_config: Res<crate::config::EarthworksConfig>,
    mut events: MessageReader<AchievementUnlockedEvent>,
    camera_query: Query<&bevy_transform::prelude::Transform, With<bevy_camera::prelude::Camera3d>>,
) {
    // Skip if achievement effects are disabled
    if !earthworks_config.enable_achievement_effects {
        // Still consume events to avoid memory buildup
        for _event in events.read() {}
        return;
    }

    // Get camera position for spawning particles in view
    let camera_pos = camera_query
        .iter()
        .next()
        .map(|t| t.translation)
        .unwrap_or(Vec3::new(0.0, 5.0, 10.0));

    for _event in events.read() {
        // Spawn particles in front of camera
        let spawn_pos = camera_pos + Vec3::new(0.0, 0.0, -5.0);
        spawn_achievement_particles(
            &mut commands,
            &mut meshes,
            &mut materials,
            spawn_pos,
            &effects_config,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wallet_earn() {
        let mut wallet = ZynsWallet::default();
        wallet.earn(100, ZynsSource::Volume);
        assert_eq!(wallet.balance, 100);
        assert_eq!(wallet.total_earned, 100);
        assert_eq!(wallet.from_volume, 100);
    }

    #[test]
    fn test_wallet_earn_multiple_sources() {
        let mut wallet = ZynsWallet::default();
        wallet.earn(50, ZynsSource::Volume);
        wallet.earn(25, ZynsSource::TimeBonus);
        wallet.earn(100, ZynsSource::Achievement);

        assert_eq!(wallet.balance, 175);
        assert_eq!(wallet.from_volume, 50);
        assert_eq!(wallet.from_time_bonus, 25);
        assert_eq!(wallet.from_achievements, 100);
    }

    #[test]
    fn test_wallet_reset() {
        let mut wallet = ZynsWallet::default();
        wallet.earn(100, ZynsSource::Volume);
        wallet.reset();
        assert_eq!(wallet.balance, 0);
        assert_eq!(wallet.total_earned, 0);
    }

    #[test]
    fn test_achievement_rewards() {
        assert_eq!(Achievement::FirstDig.reward(), 10);
        assert_eq!(Achievement::VolumeExcavated(100).reward(), 50);
        assert_eq!(Achievement::VolumeExcavated(1000).reward(), 500);
        assert_eq!(Achievement::EfficiencyMaster.reward(), 500);
    }

    #[test]
    fn test_achievement_names() {
        assert_eq!(Achievement::FirstDig.name(), "First Dig");
        assert_eq!(Achievement::VolumeExcavated(100).name(), "Novice Excavator");
        assert_eq!(Achievement::SpeedDemon.name(), "Speed Demon");
    }

    #[test]
    fn test_config_defaults() {
        let config = ZynsConfig::default();
        assert_eq!(config.zyns_per_volume, 10.0);
        assert_eq!(config.zyns_per_operation, 5);
        assert_eq!(config.time_bonus_multiplier, 2.0);
    }
}
