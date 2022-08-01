use bevy_ecs::{
    component::Component,
    prelude::{ParallelSystemDescriptorCoercion, Res, RunCriteriaDescriptorCoercion},
    schedule::{ShouldRun, Stage, SystemStage},
    system::Query,
    world::World,
};
use criterion::Criterion;

fn run_stage(stage: &mut SystemStage, world: &mut World) {
    stage.run(world);
}

pub fn run_criteria_yes(criterion: &mut Criterion) {
    let mut world = World::new();
    let mut group = criterion.benchmark_group("run_criteria/yes");
    group.warm_up_time(std::time::Duration::from_millis(500));
    group.measurement_time(std::time::Duration::from_secs(3));
    fn empty() {}
    fn always_yes() -> ShouldRun {
        ShouldRun::Yes
    }
    for amount in 0..21 {
        let mut stage = SystemStage::parallel();
        stage.add_system(empty.with_run_criteria(always_yes));
        for _ in 0..amount {
            // TODO: should change this to use a label or have another bench that uses a label instead
            stage
                .add_system(empty.with_run_criteria(always_yes))
                .add_system(empty.with_run_criteria(always_yes))
                .add_system(empty.with_run_criteria(always_yes))
                .add_system(empty.with_run_criteria(always_yes))
                .add_system(empty.with_run_criteria(always_yes));
        }
        // run once to initialize systems
        run_stage(&mut stage, &mut world);
        group.bench_function(&format!("{:03}_systems", 5 * amount + 1), |bencher| {
            bencher.iter(|| {
                run_stage(&mut stage, &mut world);
            });
        });
    }
    group.finish();
}

pub fn run_criteria_no(criterion: &mut Criterion) {
    let mut world = World::new();
    let mut group = criterion.benchmark_group("run_criteria/no");
    group.warm_up_time(std::time::Duration::from_millis(500));
    group.measurement_time(std::time::Duration::from_secs(3));
    fn empty() {}
    fn always_no() -> ShouldRun {
        ShouldRun::No
    }
    for amount in 0..21 {
        let mut stage = SystemStage::parallel();
        stage.add_system(empty.with_run_criteria(always_no));
        for _ in 0..amount {
            stage
                .add_system(empty.with_run_criteria(always_no))
                .add_system(empty.with_run_criteria(always_no))
                .add_system(empty.with_run_criteria(always_no))
                .add_system(empty.with_run_criteria(always_no))
                .add_system(empty.with_run_criteria(always_no));
        }
        // run once to initialize systems
        run_stage(&mut stage, &mut world);
        group.bench_function(&format!("{:03}_systems", 5 * amount + 1), |bencher| {
            bencher.iter(|| {
                run_stage(&mut stage, &mut world);
            });
        });
    }
    group.finish();
}

pub fn run_criteria_yes_with_labels(criterion: &mut Criterion) {
    let mut world = World::new();
    let mut group = criterion.benchmark_group("run_criteria/yes_with_labels");
    group.warm_up_time(std::time::Duration::from_millis(500));
    group.measurement_time(std::time::Duration::from_secs(3));
    fn empty() {}
    fn always_yes() -> ShouldRun {
        ShouldRun::Yes
    }
    for amount in 0..21 {
        let mut stage = SystemStage::parallel();
        stage.add_system(empty.with_run_criteria(always_yes.label("always yes")));
        for _ in 0..amount {
            stage
                .add_system(empty.with_run_criteria("always yes"))
                .add_system(empty.with_run_criteria("always yes"))
                .add_system(empty.with_run_criteria("always yes"))
                .add_system(empty.with_run_criteria("always yes"))
                .add_system(empty.with_run_criteria("always yes"));
        }
        // run once to initialize systems
        run_stage(&mut stage, &mut world);
        group.bench_function(&format!("{:03}_systems", 5 * amount + 1), |bencher| {
            bencher.iter(|| {
                run_stage(&mut stage, &mut world);
            });
        });
    }
    group.finish();
}

