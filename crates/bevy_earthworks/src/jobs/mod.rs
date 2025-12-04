//! Simple job/objective system for guided gameplay.

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use bevy_math::IVec3;
use bevy_reflect::Reflect;

use crate::terrain::{Chunk, VoxelTerrain};
use crate::zyns::{ZynsSource, ZynsWallet};

/// Plugin for the job/objective system.
pub struct JobsPlugin;

impl Plugin for JobsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CurrentJob>()
            .register_type::<CurrentJob>()
            .add_systems(Update, (check_job_progress, display_job_hud));
    }
}

/// The current active job.
#[derive(Resource, Default, Clone, Debug, Reflect)]
pub struct CurrentJob {
    /// The active job, if any.
    #[reflect(ignore)]
    pub active: Option<Job>,
    /// Current progress (0.0 to 1.0).
    pub progress: f32,
}

/// A job definition.
#[derive(Clone, Debug)]
pub struct Job {
    /// Display name.
    pub name: String,
    /// Description of the objective.
    pub description: String,
    /// The type and parameters of the job.
    pub job_type: JobType,
    /// Zyns reward on completion.
    pub reward_zyns: u32,
}

/// Types of jobs available.
#[derive(Clone, Debug)]
pub enum JobType {
    /// Level an area to a target height.
    LevelArea {
        /// Minimum X voxel coordinate.
        min_x: i32,
        /// Maximum X voxel coordinate.
        max_x: i32,
        /// Minimum Z voxel coordinate.
        min_z: i32,
        /// Maximum Z voxel coordinate.
        max_z: i32,
        /// Target height in voxels.
        target_height: i32,
    },
}

impl Job {
    /// Creates a "level this area" job.
    pub fn level_area(
        name: &str,
        min: IVec3,
        max: IVec3,
        target_height: i32,
        reward: u32,
    ) -> Self {
        Self {
            name: name.to_string(),
            description: format!("Level the marked area to height {}", target_height),
            job_type: JobType::LevelArea {
                min_x: min.x,
                max_x: max.x,
                min_z: min.z,
                max_z: max.z,
                target_height,
            },
            reward_zyns: reward,
        }
    }
}

/// Checks progress on current job.
fn check_job_progress(
    terrain: Res<VoxelTerrain>,
    chunks: Query<&Chunk>,
    mut current_job: ResMut<CurrentJob>,
    mut wallet: ResMut<ZynsWallet>,
) {
    // Extract job info to avoid borrow conflicts
    let (job_type, reward, name) = {
        let Some(job) = &current_job.active else {
            return;
        };
        (job.job_type.clone(), job.reward_zyns, job.name.clone())
    };

    match &job_type {
        JobType::LevelArea {
            min_x,
            max_x,
            min_z,
            max_z,
            target_height,
        } => {
            let mut correct_voxels = 0;
            let mut total_voxels = 0;

            for x in *min_x..=*max_x {
                for z in *min_z..=*max_z {
                    total_voxels += 1;

                    // Check if this column is at target height
                    let voxel_pos = IVec3::new(x, *target_height, z);
                    let chunk_coord = terrain.voxel_to_chunk(voxel_pos);

                    if let Some(entity) = terrain.get_chunk_entity(chunk_coord) {
                        if let Ok(chunk) = chunks.get(entity) {
                            let local = terrain.voxel_to_local(voxel_pos);
                            if local.x >= 0
                                && local.y >= 0
                                && local.z >= 0
                                && (local.x as usize) < 16
                                && (local.y as usize) < 16
                                && (local.z as usize) < 16
                            {
                                let voxel =
                                    chunk.get(local.x as usize, local.y as usize, local.z as usize);

                                // Check that target height is solid (surface)
                                if voxel.is_solid() {
                                    // Also check that one above is empty
                                    let above_pos = IVec3::new(x, target_height + 1, z);
                                    let above_local = terrain.voxel_to_local(above_pos);

                                    if above_local.y >= 0 && (above_local.y as usize) < 16 {
                                        let above_voxel = chunk.get(
                                            above_local.x as usize,
                                            above_local.y as usize,
                                            above_local.z as usize,
                                        );
                                        if !above_voxel.is_solid() {
                                            correct_voxels += 1;
                                        }
                                    } else {
                                        // Above is in different chunk - count as correct for now
                                        correct_voxels += 1;
                                    }
                                }
                            }
                        }
                    }
                }
            }

            let progress = if total_voxels > 0 {
                correct_voxels as f32 / total_voxels as f32
            } else {
                0.0
            };
            current_job.progress = progress;

            // Job complete!
            if progress >= 0.95 {
                println!("JOB COMPLETE: {}! +{} Zyns", name, reward);
                wallet.earn(reward, ZynsSource::Achievement);
                current_job.active = None;
                current_job.progress = 0.0;
            }
        }
    }
}

/// Display job progress (simple console output for now).
fn display_job_hud(current_job: Res<CurrentJob>) {
    // Only update occasionally to avoid console spam
    use std::sync::atomic::{AtomicU32, Ordering};
    static FRAME: AtomicU32 = AtomicU32::new(0);
    if FRAME.fetch_add(1, Ordering::Relaxed) % 120 != 0 {
        return;
    }

    if let Some(job) = &current_job.active {
        let bar_len = 20;
        let filled = (current_job.progress * bar_len as f32) as usize;
        let bar: String = "█".repeat(filled) + &"░".repeat(bar_len - filled);
        println!(
            "JOB: {} [{}] {:.0}%",
            job.name,
            bar,
            current_job.progress * 100.0
        );
    }
}
