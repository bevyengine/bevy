//! Machine preset catalog.

use bevy_ecs::prelude::*;
use bevy_reflect::Reflect;

use super::components::{Machine, MachineBundle, MachineType, Mobility, WorkEnvelope};

/// Resource containing preset machine configurations.
#[derive(Resource, Default, Reflect)]
pub struct MachineCatalog {
    /// Whether the catalog has been initialized.
    initialized: bool,
}

impl MachineCatalog {
    /// Creates a machine bundle from a preset type.
    pub fn create_machine(&self, machine_type: MachineType, id: String) -> MachineBundle {
        match machine_type {
            MachineType::Excavator => self.create_excavator(id),
            MachineType::Dozer => self.create_dozer(id),
            MachineType::Loader => self.create_loader(id),
            MachineType::DumpTruck => self.create_dump_truck(id),
        }
    }

    /// Creates an excavator preset.
    pub fn create_excavator(&self, id: String) -> MachineBundle {
        MachineBundle {
            machine: Machine {
                id,
                machine_type: MachineType::Excavator,
                current_load: 0.0,
                capacity: 1.5, // 1.5 cubic meters
                fuel: 1.0,
            },
            envelope: WorkEnvelope::Toroidal {
                inner_radius: 2.0,
                outer_radius: 10.0,
                min_height: -6.0,
                max_height: 8.0,
            },
            mobility: Mobility {
                max_speed: 3.0,
                turn_rate: 0.5,
                can_reverse: true,
                tracked: true,
            },
            ..Default::default()
        }
    }

    /// Creates a bulldozer preset.
    pub fn create_dozer(&self, id: String) -> MachineBundle {
        MachineBundle {
            machine: Machine {
                id,
                machine_type: MachineType::Dozer,
                current_load: 0.0,
                capacity: 3.0, // 3 cubic meters blade capacity
                fuel: 1.0,
            },
            envelope: WorkEnvelope::Rectangular {
                width: 4.0,
                depth: 3.0,
                height: 1.5,
            },
            mobility: Mobility {
                max_speed: 8.0,
                turn_rate: 0.3,
                can_reverse: true,
                tracked: true,
            },
            ..Default::default()
        }
    }

    /// Creates a wheel loader preset.
    pub fn create_loader(&self, id: String) -> MachineBundle {
        MachineBundle {
            machine: Machine {
                id,
                machine_type: MachineType::Loader,
                current_load: 0.0,
                capacity: 2.5, // 2.5 cubic meters
                fuel: 1.0,
            },
            envelope: WorkEnvelope::Arc {
                radius: 5.0,
                angle: std::f32::consts::PI / 3.0, // 60 degrees
                min_height: 0.0,
                max_height: 4.0,
            },
            mobility: Mobility {
                max_speed: 12.0,
                turn_rate: 0.8,
                can_reverse: true,
                tracked: false,
            },
            ..Default::default()
        }
    }

    /// Creates a dump truck preset.
    pub fn create_dump_truck(&self, id: String) -> MachineBundle {
        MachineBundle {
            machine: Machine {
                id,
                machine_type: MachineType::DumpTruck,
                current_load: 0.0,
                capacity: 15.0, // 15 cubic meters
                fuel: 1.0,
            },
            envelope: WorkEnvelope::Rectangular {
                width: 3.0,
                depth: 6.0,
                height: 2.0,
            },
            mobility: Mobility {
                max_speed: 20.0,
                turn_rate: 0.4,
                can_reverse: true,
                tracked: false,
            },
            ..Default::default()
        }
    }
}
