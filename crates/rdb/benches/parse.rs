//! Parses the real SNES RDB if present (no fixture is checked in).
use criterion::{Criterion, criterion_group, criterion_main};
use romburak_rdb::RdbFile;
use std::path::PathBuf;

fn bench(c: &mut Criterion) {
    let Some(home) = std::env::var_os("HOME") else {
        return;
    };
    let path = PathBuf::from(home)
        .join(".config/retroarch/database/rdb/Nintendo - Super Nintendo Entertainment System.rdb");
    let Ok(rdb) = RdbFile::open(path) else { return };
    c.bench_function("snes_entries", |b| {
        b.iter(|| rdb.entries().filter(Result::is_ok).count())
    });
}

criterion_group!(benches, bench);
criterion_main!(benches);
