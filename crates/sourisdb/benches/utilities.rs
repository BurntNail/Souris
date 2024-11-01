use criterion::{black_box, criterion_group, criterion_main, Criterion};
use sourisdb::utilities::{bits::Bits, cursor::Cursor, huffman::Huffman};

const EXAMPLE_DATA: &str = include_str!("./exampledata.json");
const EXAMPLE_DATA_LINES: usize = usize::MAX;

fn en_de_code_beemovie(c: &mut Criterion) {
    c.bench_function("create huffman", |b| {
        b.iter(|| {
            let huff = unsafe { Huffman::new_str(EXAMPLE_DATA).unwrap_unchecked() };
            black_box(huff);
        })
    });

    c.bench_function("encode huffman", |b| {
        let huff = unsafe { Huffman::new_str(EXAMPLE_DATA).unwrap_unchecked() };
        b.iter(|| {
            for line in EXAMPLE_DATA.lines().take(EXAMPLE_DATA_LINES) {
                let encoded = huff.encode_string(line).unwrap();
                black_box(encoded);
            }
        })
    });

    c.bench_function("decode huffman", |b| {
        let huff = unsafe { Huffman::new_str(EXAMPLE_DATA).unwrap_unchecked() };
        let data: Bits = EXAMPLE_DATA
            .lines()
            .take(EXAMPLE_DATA_LINES)
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
        let huff = Huffman::new_str(EXAMPLE_DATA).unwrap();

        b.iter(|| {
            let sered = huff.ser();
            black_box(sered);
        });
    });

    c.bench_function("deserialise huffman", |b| {
        let huff = Huffman::new_str(EXAMPLE_DATA).unwrap();
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
        let huff = unsafe { Huffman::new_str(EXAMPLE_DATA).unwrap_unchecked() };
        let data: Bits = EXAMPLE_DATA
            .lines()
            .take(EXAMPLE_DATA_LINES)
            .flat_map(|l| huff.encode_string(l))
            .collect();

        b.iter(|| {
            let sered = data.ser();
            black_box(sered);
        })
    });

    c.bench_function("deserialise bits", |b| {
        let huff = unsafe { Huffman::new_str(EXAMPLE_DATA).unwrap_unchecked() };
        let data: Bits = EXAMPLE_DATA
            .lines()
            .take(EXAMPLE_DATA_LINES)
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
