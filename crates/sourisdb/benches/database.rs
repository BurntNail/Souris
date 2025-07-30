use criterion::{black_box, criterion_group, criterion_main, Criterion};
use sourisdb::store::Store;

const EXAMPLE_JSON: &str = include_str!("smallexampledata.json");

fn ser_and_deser(c: &mut Criterion) {
    let json = serde_json::from_str(EXAMPLE_JSON).unwrap();
    let example = Store::from_json(json).unwrap();
    let sered = example.ser();

    c.bench_function("serialise_store", |b| b.iter(|| black_box(example.ser())));

    c.bench_function("deserialise_store", |b| {
        b.iter(|| black_box(Store::deser(&sered)))
    });
}

criterion_group!(benches, ser_and_deser);
criterion_main!(benches);
