#[macro_export]
macro_rules! static_array {
    ($type: ty, [$($elem:expr),* $(,)?]) => {{
        const LEN: usize = 0 $(+ { let _ = $elem; 1 })*;
        const ARRAY: [$type; LEN] = [$($elem),*];
        ARRAY
    }};
}
