use std::hash::{DefaultHasher, Hash, Hasher};
use encode::DeltaDiff;
use rayon::prelude::*;

mod decode;
mod encode;
mod errors;
mod utils;

pub use decode::delta_decode as decode;

const SAMPLE_STEP: usize = 64;      // 每隔 64 字节取样
const MIN_DELTA_RATE: f64 = 0.5;    // 最小 delta rate

/// 计算两个对象的 delta rate（近似）
pub fn heuristic_encode_rate(old_data: &[u8], new_data: &[u8]) -> f64 {
    let old_len = old_data.len();
    let new_len = new_data.len();

    if old_len == 0 && new_len == 0 {
        return 1.0; // 两个空 slice 完全匹配
    }
    if old_len == 0 || new_len == 0 {
        return 0.0; // 一个空一个非空，匹配率为 0
    }

    let step = SAMPLE_STEP;
    let mut match_count = 0;
    let mut sample_count = 0;

    // 对两个对象取样，计算 hash 匹配数
    let min_len = old_len.min(new_len);

    let total_samples = (min_len + step - 1) / step;
    let mut i = 0;
    while i < min_len {
        let old_chunk = &old_data[i..(i + step).min(old_len)];
        let new_chunk = &new_data[i..(i + step).min(new_len)];

        if hash_chunk(old_chunk) == hash_chunk(new_chunk) {
            match_count += 1;
        }
        sample_count += 1;

        // 早停判断：
        // 剩余样本即使全匹配也达不到 MIN_DELTA_RATE，则提前返回 0
        let remaining_samples = total_samples - sample_count;
        let max_possible_rate = (match_count + remaining_samples) as f64 / total_samples as f64;
        if max_possible_rate < MIN_DELTA_RATE {
            return 0.0;
        }

        i += step;
    }

    // 返回匹配率
    match_count as f64 / sample_count as f64
}

fn hash_chunk(chunk: &[u8]) -> u64 {
    let mut hasher = DefaultHasher::new();
    chunk.hash(&mut hasher);
    hasher.finish()
}

pub fn heuristic_encode_rate_parallel(old_data: &[u8], new_data: &[u8]) -> f64 {
    let old_len = old_data.len();
    let new_len = new_data.len();

    if old_len == 0 && new_len == 0 { return 1.0; }
    if old_len == 0 || new_len == 0 { return 0.0; }

    let min_len = old_len.min(new_len);

    let step = if min_len > 1_000_000 {
        512
    } else if min_len > 100_000 {
        128
    } else {
        16
    };

    let chunks: Vec<_> = old_data[..min_len].chunks(step)
        .zip(new_data[..min_len].chunks(step))
        .collect();

    let match_count: usize = chunks.par_iter()
        .filter(|(a, b)| a == b)
        .count();

    let rate = match_count as f64 / chunks.len() as f64;
    if rate < MIN_DELTA_RATE { 0.0 } else { rate }
}

pub fn encode_rate(old_data: &[u8], new_data: &[u8]) -> f64 {
    let differ = DeltaDiff::new(old_data, new_data);
    differ.get_ssam_rate()
}

pub fn encode(old_data: &[u8], new_data: &[u8]) -> Vec<u8> {
    let differ = DeltaDiff::new(old_data, new_data);
    differ.encode()
}

#[cfg(test)]
mod tests {
    use crate::{ encode_rate, heuristic_encode_rate, heuristic_encode_rate_parallel};

    #[test]
    fn test_heuristic_encode_rate() {
        // 完全相同的数据
        let data1 = b"hello world, this is a test for delta rate";
        let data2 = b"hello world, this is a test for delta rate";
        let rate = heuristic_encode_rate(data1, data2);
        println!("rate = {}", rate);
        assert!((rate - 1.0).abs() < 1e-6, "Expected 1.0 for identical data");

        // 部分相似的数据
        let data3 = b"hello world, this is a test for delta rate";
        let data4 = b"hello worll, this is a test for delta rate";
        let rate = encode_rate(data3, data4);
        println!("rate = {}", rate);
        assert!(rate > 0.5 && rate < 1.0, "Expected partial match for similar data");

        // 完全不同的数据
        let data5 = b"abcdefghijklmno";
        let data6 = b"1234567890!@#";
        let rate = heuristic_encode_rate(data5, data6);
        println!("rate = {}", rate);
        assert!(rate < 0.2, "Expected low match rate for different data");

        // 空数据
        let data7 = b"";
        let data8 = b"";
        let rate = heuristic_encode_rate(data7, data8);
        println!("rate = {}", rate);
        assert_eq!(rate, 1.0, "Empty slices should be fully matching");

        // 一个空一个非空
        let rate = heuristic_encode_rate(data7, data1);
        assert_eq!(rate, 0.0, "Empty vs non-empty should give 0 rate");
    }

    #[test]
    fn test_heuristic_encode_rate_large_files() {
        // 大文件完全不匹配 → 触发早停
        let data1 = vec![0u8; 1_000_00];
        let data2 = vec![1u8; 1_000_00];
        let rate = heuristic_encode_rate(&data1, &data2);
        println!("Large non-matching data rate = {}", rate);
        assert_eq!(rate, 0.0, "Large completely different data should early stop with 0 rate");

        // 大文件部分匹配 → 保留部分匹配
        let  data3 = vec![0u8; 100];
        let mut data4 = vec![0u8; 100];
        // 在一部分修改一些数据
        for i in 0..2 {
            data4[i] = 1;
        }
        let rate1 = heuristic_encode_rate_parallel(&data3, &data4);
        let rate2 = encode_rate(&data3, &data4);
        println!("Large partially matching data rate = {}, accurate rate = {}", rate1,rate2);
        
        assert!((rate2-rate1).abs() < 0.2, "Large partially matching data should preserve partial rate");
    }
}
