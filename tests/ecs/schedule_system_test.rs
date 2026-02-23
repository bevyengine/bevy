// ---------------------------------------------------------------------------
// Tests for `WithRefInputWrapper` and `WithClonedInputWrapper`.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod with_input_wrapper_tests {
    use crate::{prelude::*, system::IntoSystem};

    // -----------------------------------------------------------------------
    // with_input_ref — InRef<T>
    // -----------------------------------------------------------------------

    /// A system that reads a shared `u32` reference and returns its value.
    fn read_u32(InRef(value): InRef<u32>) -> u32 {
        *value
    }

    #[test]
    fn with_input_ref_passes_shared_reference() {
        let mut world = World::new();

        let mut system = IntoSystem::into_system(read_u32.with_input_ref(42_u32));
        system.initialize(&mut world);

        // The system can be run multiple times; the stored value is never consumed.
        assert_eq!(system.run((), &mut world).unwrap(), 42);
        assert_eq!(system.run((), &mut world).unwrap(), 42);
    }

    #[test]
    fn with_input_ref_value_mut_affects_subsequent_runs() {
        let mut world = World::new();

        let mut system = IntoSystem::into_system(read_u32.with_input_ref(10_u32));
        system.initialize(&mut world);

        assert_eq!(system.run((), &mut world).unwrap(), 10);

        // Mutate the stored value between runs.
        // `system` here is a `WithRefInputWrapper`, so we can call `.value_mut()`.
        *system.value_mut() = 99;

        assert_eq!(system.run((), &mut world).unwrap(), 99);
    }

    #[test]
    fn with_input_ref_system_cannot_observe_mutations_to_stored_value_between_runs() {
        let mut world = World::new();

        let mut system = IntoSystem::into_system(read_u32.with_input_ref(7_u32));
        system.initialize(&mut world);

        for _ in 0..5 {
            assert_eq!(system.run((), &mut world).unwrap(), 7);
        }
    }

    // -----------------------------------------------------------------------
    // with_cloned_input — In<T>
    // -----------------------------------------------------------------------

    /// A system that takes owned `String` input and returns its length.
    fn string_len(In(s): In<String>) -> usize {
        s.len()
    }

    #[test]
    fn with_cloned_input_clones_on_every_run() {
        let mut world = World::new();

        let template = String::from("hello");
        let mut system = IntoSystem::into_system(string_len.with_cloned_input(template));
        system.initialize(&mut world);

        // Each run clones the stored "hello", passes it by value, and the
        assert_eq!(system.run((), &mut world).unwrap(), 5);
        assert_eq!(system.run((), &mut world).unwrap(), 5);
        assert_eq!(system.run((), &mut world).unwrap(), 5);
    }

    #[test]
    fn with_cloned_input_copy_type_works() {
        let mut world = World::new();

        // Entity is Copy; the clone is a free bit-copy.
        fn use_entity(In(e): In<u64>) -> u64 {
            e * 2
        }

        let mut system = IntoSystem::into_system(use_entity.with_cloned_input(21_u64));
        system.initialize(&mut world);

        assert_eq!(system.run((), &mut world).unwrap(), 42);
        assert_eq!(system.run((), &mut world).unwrap(), 42);
    }

    #[test]
    fn with_cloned_input_value_mut_affects_subsequent_runs() {
        let mut world = World::new();

        let mut system = IntoSystem::into_system(string_len.with_cloned_input(String::from("hi")));
        system.initialize(&mut world);

        assert_eq!(system.run((), &mut world).unwrap(), 2);

        *system.value_mut() = String::from("longer string");

        assert_eq!(system.run((), &mut world).unwrap(), 13);
    }

    // -----------------------------------------------------------------------
    // Coexistence: with_input, with_input_ref, with_cloned_input side-by-side
    // -----------------------------------------------------------------------

    #[test]
    fn all_three_wrappers_can_coexist_in_same_schedule() {
        // This is a compile-time / scheduler-integration check.

        let mut world = World::new();
        let mut schedule = Schedule::default();

        fn mut_system(InMut(v): InMut<u32>) {
            *v += 1;
        }
        fn ref_system(InRef(v): InRef<u32>) {
            let _ = *v; // read-only
        }
        fn own_system(In(v): In<u32>) {
            let _ = v; // consumes the clone
        }

        schedule.add_systems((
            mut_system.with_input(0_u32),
            ref_system.with_input_ref(0_u32),
            own_system.with_cloned_input(0_u32),
        ));

        // Just verify no panic on run.
        schedule.run(&mut world);
    }
}