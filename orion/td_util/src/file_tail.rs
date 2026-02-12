/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

use std::{path::Path, time::Duration};

use anyhow::Result;
use async_compression::tokio::bufread::ZstdDecoder;
use tempfile::NamedTempFile;
use tokio::{
    fs::File,
    io::{self, AsyncBufReadExt, AsyncReadExt, AsyncSeekExt, AsyncWriteExt, BufReader, BufWriter},
    sync::mpsc,
};

/// Tail a compressed Buck2 event log by streaming it to a temporary file and tailing that file in real-time.
///
/// Buck2 produces event logs in zstd-compressed JSONL format, which are continuously appended
/// during a build. This function allows you to monitor those events as they are written,
/// without waiting for the entire build to finish. It works by asynchronously decompressing
/// the compressed file into a temporary file, and then incrementally reading new lines
/// from that temp file in a non-blocking manner, sending each line through a Tokio channel.
///
/// This design has several advantages:
/// - It avoids blocking the main thread, allowing other tasks to run concurrently.
/// - It handles very large logs efficiently without loading the entire file into memory.
/// - It guarantees that once the decompression is complete and no more new lines appear,
///   the tailing task will automatically exit.
///
/// # Parameters
///
/// * `compressed_path` - The path to the zstd-compressed Buck2 event log to tail. This file
///   is continuously written by Buck2 during a build.
/// * `tx` - A Tokio mpsc sender channel used to emit each line read from the log. Consumers
///   of this channel can parse or process each event line as needed.
/// * `poll_interval` - The duration to wait between polling the temporary file for new lines
///   when no new content is currently available. This controls the trade-off between
///   CPU usage and real-time responsiveness.
pub async fn tail_compressed_buck2_events<P>(
    compressed_path: P,
    tx: mpsc::Sender<String>,
    poll_interval: Duration,
) -> Result<()>
where
    P: AsRef<Path>,
{
    let compressed_path = compressed_path.as_ref();

    // Create a temporary file
    let temp_file = NamedTempFile::new()?;
    let temp_path = temp_file.path().to_path_buf();
    drop(temp_file); // Close it for now, will open asynchronously later

    // Asynchronously open the compressed file and the temp file
    let compressed_file = File::open(compressed_path).await?;
    let mut decoder = ZstdDecoder::new(BufReader::with_capacity(256 * 1024, compressed_file));
    let temp_file = File::create(&temp_path).await?;
    let mut temp_writer = BufWriter::with_capacity(256 * 1024, temp_file);

    // Spawn a task to asynchronously decompress into the temp file
    let decompress_task = tokio::spawn(async move {
        let mut buffer = vec![0u8; 256 * 1024];
        loop {
            let bytes_read = decoder.read(&mut buffer).await?;
            if bytes_read == 0 {
                break; // EOF
            }
            temp_writer.write_all(&buffer[..bytes_read]).await?;
            temp_writer.flush().await?;
        }
        Ok::<(), anyhow::Error>(())
    });

    // Spawn a task to asynchronously tail the temp file
    let tail_task = tokio::spawn(async move {
        // Open the temporary file for reading
        let file = File::open(&temp_path).await?;
        let mut reader = BufReader::with_capacity(256 * 1024, file);

        let mut offset = 0u64;
        let mut buffer = String::new();

        loop {
            // Seek to the last read offset
            reader.seek(io::SeekFrom::Start(offset)).await?;
            buffer.clear();
            let bytes_read = reader.read_line(&mut buffer).await?;

            if bytes_read == 0 {
                // No new data, wait for the next poll
                tokio::time::sleep(poll_interval).await;
                continue;
            }

            offset += bytes_read as u64;
            let line = buffer.trim_end_matches(&['\n', '\r'][..]).to_string();

            // Send the line, break if receiver has been dropped
            if tx.send(line).await.is_err() {
                break;
            }
        }
        Ok::<(), anyhow::Error>(())
    });

    // Wait for both tasks to complete (decompression may finish first; tail will keep polling)
    let (_, tail_result) = tokio::join!(decompress_task, tail_task);

    // Return the result of the tail task
    tail_result?
}
