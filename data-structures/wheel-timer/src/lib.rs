use std::{array, collections::LinkedList};

pub struct TimerWheel {
    pub current: u64,

    // 1ms - 64ms
    // 64ms - 4.1s
    // 4.1s - 4.4m
    // 4.4m - 4.7h
    // 4.7h - 12.5d
    // 12.5d - 2.2y
    levels: [Level; 6],
}

type Level = [LinkedList<TimerEntry>; 64];

pub struct TimerEntry {
    pub expires: u64,
}

impl TimerWheel {
    pub fn new() -> TimerWheel {
        TimerWheel {
            current: 0,
            levels: array::from_fn(|_| array::from_fn(|_| LinkedList::new())),
        }
    }
}

impl Default for TimerWheel {
    fn default() -> Self {
        Self::new()
    }
}
