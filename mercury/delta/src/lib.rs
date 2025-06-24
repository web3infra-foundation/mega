use encode::DeltaDiff;

mod decode;
mod encode;
mod errors;
mod utils;

pub use decode::delta_decode as decode;

pub fn encode_rate(old_data: &[u8], new_data: &[u8]) -> f64 {
    let differ = DeltaDiff::new(old_data, new_data);
    differ.get_ssam_rate()
}

pub fn encode(old_data: &[u8], new_data: &[u8]) -> Vec<u8> {
    let differ = DeltaDiff::new(old_data, new_data);
    differ.encode()
}

#[cfg(test)]
mod tests {}
