use criterion::{black_box, criterion_group, criterion_main, Criterion};
use sourisdb::{
    types::binary::rle::{rle, un_rle},
    utilities::cursor::Cursor,
};
use sourisdb::types::binary::lz::{lz, un_lz};

const EXAMPLE_JSON: &str = include_str!("exampledata.json");

fn rle_and_un_rle(c: &mut Criterion) {
    c.bench_function("rle", |b| {
        let binary_data = EXAMPLE_JSON.as_bytes().to_vec();
        b.iter(|| {
            let rle = rle(&binary_data);
            black_box(rle);
        });
    });

    c.bench_function("un-rle", |b| {
        let binary_data = EXAMPLE_JSON.as_bytes().to_vec();

        let encoded = rle(&binary_data);
        let mut cursor = Cursor::new(&encoded);

        b.iter(|| {
            let decoded = un_rle(&mut cursor).unwrap();
            black_box(decoded);
            cursor.set_pos(0);
        });
    });
}

fn lz_and_un_lz (c: &mut Criterion) {
    c.bench_function("lz", |b| {
        let binary_data = EXAMPLE_JSON.as_bytes().to_vec();
        b.iter(|| {
            let lz = lz(&binary_data);
            black_box(lz);
        });
    });

    c.bench_function("un-lz", |b| {
        let binary_data = EXAMPLE_JSON.as_bytes().to_vec();
    
        let encoded = lz(&binary_data);
        let mut cursor = Cursor::new(&encoded);
    
        b.iter(|| {
            let decoded = un_lz(&mut cursor).unwrap();
            black_box(decoded);
            cursor.set_pos(0);
        })
    });
}

criterion_group!(compression, rle_and_un_rle, lz_and_un_lz);
criterion_main!(compression);
