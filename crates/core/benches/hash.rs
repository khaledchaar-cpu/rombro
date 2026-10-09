use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use romburak_core::hash_reader;

fn bench(c: &mut Criterion) {
    let data: Vec<u8> = (0..32 * 1024 * 1024u32)
        .map(|i| (i * 2_654_435_761) as u8)
        .collect();
    let mut g = c.benchmark_group("hash");
    g.throughput(Throughput::Bytes(data.len() as u64));
    g.sample_size(10);
    g.bench_function("crc+sha1 32MiB", |b| {
        b.iter(|| hash_reader(&data[..], None, false))
    });
    g.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
