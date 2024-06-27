use criterion::{black_box, criterion_group, criterion_main, Criterion};
use sourisdb::utilities::{bits::Bits, huffman::Huffman};

const BEE_MOVIE: &'static str = include_str!("./beemoviescript.txt");

fn bee_movie(c: &mut Criterion) {
    c.bench_function("create beemovie", |b| {
        b.iter(|| {
            let huff = unsafe { Huffman::new_str(BEE_MOVIE).unwrap_unchecked() };
            black_box(huff);
        })
    });

    c.bench_function("encode first 500 beemovie", |b| {
        let huff = unsafe { Huffman::new_str(BEE_MOVIE).unwrap_unchecked() };
        b.iter(|| {
            for line in BEE_MOVIE.lines().take(500) {
                let encoded = huff.encode_string(line);
                black_box(encoded);
            }
        })
    });

    c.bench_function("decode first 500 beemovie", |b| {
        let huff = unsafe { Huffman::new_str(BEE_MOVIE).unwrap_unchecked() };
        let data: Vec<Bits> = BEE_MOVIE.lines().take(500).flat_map(|l| huff.encode_string(l)).collect();

        b.iter(|| {
            for line in &data {
                let decoded = huff.decode_string(line);
                black_box(line);
            }
        })
    });
}

criterion_group!(benches, bee_movie);
criterion_main!(benches);