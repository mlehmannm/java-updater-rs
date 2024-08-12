//! Checksum.
//!
//! This module contains code to create a checksum (SHA256) "on the fly" while writing data.

use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{self, Result as IoResult, Write};
use std::path::Path;

// Calculates the checksum (SHA256) for the given file.
pub(crate) fn checksum(path: &Path) -> IoResult<String> {
    let mut dest_file = File::open(path)?;
    let mut hasher = Sha256::new();
    io::copy(&mut dest_file, &mut hasher)?;
    let hash = hasher.finalize();
    let checksum = base16ct::lower::encode_string(&hash);

    Ok(checksum)
}

/// The struct to create the checksum (SHA256) "on the fly".
pub(crate) struct ChecksumWrite<W> {
    hasher: Sha256,
    write: W,
}

impl<W: Write> ChecksumWrite<W> {
    /// Creates a new `ChecksumWrite` on top of the given [Write].
    pub(crate) fn new(write: W) -> Self {
        Self { hasher: Sha256::new(), write }
    }

    /// Returns the checksum and consume the `ChecksumWrite`.
    pub(crate) fn checksum(mut self) -> IoResult<String> {
        self.flush()?;
        let hash = self.hasher.finalize();
        let checksum = base16ct::lower::encode_string(&hash);

        Ok(checksum)
    }
}

impl<W: Write> Write for ChecksumWrite<W> {
    fn write(&mut self, buf: &[u8]) -> IoResult<usize> {
        let n = self.write.write(buf)?;
        self.hasher.write_all(&buf[..n])?;

        Ok(n)
    }

    fn flush(&mut self) -> IoResult<()> {
        let x = self.write.flush();
        let y = self.hasher.flush();

        x.and(y)
    }
}
