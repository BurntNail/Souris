use criterion::{black_box, criterion_group, criterion_main, Criterion};
use sourisdb::{
    types::binary::rle::rle,
    utilities::{cursor::Cursor},
};
use sourisdb::types::binary::rle::un_rle;

const EXAMPLE_JSON: &str = include_str!("exampledata.json");

fn rle_and_un_rle(c: &mut Criterion) {

    c.bench_function("serialise rle", |b| {
        let binary_data = EXAMPLE_JSON.as_bytes().to_vec();
        b.iter(|| {
            let rle = rle(binary_data.clone());
            black_box(rle);
        });
    });

    c.bench_function("deserialise rle", |b| {
        let binary_data = EXAMPLE_JSON.as_bytes().to_vec();

        let encoded = rle(binary_data);
        let mut cursor = Cursor::new(&encoded);

        b.iter(|| {
            let decoded = un_rle(&mut cursor).unwrap();
            black_box(decoded);
            cursor.set_pos(0);
        });
    });
}

criterion_group!(compression, rle_and_un_rle);
criterion_main!(compression);
