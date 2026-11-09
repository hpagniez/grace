//! `src/main/batch.ts`: items queued as they come and taken in batches, so whoever shows them applies a whole poll of
//! run events in one go instead of one per event (#123).
//!
//! Electron Grace's `batched(send)` queued each item and, on the first of a tick, scheduled `setImmediate` to send the
//! queue as one message. A thread has no event-loop tick to schedule on, so here the queue is a channel and the
//! receiver sets the tick: each `next_batch` takes everything that has been queued since the last one, in order.
use std::sync::mpsc::{self, Receiver, Sender};

// Rust-native: a channel the receiver drains in batches, in place of a closure and `setImmediate`. Parity: the
// TypeScript's two tests, `sends_the_items_of_one_tick_as_one_message_in_order` and
// `starts_a_new_message_on_the_next_tick`, with the receiver's call as the tick.

/// The queueing end: any number of threads may hold a clone.
pub struct Batcher<T>(Sender<T>);

impl<T> Clone for Batcher<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> Batcher<T> {
    /// Queues an item. `Err` gives it back when the `Batches` is gone, so nothing is queued for nobody.
    pub fn push(&self, item: T) -> Result<(), T> {
        self.0.send(item).map_err(|e| e.0)
    }
}

/// The taking end: batches of what was queued, in order.
pub struct Batches<T>(Receiver<T>);

impl<T> Batches<T> {
    /// Waits for an item, then returns it with every item queued behind it: one message per tick. `None` once every
    /// `Batcher` is gone and the queue is empty.
    pub fn next_batch(&self) -> Option<Vec<T>> {
        let first = self.0.recv().ok()?;
        Some(std::iter::once(first).chain(self.0.try_iter()).collect())
    }

    /// The items queued now as one batch, without waiting; `None` if there are none.
    pub fn try_next_batch(&self) -> Option<Vec<T>> {
        let batch: Vec<T> = self.0.try_iter().collect();
        (!batch.is_empty()).then_some(batch)
    }
}

/// A queue and its two ends.
pub fn batched<T>() -> (Batcher<T>, Batches<T>) {
    let (tx, rx) = mpsc::channel();
    (Batcher(tx), Batches(rx))
}

#[cfg(test)]
mod tests {
    use super::*;

    mod batched {
        use super::*;

        #[test]
        fn sends_the_items_of_one_tick_as_one_message_in_order() {
            let (emit, batches) = batched::<u32>();
            for i in 1..=500 {
                emit.push(i).unwrap();
            }
            assert_eq!(batches.next_batch(), Some((1..=500).collect()));
            assert_eq!(batches.try_next_batch(), None);
        }

        #[test]
        fn starts_a_new_message_on_the_next_tick() {
            let (emit, batches) = batched::<&str>();
            emit.push("a").unwrap();
            emit.push("b").unwrap();
            assert_eq!(batches.next_batch(), Some(vec!["a", "b"]));
            emit.push("c").unwrap();
            assert_eq!(batches.next_batch(), Some(vec!["c"]));
            assert_eq!(batches.try_next_batch(), None);
        }

        // New in the port: the queue is shared by threads and ends with its senders.
        #[test]
        fn items_from_other_threads_arrive_in_order_and_the_queue_ends_with_its_senders() {
            let (emit, batches) = batched::<u32>();
            let sender = std::thread::spawn(move || {
                for i in 0..1000 {
                    emit.push(i).unwrap();
                }
            });
            let mut all = Vec::new();
            while let Some(batch) = batches.next_batch() {
                assert!(!batch.is_empty());
                all.extend(batch);
            }
            sender.join().unwrap();
            assert_eq!(all, (0..1000).collect::<Vec<_>>());
        }

        #[test]
        fn a_push_after_the_batches_is_gone_gives_the_item_back() {
            let (emit, batches) = batched::<&str>();
            drop(batches);
            assert_eq!(emit.push("a"), Err("a"));
        }
    }
}
