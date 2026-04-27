use std::collections::VecDeque;

use crate::{AicoreError, AicoreResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundedQueue<T> {
    capacity: usize,
    items: VecDeque<T>,
}

impl<T> BoundedQueue<T> {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            items: VecDeque::new(),
        }
    }

    pub fn push(&mut self, item: T) -> AicoreResult<()> {
        if self.items.len() >= self.capacity {
            return Err(AicoreError::QueueFull(format!(
                "capacity {} reached",
                self.capacity
            )));
        }

        self.items.push_back(item);
        Ok(())
    }

    pub fn pop(&mut self) -> Option<T> {
        self.items.pop_front()
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use crate::AicoreError;

    use super::BoundedQueue;

    #[test]
    fn bounded_queue_rejects_over_capacity() {
        let mut queue = BoundedQueue::new(1);
        queue.push("first").expect("first item should fit");

        let error = queue.push("second").expect_err("queue should be full");

        assert_eq!(
            error,
            AicoreError::QueueFull("capacity 1 reached".to_string())
        );
    }

    #[test]
    fn bounded_queue_pops_fifo() {
        let mut queue = BoundedQueue::new(2);
        queue.push("first").expect("first item should fit");
        queue.push("second").expect("second item should fit");

        assert_eq!(queue.len(), 2);
        assert_eq!(queue.capacity(), 2);
        assert_eq!(queue.pop(), Some("first"));
        assert_eq!(queue.pop(), Some("second"));
        assert_eq!(queue.pop(), None);
    }
}
