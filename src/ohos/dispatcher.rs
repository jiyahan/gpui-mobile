use std::{
    sync::mpsc::Sender,
    thread::{self, ThreadId},
    time::Duration,
};

use gpui::{PlatformDispatcher, Priority, RunnableVariant};

use super::Command;

pub(super) struct OhosDispatcher {
    main_thread: ThreadId,
    sender: Sender<Command>,
}

impl OhosDispatcher {
    pub(super) fn new(sender: Sender<Command>) -> Self {
        Self {
            main_thread: thread::current().id(),
            sender,
        }
    }
}

impl PlatformDispatcher for OhosDispatcher {
    fn is_main_thread(&self) -> bool {
        thread::current().id() == self.main_thread
    }

    fn dispatch(&self, runnable: RunnableVariant, _priority: Priority) {
        thread::spawn(move || runnable.run());
    }

    fn dispatch_on_main_thread(&self, runnable: RunnableVariant, _priority: Priority) {
        let _ = self.sender.send(Command::Task(runnable));
    }

    fn dispatch_after(&self, duration: Duration, runnable: RunnableVariant) {
        let sender = self.sender.clone();
        thread::spawn(move || {
            thread::sleep(duration);
            let _ = sender.send(Command::Task(runnable));
        });
    }

    fn spawn_realtime(&self, f: Box<dyn FnOnce() + Send>) {
        thread::spawn(f);
    }
}
