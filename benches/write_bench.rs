use criterion::{black_box, criterion_group, criterion_main, Criterion};
use scribe::{LogFrame, LogLevel};

fn bench_frame_serialize(c: &mut Criterion) {
    let frame = LogFrame::new(
        LogLevel::Info,
        "benchmark".to_string(),
        "test message for benchmarking".to_string(),
    );

    c.bench_function("frame_serialize", |b| {
        b.iter(|| {
            black_box(frame.serialize().unwrap());
        });
    });
}

fn bench_frame_deserialize(c: &mut Criterion) {
    let frame = LogFrame::new(
        LogLevel::Info,
        "benchmark".to_string(),
        "test message for benchmarking".to_string(),
    );
    let serialized = frame.serialize().unwrap();

    c.bench_function("frame_deserialize", |b| {
        b.iter(|| {
            black_box(LogFrame::deserialize(&serialized).unwrap());
        });
    });
}

criterion_group!(benches, bench_frame_serialize, bench_frame_deserialize);
criterion_main!(benches);
