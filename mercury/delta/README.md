# Git Delta

In Git, delta refers to the differences or changes between files or data objects. It is a measure of the amount of change between two versions. By using delta, Git can more efficiently store and transfer changes to files or data objects.

## Example

This module exposes three functions to the outside world:

```rust
pub fn delta_decode(mut stream : &mut impl Read,base_info: &Vec<u8>) -> Result<Vec<u8>, GitDeltaError>

pub fn delta_encode_rate(old_data: & [u8], new_data: & [u8]) -> f64

pub fn delta_encode(old_data: & [u8], new_data: & [u8]) -> Vec<u8>
```

If you want to decode a delta data, you need a base data(base_info) and a delta instruction data(stream).

```rust
use delta;

let delta_result:Result<Vec<u8>, GitDeltaError> = delta::delta_decode(stream, base_info);
```

On the contrary, if you want to compress another object in delta form based on it, use the encode function

```rust
use delta;

let delta_data:Vec<u8> = delta::delta_encode(old_data, new_data) ;
```

In addition, the `delta::delta_encode_rate` function can represent the compression rate of delta