pub fn run_criteria_no_with_labels(criterion: &mut Criterion) {
    let mut world = World::new();
    let mut group = criterion.benchmark_group("run_criteria/no_with_labels");
    group.warm_up_time(std::time::Duration::from_millis(500));
    group.measurement_time(std::time::Duration::from_secs(3));
    fn empty() {}
    fn always_no() -> ShouldRun {
        ShouldRun::No
    }
    for amount in 0..21 {
        let mut stage = SystemStage::parallel();
        stage.add_system(empty.with_run_criteria(always_no.label("always no")));
        for _ in 0..amount {
            stage
                .add_system(empty.with_run_criteria("always no"))
                .add_system(empty.with_run_criteria("always no"))
                .add_system(empty.with_run_criteria("always no"))
                .add_system(empty.with_run_criteria("always no"))
                .add_system(empty.with_run_criteria("always no"));
        }
        // run once to initialize systems
        run_stage(&mut stage, &mut world);
        group.bench_function(&format!("{:03}_systems", 5 * amount + 1), |bencher| {
            bencher.iter(|| {
                run_stage(&mut stage, &mut world);
            });
        });
    }
    group.finish();
}

#[derive(Component)]
struct TestBool(pub bool);

pub fn run_criteria_yes_with_query(criterion: &mut Criterion) {
    let mut world = World::new();
    world.spawn().insert(TestBool(true));
    let mut group = criterion.benchmark_group("run_criteria/yes_using_query");
    group.warm_up_time(std::time::Duration::from_millis(500));
    group.measurement_time(std::time::Duration::from_secs(3));
    fn empty() {}
    fn yes_with_query(query: Query<&TestBool>) -> ShouldRun {
        query.single().0.into()
    }
    for amount in 0..21 {
        let mut stage = SystemStage::parallel();
        stage.add_system(empty.with_run_criteria(yes_with_query));
        for _ in 0..amount {
            stage
                .add_system(empty.with_run_criteria(yes_with_query))
                .add_system(empty.with_run_criteria(yes_with_query))
                .add_system(empty.with_run_criteria(yes_with_query))
                .add_system(empty.with_run_criteria(yes_with_query))
                .add_system(empty.with_run_criteria(yes_with_query));
        }
        // run once to initialize systems
        run_stage(&mut stage, &mut world);
        group.bench_function(&format!("{:03}_systems", 5 * amount + 1), |bencher| {
            bencher.iter(|| {
                run_stage(&mut stage, &mut world);
            });
        });
    }
    group.finish();
}

pub fn run_criteria_yes_with_resource(criterion: &mut Criterion) {
    let mut world = World::new();
    world.insert_resource(TestBool(true));
    let mut group = criterion.benchmark_group("run_criteria/yes_using_resource");
    group.warm_up_time(std::time::Duration::from_millis(500));
    group.measurement_time(std::time::Duration::from_secs(3));
    fn empty() {}
    fn yes_with_resource(res: Res<TestBool>) -> ShouldRun {
        res.0.into()
    }
    for amount in 0..21 {
        let mut stage = SystemStage::parallel();
        stage.add_system(empty.with_run_criteria(yes_with_resource));
        for _ in 0..amount {
            stage
                .add_system(empty.with_run_criteria(yes_with_resource))
                .add_system(empty.with_run_criteria(yes_with_resource))
                .add_system(empty.with_run_criteria(yes_with_resource))
                .add_system(empty.with_run_criteria(yes_with_resource))
                .add_system(empty.with_run_criteria(yes_with_resource));
        }
        // run once to initialize systems
        run_stage(&mut stage, &mut world);
        group.bench_function(&format!("{:03}_systems", 5 * amount + 1), |bencher| {
            bencher.iter(|| {
                run_stage(&mut stage, &mut world);
            });
        });
    }
    group.finish();
}
