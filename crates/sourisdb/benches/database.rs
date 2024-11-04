use criterion::{black_box, criterion_group, criterion_main, Criterion};
use sourisdb::store::Store;

const EXAMPLE_JSON: &str = include_str!("exampledata.json");

fn ser_and_deser(c: &mut Criterion) {
    let json = serde_json::from_str(EXAMPLE_JSON)
        .expect("Failed to parse example JSON data");
    let example = Store::from_json(json)
        .expect("Failed to create Store from JSON");
    let sered = example.ser().unwrap();

    c.bench_function("serialise_store", |b| b.iter(|| black_box(example.ser())));

    c.bench_function("deserialise_store", |b| {
        b.iter(|| black_box(Store::deser(&sered)))
    });
}

criterion_group!(benches, ser_and_deser);
criterion_main!(benches);
