use std::thread;
use std::sync::{mpsc, Arc, Mutex};


/// Implements the thread pool concept.
///
/// It contains a group of pre-instantiated, idle threads
/// which stand ready to be given work
pub struct ThreadPool {
    /// It must contain a list of workers
    workers: Vec<Worker>,
    /// And a Sender connecting to each of the threads
    sender: mpsc::Sender<Message>,
}

/// Type alias of a smart function pointer with 
/// Send trait and static lifetime
type Job = Box<dyn FnOnce() + Send + 'static>;

/// Contains either a Job or a Terminate message
enum Message {
    NewJob(Job),
    Terminate,
}

impl ThreadPool {
    /// Create a new ThreadPool.
    ///
    /// # Arguments
    ///
    /// * `size` - is the number of threads in the pool.
    ///
    /// # Example
    ///
    /// ```
    /// use multithreaded_webserver::ThreadPool;
    ///
    /// let pool = ThreadPool::new(5);
    /// ```
    ///
    /// # Panics
    ///
    /// The `new` function will panic if the size is zero.
    pub fn new(size: usize) -> ThreadPool {
        assert!(size > 0);

        let (sender, receiver) = mpsc::channel();

        let receiver = Arc::new(Mutex::new(receiver));

        let mut workers = Vec::with_capacity(size);

        for id in 0..size {
            // create some threads and store them in the vector
            workers.push(Worker::new(id, Arc::clone(&receiver)));
        }

        ThreadPool {
            workers,
            sender,
        }
    }

    /// Spawns a thread by sendin the worker a message with a job
    ///
    /// # Arguments
    ///
    /// * `f` -> Function of generic type
    ///
    /// # Example
    ///
    /// ```
    /// use multithreaded_webserver::ThreadPool;
    ///
    /// let pool = ThreadPool::new(5);
    ///
    /// pool.execute(|| println!("Spawned thread executing this closure!"));
    /// ```
    pub fn execute<F>(&self, f: F)
        where
            F: FnOnce() + Send + 'static
    {
        let job = Box::new(f);
        
        self.sender.send(Message::NewJob(job)).unwrap();
    }
}

impl Drop for ThreadPool {
    /// Implement Drop Trait for ThreadPool
    ///
    /// It avoid all threads to instantly close as soon as drop is called
    /// instead, while a process is being run, that thread waits
    fn drop(&mut self) {
        println!("Sending terminate message to all workers.");

        for _ in &mut self.workers {
            self.sender.send(Message::Terminate).unwrap();
        }

        println!("Shutting down all workers.");

        for worker in &mut self.workers {
            println!("Shutting down worker {}", worker.id);               

            if let Some(thread) = worker.thread.take() {
                thread.join().unwrap();
            }
        }
    }
}

/// Connects the thread with others with the join handle
struct Worker {
    /// Also contains an id to be shown to user 
    id: usize,
    thread: Option<thread::JoinHandle<()>>,
}

impl Worker {
    /// Creates new instance of Worker
    ///
    /// # Arguments
    ///
    /// * `id` - Number from 0 to n given to worker
    /// * `receiver` - Its an atomic reference counter containing a mutex
    /// which contains the receiving end of a mpsc in ThreadPool and receives
    /// a Message enum contianing either a Terminate value or function call
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Message>>>) -> Worker {
        let thread = thread::spawn(move || {
            loop {
                let message = receiver.lock().unwrap().recv().unwrap();

                match message {
                    Message::NewJob(job) => {
                        println!("Worker {} got a job; executing.", id);

                        job();
                    },
                    Message::Terminate => {
                        println!("Worker {} was told to terminate.", id);

                        break;
                    }
                }
            }
        });

        Worker {
            id,
            thread: Some(thread),
        }
    }
}
