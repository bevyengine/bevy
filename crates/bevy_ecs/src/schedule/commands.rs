use super::{IntoSystemDescriptor, Schedule, StageLabel, SystemDescriptor};

#[derive(Default)]
pub struct ScheduleCommandQueue {
    items: Vec<Box<dyn ScheduleCommand>>,
}

impl ScheduleCommandQueue {
    pub fn push<C>(&mut self, command: C)
    where
        C: ScheduleCommand,
    {
        self.items.push(Box::new(command));
    }

    pub fn apply(&mut self, schedule: &mut Schedule) {
        for command in self.items.drain(..) {
            command.write(schedule);
        }
    }

    pub fn transfer(&mut self, queue: &mut ScheduleCommandQueue) {
        queue.items.extend(self.items.drain(..));
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

/// A [`Schedule`] mutation.
pub trait ScheduleCommand: Send + Sync + 'static {
    fn write(self: Box<Self>, schedule: &mut Schedule);
}

pub struct ScheduleCommands<'a> {
    queue: &'a mut ScheduleCommandQueue,
}

impl<'a> ScheduleCommands<'a> {
    pub fn new(queue: &'a mut ScheduleCommandQueue) -> Self {
        Self { queue }
    }

    pub fn insert_system<T, S, Params>(&mut self, system: T, stage_label: S)
    where
        T: IntoSystemDescriptor<Params>,
        S: StageLabel,
    {
        self.queue.push(InsertSystem {
            system: system.into_descriptor(),
            stage_label,
        });
    }
}

pub struct InsertSystem<S>
where
    S: StageLabel,
{
    pub system: SystemDescriptor,
    pub stage_label: S,
}

impl<S> ScheduleCommand for InsertSystem<S>
where
    S: StageLabel,
{
    fn write(self: Box<Self>, schedule: &mut Schedule) {
        schedule.add_system_to_stage(self.stage_label, self.system);
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        schedule::{Schedule, ScheduleCommandQueue, ScheduleCommands, SystemStage},
        system::Commands,
        world::World,
    };

    #[test]
    fn insert_system() {
        fn sample_system(mut _commands: Commands) {}
        let mut schedule = Schedule::default();
        schedule.add_stage("test", SystemStage::parallel());
        let mut queue = ScheduleCommandQueue::default();
        let mut schedule_commands = ScheduleCommands::new(&mut queue);
        schedule_commands.insert_system(sample_system, "test");
        queue.apply(&mut schedule);

        let stage = schedule.get_stage::<SystemStage>(&"test").unwrap();
        assert_eq!(stage.parallel_systems().len(), 1);
    }

    #[test]
    fn insert_system_from_system() {
        fn sample_system(mut schedule_commands: ScheduleCommands) {
            schedule_commands.insert_system(|| {}, "test");
        }

        let mut world = World::default();
        let mut schedule = Schedule::default();
        schedule.add_stage("test", SystemStage::parallel());
        schedule.add_system_to_stage("test", sample_system);
        schedule.run_once(&mut world);

        let stage = schedule.get_stage::<SystemStage>(&"test").unwrap();
        assert_eq!(stage.parallel_systems().len(), 2);
    }
}
