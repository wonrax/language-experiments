use std::{array, collections::LinkedList};

const LEVEL_MULT: usize = 64;

/// Number of levels. Each level has 64 slots. By using 6 levels with 64 slots
/// each, the timer is able to track time up to 2 years into the future with a
/// precision of 1 millisecond.
const NUM_LEVELS: usize = 6;

pub struct TimerWheel {
    pub current: u64,

    // 1ms - 64ms
    // 64ms - 4.1s
    // 4.1s - 4.4m
    // 4.4m - 4.7h
    // 4.7h - 12.5d
    // 12.5d - 2.2y
    levels: [Level; NUM_LEVELS],
}

impl TimerWheel {
    pub fn new() -> TimerWheel {
        TimerWheel {
            current: 0,
            levels: array::from_fn(Level::new),
        }
    }

    pub fn add(&mut self, entry: TimerEntry) {
        let level = level_for(entry.expiration, self.current);
        self.levels[level].add(entry);
    }

    pub fn next_expiration(&self) -> Option<u64> {
        for level in &self.levels {
            if let Some(expiration) = level.next_expiration() {
                return Some(expiration);
            }
        }

        None
    }
}

impl Default for TimerWheel {
    fn default() -> Self {
        Self::new()
    }
}

struct Level {
    /// The current level.
    level: usize,

    slots: [LinkedList<TimerEntry>; LEVEL_MULT],
}

impl Level {
    pub fn new(level: usize) -> Level {
        Level {
            level,
            slots: array::from_fn(|_| LinkedList::new()),
        }
    }

    pub fn add(&mut self, entry: TimerEntry) {
        let slot = slot_for(entry.expiration, 0);
        self.slots[slot].push_back(entry);
    }

    pub fn remove(&mut self, entry: TimerEntry) {
        let slot = slot_for(entry.expiration, 0);

        // TODO: This is inefficient because we have to iterate through the list twice, once to
        // find the entry and once to remove it. We should be able to remove it in one pass.
        for (i, e) in self.slots[slot].iter().enumerate() {
            if e.expiration == entry.expiration {
                self.slots[slot].remove(i);
                break;
            }
        }
    }

    pub fn next_expiration(&self) -> Option<u64> {
        for slot in &self.slots {
            if let Some(entry) = slot.front() {
                return Some(entry.expiration);
            }
        }

        None
    }
}

/// NOTE: Yanked from the Tokio codebase.
/// TODO: still haven't fully figured out how this works.
fn level_for(elapsed: u64, when: u64) -> usize {
    const SLOT_MASK: u64 = (1 << 6) - 1;
    /// The maximum duration of a `Sleep`.
    const MAX_DURATION: u64 = (1 << (6 * NUM_LEVELS)) - 1;

    // Mask in the trailing bits ignored by the level calculation in order to cap
    // the possible leading zeros
    let mut masked = elapsed ^ when | SLOT_MASK;

    if masked >= MAX_DURATION {
        // Fudge the timer into the top level
        masked = MAX_DURATION - 1;
    }

    let leading_zeros = masked.leading_zeros() as usize;
    let significant = 63 - leading_zeros;

    significant / NUM_LEVELS
}

/// Converts a duration (milliseconds) and a level to a slot position.
/// NOTE: Yanked from the Tokio codebase.
/// TODO: still haven't fully figured out how this works.
fn slot_for(duration: u64, level: usize) -> usize {
    ((duration >> (level * 6)) % LEVEL_MULT as u64) as usize
}

pub struct TimerEntry {
    /// In milliseconds.
    pub expiration: u64,
}
