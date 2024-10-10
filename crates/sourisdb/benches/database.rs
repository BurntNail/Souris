use criterion::{black_box, criterion_group, criterion_main, Criterion};
use sourisdb::store::Store;

const EXAMPLE: &'static [u8] = include_bytes!("example.sdb");

fn ser_and_deser(c: &mut Criterion) {
    let example = Store::deser(EXAMPLE).unwrap();


    c.bench_function("serialise_store", |b| {
        b.iter(|| black_box(example.ser()))
    });

    c.bench_function("deserialise_store", |b| {
        b.iter(|| black_box(Store::deser(EXAMPLE)))
    });
}

criterion_group!(benches, ser_and_deser);
criterion_main!(benches);