use std::{
    collections::BinaryHeap,
    sync::{Arc, Mutex},
};

struct Prioritized<Priority, Msg> {
    priority: Priority,
    msg: Msg,
}

impl<Priority: std::cmp::PartialEq, Msg> std::cmp::PartialEq for Prioritized<Priority, Msg> {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority
    }
}

impl<Priority: std::cmp::Eq, Msg> std::cmp::Eq for Prioritized<Priority, Msg> {}

impl<Priority: std::cmp::Ord, Msg> std::cmp::PartialOrd for Prioritized<Priority, Msg> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.priority.cmp(&other.priority))
    }
}

impl<Priority: Ord, Msg> Ord for Prioritized<Priority, Msg> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.priority.cmp(&other.priority)
    }
}

pub struct PriorityThreadPool<Priority, Msg> {
    tx: std::sync::mpsc::Sender<Msg>,
    queue: Arc<Mutex<BinaryHeap<Prioritized<Priority, Msg>>>>,
}

impl<Priority, Msg> PriorityThreadPool<Priority, Msg> {
    pub fn new(workers: usize) -> Self {
        todo!()
    }
}
