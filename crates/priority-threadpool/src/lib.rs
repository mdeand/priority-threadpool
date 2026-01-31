use std::{
    marker::PhantomData,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    thread::JoinHandle,
};

use async_task::Runnable;
use lockfree::stack::Stack;

//mod stack;
mod util;
//mod dropper;

pub trait Priority {
    const COUNT: usize;

    fn index(&self) -> usize;
}

#[derive(Clone)]
struct PriorityQueue<P: Priority> {
    stacks: Arc<Vec<Stack<Runnable>>>,
    _phantom: PhantomData<P>,
}

impl<P: Priority> PriorityQueue<P> {
    fn pop(&self) -> Option<Runnable> {
        for ix in 0..P::COUNT {
            let stack = &self.stacks[ix];

            if let Some(value) = stack.pop() {
                return Some(value);
            }
        }

        None
    }

    fn push(&self, priority: &P, runnable: Runnable) {
        let index = priority.index();

        assert!(index < P::COUNT);
        assert!(index < self.stacks.len());

        self.stacks[priority.index()].push(runnable);
    }

    pub fn new() -> Self {
        Self {
            stacks: Arc::new((0..P::COUNT).into_iter().map(|_| Stack::new()).collect()),
            _phantom: PhantomData,
        }
    }
}

pub struct ThreadPool<P: Priority + Clone> {
    jobs_queued: Arc<AtomicUsize>,
    should_stop: Arc<AtomicBool>,
    waiting: Vec<Arc<AtomicBool>>,
    threads: Vec<JoinHandle<()>>,
    queue: PriorityQueue<P>,
}

impl<P: Priority + Clone + Send + 'static> ThreadPool<P> {
    pub fn new(nworkers: usize) -> Self {
        let jobs_queued = Arc::new(AtomicUsize::new(0));

        let should_stop = Arc::new(AtomicBool::new(false));

        let waiting: Vec<_> = (0..nworkers)
            .into_iter()
            .map(|_| Arc::new(AtomicBool::new(false)))
            .collect();

        let queue = PriorityQueue::new();

        let threads: Vec<_> = (0..nworkers)
            .into_iter()
            .map(|ix| {
                let jobs_queued = jobs_queued.clone();
                let should_stop = should_stop.clone();
                let thread_waiting = waiting[ix].clone();
                let queue = queue.clone();

                std::thread::Builder::new()
                    .name(format!("ThreadPool worker {}", ix))
                    .spawn(move || {
                        thread_waiting.store(true, Ordering::Release);

                        while !should_stop.load(Ordering::Relaxed) {
                            // TODO(mdeand): This probably doesn't need to happen here
                            match util::atomic_saturating_sub(&jobs_queued, 1) {
                                (old, new) if old > 0 => {
                                    if let Some(runnable) = queue.pop() {
                                        thread_waiting.store(false, Ordering::Release);

                                        /*
                                        println!(
                                            "Thread {:?} running job...",
                                            std::thread::current().id()
                                        );
                                        */

                                        runnable.run();
                                    }
                                }
                                // If there's no jobs in queue, do nothing.
                                _ => std::thread::park(),
                            };

                            thread_waiting.store(true, Ordering::Release);
                        }
                    })
                    .unwrap()
            })
            .collect();

        Self {
            jobs_queued,
            should_stop,
            waiting,
            threads,
            queue,
        }
    }

    pub fn block_til_ready(&self) {
        // Wait for all threads to park
        for is_waiting in &self.waiting {
            while !is_waiting.load(Ordering::Acquire) {
                std::hint::spin_loop();
            }
        }
    }

    pub fn signal_stop(&self) {
        self.should_stop.store(false, Ordering::Release);
    }

    pub fn wake(&self) {
        for ix in 0..self.threads.len() {
            let handle = &self.threads[ix];
            let waiting = &self.waiting[ix];

            if waiting.load(Ordering::Acquire) {
                handle.thread().unpark();
            }
        }
    }

    pub fn queue(&self, priority: &P, job: Runnable) {
        self.jobs_queued.fetch_add(1, Ordering::SeqCst);
        self.queue.push(priority, job);

        // TODO(mdeand): Unparking when the thread is unparked will cause
        // TODO(mdeand): the next invoation of park() on that given thread
        // TODO(mdeand): to not block. This isn't a terrible issue currently
        // TODO(mdeand): however this implementation could be more efficient.

        //self.wake();
    }
}
