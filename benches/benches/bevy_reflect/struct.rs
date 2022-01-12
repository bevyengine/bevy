use std::time::Duration;

use bevy::reflect::{DynamicStruct, GetField, Reflect, Struct};
use criterion::{
    black_box, criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion, Throughput,
};

criterion_group!(
    benches,
    concrete_struct_apply,
    concrete_struct_field,
    concrete_struct_name_at,
    dynamic_struct_get_field,
    dynamic_struct_insert,
);
criterion_main!(benches);

const WARM_UP_TIME: Duration = Duration::from_millis(500);
const MEASUREMENT_TIME: Duration = Duration::from_secs(4);
const SIZES: [usize; 4] = [16, 32, 64, 128];

fn concrete_struct_field(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("concrete_struct_field");
    group.warm_up_time(WARM_UP_TIME);
    group.measurement_time(MEASUREMENT_TIME);

    let structs: [Box<dyn Struct>; 4] = [
        Box::new(Struct16::default()),
        Box::new(Struct32::default()),
        Box::new(Struct64::default()),
        Box::new(Struct128::default()),
    ];

    for s in structs {
        let field_count = s.field_len();

        group.bench_with_input(
            BenchmarkId::from_parameter(field_count),
            &s,
            |bencher, s| {
                let field_names = (0..field_count)
                    .map(|i| format!("field_{}", i))
                    .collect::<Vec<_>>();

                bencher.iter(|| {
                    for name in field_names.iter() {
                        s.field(black_box(name));
                    }
                });
            },
        );
    }
}

fn concrete_struct_apply(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("concrete_struct_apply");
    group.warm_up_time(WARM_UP_TIME);
    group.measurement_time(MEASUREMENT_TIME);

    // Use functions that produce trait objects of varying concrete types as the
    // input to the benchmark.
    let inputs: &[fn() -> (Box<dyn Struct>, Box<dyn Reflect>)] = &[
        || (Box::new(Struct16::default()), Box::new(Struct16::default())),
        || (Box::new(Struct32::default()), Box::new(Struct32::default())),
        || (Box::new(Struct64::default()), Box::new(Struct64::default())),
        || {
            (
                Box::new(Struct128::default()),
                Box::new(Struct128::default()),
            )
        },
    ];

    for input in inputs {
        let field_count = input().0.field_len();
        group.throughput(Throughput::Elements(field_count as u64));

        group.bench_with_input(
            BenchmarkId::new("apply_concrete", field_count),
            input,
            |bencher, input| {
                bencher.iter_batched(
                    input,
                    |(mut obj, patch)| obj.apply(black_box(patch.as_ref())),
                    BatchSize::SmallInput,
                );
            },
        );
    }

    for input in inputs {
        let field_count = input().0.field_len();
        group.throughput(Throughput::Elements(field_count as u64));

        group.bench_with_input(
            BenchmarkId::new("apply_dynamic", field_count),
            input,
            |bencher, input| {
                bencher.iter_batched(
                    || {
                        let (obj, _) = input();
                        let patch = obj.clone_dynamic();
                        (obj, patch)
                    },
                    |(mut obj, patch)| obj.apply(black_box(&patch)),
                    BatchSize::SmallInput,
                );
            },
        );
    }
}

