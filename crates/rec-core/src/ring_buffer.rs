//! Bounded ring buffer for low-latency frame queueing.
//!
//! Conforms to `[SPEC-CORE-001]` and `[CONTRACT-CORE-001]`.

use std::collections::VecDeque;
use parking_lot::Mutex;
use crate::errors::CoreError;

/// A thread-safe bounded FIFO ring buffer.
///
/// Designed for buffering video and audio frames between acquisition and encoding
/// without unbounded heap allocations.
#[derive(Debug)]
pub struct LockFreeRingBuffer<T> {
    inner: Mutex<VecDeque<T>>,
    capacity: usize,
}

impl<T> LockFreeRingBuffer<T> {
    /// Creates a new `LockFreeRingBuffer` with a fixed maximum capacity.
    ///
    /// # Arguments
    ///
    /// * `capacity` - Maximum number of items the buffer can hold.
    ///
    /// # Returns
    ///
    /// An empty ring buffer with preallocated capacity.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rec_core::ring_buffer::LockFreeRingBuffer;
    ///
    /// let buffer: LockFreeRingBuffer<u32> = LockFreeRingBuffer::new(60);
    /// assert_eq!(buffer.capacity(), 60);
    /// assert!(buffer.is_empty());
    /// ```
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        Self {
            inner: Mutex::new(VecDeque::with_capacity(capacity)),
            capacity,
        }
    }

    /// Pushes an item into the buffer.
    ///
    /// # Arguments
    ///
    /// * `item` - The item to push.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::BufferOverflow`] if the buffer is currently full.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rec_core::ring_buffer::LockFreeRingBuffer;
    ///
    /// let buffer = LockFreeRingBuffer::new(2);
    /// assert!(buffer.push(10).is_ok());
    /// assert!(buffer.push(20).is_ok());
    /// assert!(buffer.push(30).is_err());
    /// ```
    pub fn push(&self, item: T) -> Result<(), CoreError> {
        let mut queue = self.inner.lock();
        if queue.len() >= self.capacity {
            return Err(CoreError::BufferOverflow {
                capacity: self.capacity,
            });
        }
        queue.push_back(item);
        Ok(())
    }

    /// Pops the oldest item from the buffer.
    ///
    /// # Returns
    ///
    /// Returns the popped item on success.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::BufferEmpty`] if the buffer contains no elements.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rec_core::ring_buffer::LockFreeRingBuffer;
    ///
    /// let buffer = LockFreeRingBuffer::new(2);
    /// buffer.push(42).unwrap();
    /// assert_eq!(buffer.pop().unwrap(), 42);
    /// assert!(buffer.pop().is_err());
    /// ```
    pub fn pop(&self) -> Result<T, CoreError> {
        let mut queue = self.inner.lock();
        queue.pop_front().ok_or(CoreError::BufferEmpty)
    }

    /// Returns the maximum capacity of this buffer.
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Returns the current number of items stored in the buffer.
    #[must_use]
    pub fn len(&self) -> usize {
        self.inner.lock().len()
    }

    /// Checks if the buffer is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.inner.lock().is_empty()
    }

    /// Checks if the buffer has reached maximum capacity.
    #[must_use]
    pub fn is_full(&self) -> bool {
        self.inner.lock().len() >= self.capacity
    }

    /// Clears all elements from the buffer.
    pub fn clear(&self) {
        self.inner.lock().clear();
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_ring_buffer_fifo_order() {
        let buffer = LockFreeRingBuffer::new(3);
        buffer.push("A").unwrap();
        buffer.push("B").unwrap();
        assert_eq!(buffer.pop().unwrap(), "A");
        buffer.push("C").unwrap();
        assert_eq!(buffer.pop().unwrap(), "B");
        assert_eq!(buffer.pop().unwrap(), "C");
        assert!(buffer.pop().is_err());
    }

    #[test]
    fn test_ring_buffer_overflow() {
        let buffer = LockFreeRingBuffer::new(1);
        assert!(buffer.push(1).is_ok());
        assert!(matches!(buffer.push(2), Err(CoreError::BufferOverflow { capacity: 1 })));
    }
}
