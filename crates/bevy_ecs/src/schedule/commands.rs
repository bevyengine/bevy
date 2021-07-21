use super::{IntoSystemDescriptor, Schedule, StageLabel, SystemDescriptor};

#[derive(Default)]
pub struct SchedulerCommandQueue {
    items: Vec<Box<dyn SchedulerCommand>>,
}

impl SchedulerCommandQueue {
    pub fn push<C>(&mut self, command: C)
    where
        C: SchedulerCommand,
    {
        self.items.push(Box::new(command));
    }

    pub fn apply(&mut self, schedule: &mut Schedule) {
        for command in self.items.drain(..) {
            command.write(schedule);
        }
    }

    pub fn transfer(&mut self, queue: &mut SchedulerCommandQueue) {
        queue.items.extend(self.items.drain(..));
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

/// A [`Schedule`] mutation.
pub trait SchedulerCommand: Send + Sync + 'static {
    fn write(self: Box<Self>, schedule: &mut Schedule);
}

pub struct SchedulerCommands<'a> {
    queue: &'a mut SchedulerCommandQueue,
}

impl<'a> SchedulerCommands<'a> {
    pub fn new(queue: &'a mut SchedulerCommandQueue) -> Self {
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

impl<S> SchedulerCommand for InsertSystem<S>
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
        schedule::{Schedule, SchedulerCommandQueue, SchedulerCommands, SystemStage},
        system::Commands,
        world::World,
    };

    #[test]
    fn insert_system() {
        fn sample_system(mut _commands: Commands) {}
        let mut schedule = Schedule::default();
        schedule.add_stage("test", SystemStage::parallel());
        let mut queue = SchedulerCommandQueue::default();
        let mut scheduler_commands = SchedulerCommands::new(&mut queue);
        scheduler_commands.insert_system(sample_system, "test");
        queue.apply(&mut schedule);

        let stage = schedule.get_stage::<SystemStage>(&"test").unwrap();
        assert_eq!(stage.parallel_systems().len(), 1);
    }

    #[test]
    fn insert_system_from_system() {
        fn sample_system(mut scheduler_commands: SchedulerCommands) {
            scheduler_commands.insert_system(|| {}, "test");
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
