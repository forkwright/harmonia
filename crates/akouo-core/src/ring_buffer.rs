use std::cell::UnsafeCell;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Lock-free SPSC (single-producer, single-consumer) ring buffer for f64 audio samples.
///
/// # Safety
///
/// The buffer is safe only when used with exactly one producer calling `push_frame` and
/// exactly one consumer calling `pop_frame`, never concurrently on the same side. This is
/// the standard SPSC contract: separate `&RingBuffer` references held by producer and
/// consumer threads are safe because the atomic read/write positions coordinate access to
/// non-overlapping slots.
///
/// The backing store uses `UnsafeCell<f64>` per slot. The producer exclusively writes INTO
/// slots in the range `[write_pos, write_pos + count)` and publishes via a Release store to
/// `write_pos`. The consumer exclusively reads FROM slots in `[read_pos, read_pos + count)`
/// after an Acquire load of `write_pos` to synchronise.
pub struct RingBuffer {
    data: Box<[UnsafeCell<f64>]>,
    /// Capacity, always a power of two, so `& (capacity - 1)` replaces modulo.
    capacity: usize,
    /// Monotonically increasing read position. Only advanced by the consumer.
    read_pos: AtomicUsize,
    /// Monotonically increasing write position. Only advanced by the producer.
    write_pos: AtomicUsize,
}

// SPSC contract: one producer + one consumer, never same-side concurrent access.
unsafe impl Send for RingBuffer {}
unsafe impl Sync for RingBuffer {}

impl RingBuffer {
    /// Allocates a ring buffer with at least `min_capacity` slots, rounded up to the next
    /// power of two. No allocation occurs after this call.
    pub fn new(min_capacity: usize) -> Self {
        let capacity = min_capacity.next_power_of_two().max(2);
        let data = (0..capacity)
            .map(|_| UnsafeCell::new(0.0_f64))
            .collect::<Vec<_>>()
            .into_boxed_slice();
        Self {
            data,
            capacity,
            read_pos: AtomicUsize::new(0),
            write_pos: AtomicUsize::new(0),
        }
    }

    /// Returns the number of slots currently available for writing.
    #[inline]
    pub fn available_to_write(&self) -> usize {
        let read = self.read_pos.load(Ordering::Acquire);
        let write = self.write_pos.load(Ordering::Relaxed);
        // One slot is kept empty to distinguish full FROM empty.
        (self.capacity - 1 - write.wrapping_sub(read)) & (self.capacity - 1)
    }

    /// Returns the number of samples currently available for reading.
    #[inline]
    pub fn available_to_read(&self) -> usize {
        let write = self.write_pos.load(Ordering::Acquire);
        let read = self.read_pos.load(Ordering::Relaxed);
        write.wrapping_sub(read) & (self.capacity - 1)
    }

    /// Writes `samples` INTO the buffer. Returns `true` on success, `false` if the buffer
    /// does not have enough space (backpressure  -  caller should retry later).
    ///
    /// Must only be called FROM the single producer thread.
    pub fn push_frame(&self, samples: &[f64]) -> bool {
        let n = samples.len();
        if n == 0 {
            return true;
        }

        let write = self.write_pos.load(Ordering::Relaxed);
        let read = self.read_pos.load(Ordering::Acquire);
        let used = write.wrapping_sub(read) & (self.capacity - 1);
        if used + n >= self.capacity {
            return false; // backpressure
        }

        let mask = self.capacity - 1;
        for (i, &sample) in samples.iter().enumerate() {
            let idx = (write + i) & mask;
            // SAFETY: We are the sole writer. The slot at `idx` is not being read because
            // `used + n < capacity` ensures no overlap with the consumer window.
            unsafe { *self.data[idx].get() = sample };
        }

        // Release: make written data visible to the consumer before advancing write_pos.
        self.write_pos
            .store(write.wrapping_add(n) & usize::MAX, Ordering::Release);
        true
    }