fn concrete_struct_name_at(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("concrete_struct_name_at");
    group.warm_up_time(WARM_UP_TIME);
    group.measurement_time(MEASUREMENT_TIME);

    let objects: &[Box<dyn Struct>] = &[
        Box::new(Struct16::default()),
        Box::new(Struct32::default()),
        Box::new(Struct64::default()),
        Box::new(Struct128::default()),
    ];

    for obj in objects {
        let field_count = obj.field_len();
        group.throughput(Throughput::Elements(field_count as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(field_count),
            obj,
            |bencher, obj| {
                let idx = field_count / 2;
                bencher.iter(|| obj.name_at(black_box(idx)));
            },
        );
    }
}

fn dynamic_struct_insert(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("dynamic_struct_insert");
    group.warm_up_time(WARM_UP_TIME);
    group.measurement_time(MEASUREMENT_TIME);

    for field_count in SIZES {
        group.throughput(Throughput::Elements(field_count as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(field_count),
            &field_count,
            |bencher, field_count| {
                let mut s = DynamicStruct::default();
                for i in 0..*field_count {
                    let field_name = format!("field_{}", i);
                    s.insert(&field_name, ());
                }

                let field = format!("field_{}", field_count);
                bencher.iter_batched(
                    || s.clone_dynamic(),
                    |mut s| {
                        black_box(s.insert(black_box(&field), ()));
                    },
                    BatchSize::SmallInput,
                );
            },
        );
    }

    group.finish();
}

fn dynamic_struct_get_field(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("dynamic_struct_get");
    group.warm_up_time(WARM_UP_TIME);
    group.measurement_time(MEASUREMENT_TIME);

    for field_count in SIZES {
        group.throughput(Throughput::Elements(field_count as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(field_count),
            &field_count,
            |bencher, field_count| {
                let mut s = DynamicStruct::default();
                for i in 0..*field_count {
                    let field_name = format!("field_{}", i);
                    s.insert(&field_name, ());
                }

                let field = black_box("field_63");
                bencher.iter(|| {
                    black_box(s.get_field::<()>(field));
                });
            },
        );
    }
}

#[derive(Clone, Default, Reflect)]
struct Struct16 {
    field_0: u32,
    field_1: u32,
    field_2: u32,
    field_3: u32,
    field_4: u32,
    field_5: u32,
    field_6: u32,
    field_7: u32,
    field_8: u32,
    field_9: u32,
    field_10: u32,
    field_11: u32,
    field_12: u32,
    field_13: u32,
    field_14: u32,
    field_15: u32,
}

#[derive(Clone, Default, Reflect)]
struct Struct32 {
    field_0: u32,
    field_1: u32,
    field_2: u32,
    field_3: u32,
    field_4: u32,
    field_5: u32,
    field_6: u32,
    field_7: u32,
    field_8: u32,
    field_9: u32,
    field_10: u32,
    field_11: u32,
    field_12: u32,
    field_13: u32,
    field_14: u32,
    field_15: u32,
    field_16: u32,
    field_17: u32,
    field_18: u32,
    field_19: u32,
    field_20: u32,
    field_21: u32,
    field_22: u32,
    field_23: u32,
    field_24: u32,
    field_25: u32,
    field_26: u32,
    field_27: u32,
    field_28: u32,
    field_29: u32,
    field_30: u32,
    field_31: u32,
}

#[derive(Clone, Default, Reflect)]
struct Struct64 {
    field_0: u32,
    field_1: u32,
    field_2: u32,
    field_3: u32,
    field_4: u32,
    field_5: u32,
    field_6: u32,
    field_7: u32,
    field_8: u32,
    field_9: u32,
    field_10: u32,
    field_11: u32,
    field_12: u32,
    field_13: u32,
    field_14: u32,
    field_15: u32,
    field_16: u32,
    field_17: u32,
    field_18: u32,
    field_19: u32,
    field_20: u32,
    field_21: u32,
    field_22: u32,
    field_23: u32,
    field_24: u32,
    field_25: u32,
    field_26: u32,
    field_27: u32,
    field_28: u32,
    field_29: u32,
    field_30: u32,
    field_31: u32,
    field_32: u32,
    field_33: u32,
    field_34: u32,
    field_35: u32,
    field_36: u32,
    field_37: u32,
    field_38: u32,
    field_39: u32,
    field_40: u32,
    field_41: u32,
    field_42: u32,
    field_43: u32,
    field_44: u32,
    field_45: u32,
    field_46: u32,
    field_47: u32,
    field_48: u32,
    field_49: u32,
    field_50: u32,
    field_51: u32,
    field_52: u32,
    field_53: u32,
    field_54: u32,
    field_55: u32,
    field_56: u32,
    field_57: u32,
    field_58: u32,
    field_59: u32,
    field_60: u32,
    field_61: u32,
    field_62: u32,
    field_63: u32,
}

#[derive(Clone, Default, Reflect)]
struct Struct128 {
    field_0: u32,
    field_1: u32,
    field_2: u32,
    field_3: u32,
    field_4: u32,
    field_5: u32,
    field_6: u32,
    field_7: u32,
    field_8: u32,
    field_9: u32,
    field_10: u32,
    field_11: u32,
    field_12: u32,
    field_13: u32,
    field_14: u32,
    field_15: u32,
    field_16: u32,
    field_17: u32,
    field_18: u32,
    field_19: u32,
    field_20: u32,
    field_21: u32,
    field_22: u32,
    field_23: u32,
    field_24: u32,
    field_25: u32,
    field_26: u32,
    field_27: u32,
    field_28: u32,
    field_29: u32,
    field_30: u32,
    field_31: u32,
    field_32: u32,
    field_33: u32,
    field_34: u32,
    field_35: u32,
    field_36: u32,
    field_37: u32,
    field_38: u32,
    field_39: u32,
    field_40: u32,
    field_41: u32,
    field_42: u32,
    field_43: u32,
    field_44: u32,
    field_45: u32,
    field_46: u32,
    field_47: u32,
    field_48: u32,
    field_49: u32,
    field_50: u32,
    field_51: u32,
    field_52: u32,
    field_53: u32,
    field_54: u32,
    field_55: u32,
    field_56: u32,
    field_57: u32,
    field_58: u32,
    field_59: u32,
    field_60: u32,
    field_61: u32,
    field_62: u32,
    field_63: u32,
    field_64: u32,
    field_65: u32,
    field_66: u32,
    field_67: u32,
    field_68: u32,
    field_69: u32,
    field_70: u32,
    field_71: u32,
    field_72: u32,
    field_73: u32,
    field_74: u32,
    field_75: u32,
    field_76: u32,
    field_77: u32,
    field_78: u32,
    field_79: u32,
    field_80: u32,
    field_81: u32,
    field_82: u32,
    field_83: u32,
    field_84: u32,
    field_85: u32,
    field_86: u32,
    field_87: u32,
    field_88: u32,
    field_89: u32,
    field_90: u32,
    field_91: u32,
    field_92: u32,
    field_93: u32,
    field_94: u32,
    field_95: u32,
    field_96: u32,
    field_97: u32,
    field_98: u32,
    field_99: u32,
    field_100: u32,
    field_101: u32,
    field_102: u32,
    field_103: u32,
    field_104: u32,
    field_105: u32,
    field_106: u32,
    field_107: u32,
    field_108: u32,
    field_109: u32,
    field_110: u32,
    field_111: u32,
    field_112: u32,
    field_113: u32,
    field_114: u32,
    field_115: u32,
    field_116: u32,
    field_117: u32,
    field_118: u32,
    field_119: u32,
    field_120: u32,
    field_121: u32,
    field_122: u32,
    field_123: u32,
    field_124: u32,
    field_125: u32,
    field_126: u32,
    field_127: u32,
}
