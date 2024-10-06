use criterion::{black_box, criterion_group, criterion_main, Criterion};

use sourisdb::utilities::{bits::Bits, cursor::Cursor, huffman::Huffman};

const BEE_MOVIE: &str = include_str!("./beemoviescript.txt");
const BEE_MOVIE_LINES: usize = usize::MAX;

fn en_de_code_beemovie(c: &mut Criterion) {
    c.bench_function("create huffman", |b| {
        b.iter(|| {
            let huff = unsafe { Huffman::new_str(BEE_MOVIE).unwrap_unchecked() };
            black_box(huff);
        })
    });

    c.bench_function("encode huffman", |b| {
        let huff = unsafe { Huffman::new_str(BEE_MOVIE).unwrap_unchecked() };
        b.iter(|| {
            for line in BEE_MOVIE.lines().take(BEE_MOVIE_LINES) {
                let encoded = huff.encode_string(line).unwrap();
                black_box(encoded);
            }
        })
    });

    c.bench_function("decode huffman", |b| {
        let huff = unsafe { Huffman::new_str(BEE_MOVIE).unwrap_unchecked() };
        let data: Bits = BEE_MOVIE
            .lines()
            .take(BEE_MOVIE_LINES)
            .flat_map(|l| huff.encode_string(l))
            .collect();

        b.iter(|| {
            let decoded = huff.decode_string(data.clone()).unwrap();
            black_box(decoded);
        })
    });
}

fn ser_de_huffman(c: &mut Criterion) {
    c.bench_function("serialise huffman", |b| {
        let huff = Huffman::new_str(BEE_MOVIE).unwrap();

        b.iter(|| {
            let sered = huff.ser();
            black_box(sered);
        });
    });

    c.bench_function("deserialise huffman", |b| {
        let huff = Huffman::new_str(BEE_MOVIE).unwrap();
        let sered = huff.ser();

        b.iter(|| {
            let mut cursor = Cursor::new(&sered);
            let desered = Huffman::deser(&mut cursor).unwrap();
            black_box(desered);
        });
    });
}

fn ser_de_bits(c: &mut Criterion) {
    c.bench_function("serialise bits", |b| {
        let huff = unsafe { Huffman::new_str(BEE_MOVIE).unwrap_unchecked() };
        let data: Bits = BEE_MOVIE
            .lines()
            .take(BEE_MOVIE_LINES)
            .flat_map(|l| huff.encode_string(l))
            .collect();

        b.iter(|| {
            let sered = data.ser();
            black_box(sered);
        })
    });

    c.bench_function("deserialise bits", |b| {
        let huff = unsafe { Huffman::new_str(BEE_MOVIE).unwrap_unchecked() };
        let data: Bits = BEE_MOVIE
            .lines()
            .take(BEE_MOVIE_LINES)
            .flat_map(|l| huff.encode_string(l))
            .collect();
        let sered = data.ser();

        b.iter(|| {
            let mut cursor = Cursor::new(&sered);
            let decoded = Bits::deser(&mut cursor).unwrap();
            black_box(decoded);
        })
    });
}

criterion_group!(runtime, en_de_code_beemovie);
criterion_group!(serde, ser_de_huffman, ser_de_bits);
criterion_main!(runtime, serde);
