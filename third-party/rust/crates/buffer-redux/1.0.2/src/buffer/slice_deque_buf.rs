//! Move-free buffer and reader utilizing the [`slice-ring-buffer`] crate.
//!
//! These types are only available on target platforms with virtual memory support,
//! namely Windows, OS X and Linux.
//!
//! [`slice-ring-buffer`]: https://crates.io/crates/slice-ring-buffer
use slice_ring_buffer::SliceRingBuffer as SliceDeque;

use std::{cmp, mem::MaybeUninit};

pub struct SliceDequeBuf {
    deque: SliceDeque<u8>,
}

/// Move-free buffer utilizing the [`slice-deque`] crate.
///
/// Its usable space will always be equal to its free space.
///
/// This is only available on target platforms with virtual memory support,
/// namely Windows, OS X and Linux.
impl SliceDequeBuf {
    pub fn with_capacity(cap: usize) -> Self {
        SliceDequeBuf {
            deque: SliceDeque::with_capacity(cap),
        }
    }

    pub fn capacity(&self) -> usize {
        self.deque.capacity()
    }

    pub fn len(&self) -> usize {
        self.deque.len()
    }

    pub fn usable_space(&self) -> usize {
        self.capacity() - self.len()
    }

    pub fn reserve(&mut self, additional: usize) -> bool {
        self.deque.reserve(additional);
        true
    }

    /// This method is a no-op.
    pub fn make_room(&mut self) {}

    #[inline]
    pub fn buf(&self) -> &[u8] {
        &self.deque
    }

    pub fn buf_mut(&mut self) -> &mut [u8] {
        &mut self.deque
    }

    pub unsafe fn write_buf(&mut self) -> &mut [MaybeUninit<u8>] {
        self.deque.tail_head_slice()
    }

    pub unsafe fn bytes_written(&mut self, add: usize) {
        let offset = cmp::min(add, self.usable_space()) as isize;

        if offset < 0 {
            panic!("BufImpl.bytes_written() arg overflowed isize: {add:x}");
        }

        self.deque.move_tail(offset);
    }

    pub fn consume(&mut self, amt: usize) {
        unsafe {
            let offset = cmp::min(amt, self.len()) as isize;

            if offset < 0 {
                panic!("BufImpl.consume() arg overflowed isize: {amt:x}")
            }

            self.deque.move_head(offset);
        }
    }
}