    /// Reads the next frame of `out.len()` samples INTO `out`. Returns `true` on success,
    /// `false` if the buffer does not have enough data.
    ///
    /// Must only be called FROM the single consumer thread.
    pub fn pop_frame(&self, out: &mut [f64]) -> bool {
        let n = out.len();
        if n == 0 {
            return true;
        }

        let read = self.read_pos.load(Ordering::Relaxed);
        let write = self.write_pos.load(Ordering::Acquire);
        let available = write.wrapping_sub(read) & (self.capacity - 1);
        if available < n {
            return false;
        }

        let mask = self.capacity - 1;
        for (i, slot) in out.iter_mut().enumerate() {
            let idx = (read + i) & mask;
            // SAFETY: We are the sole reader. The write_pos Acquire load above synchronises
            // with the producer's Release store, guaranteeing the data is fully written.
            *slot = unsafe { *self.data[idx].get() };
        }

        // Release: make read_pos advance visible to the producer.
        self.read_pos
            .store(read.wrapping_add(n) & usize::MAX, Ordering::Release);
        true
    }

    /// Resets both positions to zero, discarding all buffered data.
    /// Call only when no other thread is concurrently accessing the buffer.
    pub fn clear(&self) {
        self.read_pos.store(0, Ordering::SeqCst);
        self.write_pos.store(0, Ordering::SeqCst);
    }

    /// Buffer capacity in samples (always a power of two).
    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_capacity_rounds_to_power_of_two() {
        let rb = RingBuffer::new(100);
        assert!(rb.capacity().is_power_of_two());
        assert!(rb.capacity() >= 100);
    }

    #[test]
    fn empty_buffer_reports_zero_available_to_read() {
        let rb = RingBuffer::new(64);
        assert_eq!(rb.available_to_read(), 0);
    }

    #[test]
    fn push_and_pop_single_frame() {
        let rb = RingBuffer::new(16);
        let input = [1.0_f64, 2.0, 3.0, 4.0];
        assert!(rb.push_frame(&input));
        assert_eq!(rb.available_to_read(), 4);

        let mut output = [0.0_f64; 4];
        assert!(rb.pop_frame(&mut output));
        assert_eq!(output, input);
        assert_eq!(rb.available_to_read(), 0);
    }

    #[test]
    fn push_returns_false_when_full() {
        let rb = RingBuffer::new(4); // capacity = 4, usable = 3
        let frame = [0.5_f64; 3];
        assert!(rb.push_frame(&frame));
        assert!(!rb.push_frame(&[0.5])); // no space
    }

    #[test]
    fn pop_returns_false_when_empty() {
        let rb = RingBuffer::new(16);
        let mut out = [0.0_f64; 4];
        assert!(!rb.pop_frame(&mut out));
    }

    #[test]
    fn multiple_push_pop_cycles() {
        let rb = RingBuffer::new(32);
        for i in 0..10_u32 {
            let frame = [f64::from(i), f64::from(i) + 0.5];
            assert!(rb.push_frame(&frame));
            let mut out = [0.0_f64; 2];
            assert!(rb.pop_frame(&mut out));
            assert_eq!(out.first().copied().unwrap_or_default(), f64::from(i));
            assert_eq!(out.get(1).copied().unwrap_or_default(), f64::from(i) + 0.5);
        }
    }

    #[test]
    fn clear_resets_positions() {
        let rb = RingBuffer::new(16);
        assert!(rb.push_frame(&[1.0, 2.0, 3.0]));
        assert_eq!(rb.available_to_read(), 3);
        rb.clear();
        assert_eq!(rb.available_to_read(), 0);
    }

    #[test]
    fn empty_slice_push_and_pop_are_no_ops() {
        let rb = RingBuffer::new(16);
        assert!(rb.push_frame(&[]));
        let mut out: [f64; 0] = [];
        assert!(rb.pop_frame(&mut out));
        assert_eq!(rb.available_to_read(), 0);
    }
}
