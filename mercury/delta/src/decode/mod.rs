use crate::{errors::GitDeltaError, utils};
use std::io::{ErrorKind, Read};

const COPY_INSTRUCTION_FLAG: u8 = 1 << 7;
const COPY_OFFSET_BYTES: u8 = 4;
const COPY_SIZE_BYTES: u8 = 3;
const COPY_ZERO_SIZE: usize = 0x10000;

pub fn delta_decode(
    mut stream: &mut impl Read,
    base_info: &[u8],
) -> Result<Vec<u8>, GitDeltaError> {
    // Read the bash object size & Result Size
    let base_size = utils::read_size_encoding(&mut stream).unwrap();
    if base_info.len() != base_size {
        return Err(GitDeltaError::DeltaDecoderError(
            "base object len is not equal".to_owned(),
        ));
    }

    let result_size = utils::read_size_encoding(&mut stream).unwrap();
    let mut buffer = Vec::with_capacity(result_size);
    loop {
        // Check if the stream has ended, meaning the new object is done
        let instruction = match utils::read_bytes(stream) {
            Ok([instruction]) => instruction,
            Err(err) if err.kind() == ErrorKind::UnexpectedEof => break,
            Err(err) => {
                panic!(
                    "{}",
                    GitDeltaError::DeltaDecoderError(format!(
                        "Wrong instruction in delta :{}",
                        err
                    ))
                );
            }
        };

        if instruction & COPY_INSTRUCTION_FLAG == 0 {
            // Data instruction; the instruction byte specifies the number of data bytes
            if instruction == 0 {
                // Appending 0 bytes doesn't make sense, so git disallows it
                panic!(
                    "{}",
                    GitDeltaError::DeltaDecoderError(String::from("Invalid data instruction"))
                );
            }

            // Append the provided bytes
            let mut data = vec![0; instruction as usize];
            stream.read_exact(&mut data).unwrap();
            buffer.extend_from_slice(&data);
        // result.extend_from_slice(&data);
        } else {
            // Copy instruction
            let mut nonzero_bytes = instruction;
            let offset =
                utils::read_partial_int(&mut stream, COPY_OFFSET_BYTES, &mut nonzero_bytes)
                    .unwrap();
            let mut size =
                utils::read_partial_int(&mut stream, COPY_SIZE_BYTES, &mut nonzero_bytes).unwrap();
            if size == 0 {
                // Copying 0 bytes doesn't make sense, so git assumes a different size
                size = COPY_ZERO_SIZE;
            }
            // Copy bytes from the base object
            let base_data = base_info.get(offset..(offset + size)).ok_or_else(|| {
                GitDeltaError::DeltaDecoderError("Invalid copy instruction".to_string())
            });

            match base_data {
                Ok(data) => buffer.extend_from_slice(data),
                Err(e) => return Err(e),
            }
        }
    }
    assert!(buffer.len() == result_size);
    Ok(buffer)
}
