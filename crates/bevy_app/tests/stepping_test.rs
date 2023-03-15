mod stepping {
    use bevy_app::{App, CoreSchedule};
    use bevy_ecs::prelude::*;
    use bevy_ecs::schedule::ScheduleEvent;

    /// verify App::update() ScheduleEvents behavior
    #[test]
    fn app_update_schedule_events() {
        let mut app = App::new();

        // add a system to write a ScheduleEvent
        app.add_system(|mut schedule_events: EventWriter<ScheduleEvent>| {
            schedule_events.send(ScheduleEvent::EnableStepping(Box::new(CoreSchedule::Main)));
        });

        // ensure stepping isn't enabled on the schedule
        let schedule = app.get_schedule(CoreSchedule::Main).unwrap();
        assert!(!schedule.stepping());

        app.update();

        // verify the event was sent to the schedule by verifing stepping has
        // been turned on
        let schedule = app.get_schedule(CoreSchedule::Main).unwrap();
        assert!(schedule.stepping());

        // verify the ScheduleEvent list was cleared
        let schedule_events = app.world.get_resource::<Events<ScheduleEvent>>().unwrap();
        assert!(schedule_events.is_empty());
    }
}
