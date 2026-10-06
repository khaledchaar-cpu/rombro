//! Byte-level scan progress: readers report while they hash, so large files move the bar.

use crate::scan::ScanTick;
use std::io::{self, Read};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

pub(crate) struct Meter<'a> {
    done: AtomicUsize,
    total: usize,
    bytes: AtomicU64,
    bytes_total: u64,
    progress: &'a (dyn Fn(ScanTick<'_>) + Sync),
}

impl<'a> Meter<'a> {
    pub(crate) fn new(
        total: usize,
        bytes_total: u64,
        progress: &'a (dyn Fn(ScanTick<'_>) + Sync),
    ) -> Self {
        let m = Self {
            done: AtomicUsize::new(0),
            total,
            bytes: AtomicU64::new(0),
            bytes_total,
            progress,
        };
        m.emit(0, 0, None);
        m
    }

    fn emit(&self, done: usize, bytes: u64, item: Option<&str>) {
        (self.progress)(ScanTick {
            done,
            total: self.total,
            bytes: bytes.min(self.bytes_total),
            bytes_total: self.bytes_total,
            item,
        });
    }

    /// Counts `n` bytes read without finishing an item.
    fn add(&self, n: u64, item: &str) {
        let bytes = self.bytes.fetch_add(n, Ordering::Relaxed) + n;
        self.emit(self.done.load(Ordering::Relaxed), bytes, Some(item));
    }

    /// Finishes an item of `size` bytes of which `read` were already counted.
    pub(crate) fn finish(&self, size: u64, read: u64) {
        let n = size.saturating_sub(read);
        let bytes = self.bytes.fetch_add(n, Ordering::Relaxed) + n;
        self.emit(self.done.fetch_add(1, Ordering::Relaxed) + 1, bytes, None);
    }
}

/// Bytes one item has counted so far; capped at the item's size by the caller via `limit`.
#[derive(Default)]
pub(crate) struct Counter<'m, 'a> {
    meter: Option<&'m Meter<'a>>,
    read: AtomicU64,
    limit: u64,
    name: &'m str,
}

impl<'m, 'a> Counter<'m, 'a> {
    pub(crate) fn new(meter: &'m Meter<'a>, limit: u64, name: &'m str) -> Self {
        Self {
            meter: Some(meter),
            read: AtomicU64::new(0),
            limit,
            name,
        }
    }

    pub(crate) fn read(&self) -> u64 {
        self.read.load(Ordering::Relaxed)
    }

    fn count(&self, n: u64) {
        let Some(m) = self.meter else { return };
        let before = self.read.fetch_add(n, Ordering::Relaxed);
        // CHD tracks decompress to more than the file size; never count past it.
        let add = (before + n)
            .min(self.limit)
            .saturating_sub(before.min(self.limit));
        if add > 0 {
            m.add(add, self.name);
        }
    }

    pub(crate) fn wrap<R: Read>(&self, inner: R) -> Counted<'_, 'm, 'a, R> {
        Counted {
            inner,
            counter: self,
        }
    }
}

pub(crate) struct Counted<'c, 'm, 'a, R> {
    inner: R,
    counter: &'c Counter<'m, 'a>,
}

impl<R: Read> Read for Counted<'_, '_, '_, R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = self.inner.read(buf)?;
        self.counter.count(n as u64);
        Ok(n)
    }
}
