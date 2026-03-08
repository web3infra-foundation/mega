use std::collections::HashMap;

const MAX_SCORE: u64 = 100;
const SPAN_SIZE: u64 = 64;

// Aggregated byte weight for one hashed span value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SpanHash {
    hash: u32,
    bytes: u64,
}

// Estimate a Git-like similarity score from raw blob content.
pub fn content_similarity_percent(old_bytes: &[u8], new_bytes: &[u8], minimum_score: u8) -> u8 {
    // Fast paths for exact matches and empty-file cases.
    if old_bytes == new_bytes {
        return 100;
    }
    if old_bytes.is_empty() || new_bytes.is_empty() {
        return 0;
    }

    // Reject pairs that cannot reach the requested threshold because of size skew.
    let minimum_score = minimum_score.min(MAX_SCORE as u8);
    if !passes_size_guard(
        old_bytes.len() as u64,
        new_bytes.len() as u64,
        minimum_score,
    ) {
        return 0;
    }

    // Hash the content into Git-like spans and count the copied source bytes.
    let old_spans = build_span_hashes(old_bytes);
    let new_spans = build_span_hashes(new_bytes);
    let src_copied = count_copied_source_bytes(&old_spans, &new_spans);

    let max_size = old_bytes.len().max(new_bytes.len()) as u64;
    if max_size == 0 {
        return 0;
    }

    ((src_copied * MAX_SCORE) / max_size).min(MAX_SCORE) as u8
}

// Drop pairs whose size gap already rules them out under the minimum score.
fn passes_size_guard(old_size: u64, new_size: u64, minimum_score: u8) -> bool {
    let max_size = old_size.max(new_size) as u128;
    let base_size = old_size.min(new_size) as u128;
    let delta_size = max_size - base_size;
    max_size * (MAX_SCORE as u128 - minimum_score as u128) >= delta_size * MAX_SCORE as u128
}

// Split content into newline-or-64-byte spans and aggregate their hashed byte weights.
fn build_span_hashes(content: &[u8]) -> Vec<SpanHash> {
    let is_text = !is_probably_binary(content);
    let mut spans: HashMap<u32, u64> = HashMap::new();
    let mut accum1 = 0u32;
    let mut accum2 = 0u32;
    let mut span_len = 0u64;
    let mut idx = 0usize;

    while idx < content.len() {
        let byte = content[idx];
        idx += 1;

        // Git ignores CR in CRLF text sequences before hashing spans.
        if is_text && byte == b'\r' && idx < content.len() && content[idx] == b'\n' {
            continue;
        }

        let old_accum1 = accum1;
        accum1 = (accum1 << 7) ^ (accum2 >> 25);
        accum2 = (accum2 << 7) ^ (old_accum1 >> 25);
        accum1 = accum1.wrapping_add(byte as u32);
        span_len += 1;

        if span_len < SPAN_SIZE && (!is_text || byte != b'\n') {
            continue;
        }

        add_span_hash(&mut spans, accum1, accum2, span_len);
        accum1 = 0;
        accum2 = 0;
        span_len = 0;
    }

    if span_len > 0 {
        add_span_hash(&mut spans, accum1, accum2, span_len);
    }

    let mut hashed_spans: Vec<SpanHash> = spans
        .into_iter()
        .map(|(hash, bytes)| SpanHash { hash, bytes })
        .collect();
    hashed_spans.sort_unstable_by_key(|span| span.hash);
    hashed_spans
}

// Fold one completed span into the hash-to-byte-weight table.
fn add_span_hash(spans: &mut HashMap<u32, u64>, accum1: u32, accum2: u32, span_len: u64) {
    let hash = accum1.wrapping_add(accum2.wrapping_mul(0x61));
    *spans.entry(hash).or_default() += span_len;
}

// Count copied source bytes by merging the sorted span hash tables.
fn count_copied_source_bytes(src_spans: &[SpanHash], dst_spans: &[SpanHash]) -> u64 {
    let mut src_idx = 0usize;
    let mut dst_idx = 0usize;
    let mut src_copied = 0u64;

    while src_idx < src_spans.len() && dst_idx < dst_spans.len() {
        let src = src_spans[src_idx];
        let dst = dst_spans[dst_idx];
        match src.hash.cmp(&dst.hash) {
            std::cmp::Ordering::Less => src_idx += 1,
            std::cmp::Ordering::Greater => dst_idx += 1,
            std::cmp::Ordering::Equal => {
                src_copied += src.bytes.min(dst.bytes);
                src_idx += 1;
                dst_idx += 1;
            }
        }
    }

    src_copied
}

// Use a simple NUL-byte heuristic for binary blobs.
fn is_probably_binary(content: &[u8]) -> bool {
    content.contains(&0)
}

#[cfg(test)]
mod tests {
    use super::content_similarity_percent;

    #[test]
    fn identical_content_scores_as_full_match() {
        assert_eq!(
            content_similarity_percent(b"alpha\nbeta\n", b"alpha\nbeta\n", 50),
            100
        );
    }

    #[test]
    fn repeated_lines_keep_their_weight() {
        let old = b"same\nsame\nsame\nsame\nother\n";
        let new = b"same\nsame\nsame\nsame\nchanged\n";

        let score = content_similarity_percent(old, new, 50);

        assert!(
            score >= 70,
            "expected repeated spans to contribute, got {score}"
        );
        assert!(
            score < 100,
            "expected a non-identical file score, got {score}"
        );
    }

    #[test]
    fn size_guard_rejects_large_growth() {
        let old = vec![b'a'; 10];
        let new = vec![b'a'; 100];

        assert_eq!(content_similarity_percent(&old, &new, 50), 0);
    }

    #[test]
    fn binary_content_uses_fixed_size_spans() {
        let old = vec![1u8; 128];
        let mut new = old.clone();
        new[100] = 2;

        assert_eq!(content_similarity_percent(&old, &new, 50), 50);
    }

    #[test]
    fn binary_newlines_do_not_split_fixed_size_spans() {
        let mut old = vec![1u8; 128];
        old[0] = 0;
        for idx in (7..128).step_by(8) {
            old[idx] = b'\n';
        }

        let mut new = old.clone();
        new[40] = 2;

        assert_eq!(content_similarity_percent(&old, &new, 50), 50);
    }

    #[test]
    fn crlf_text_matches_lf_text() {
        let old = b"line1\r\nline2\r\n";
        let new = b"line1\nline2\n";

        let score = content_similarity_percent(old, new, 50);

        assert!(
            score >= 80,
            "expected CRLF/LF normalization to stay close, got {score}"
        );
        assert!(
            score < 100,
            "raw-size scoring should keep CRLF/LF from being exact"
        );
    }
}
