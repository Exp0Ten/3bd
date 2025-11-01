use std::sync::{Mutex, MutexGuard};
use std::path::Path;


//Internal Data (for debbuging)
#[derive(Debug, Clone)]
pub struct Internal {
    file: Option<Box<Path>>,
}

// Public Handle

pub static INTERNAL: Mutex<Internal> = Mutex::new(Internal::new());

impl Internal {
    const fn new() -> Self {
        Internal {
            file: None
        }
    }
}

pub trait Glob<'a> {
    fn access(&'a self) -> MutexGuard<'a, Internal>;
    fn get(&'a self) -> Internal;
    fn set(&'a self, internal: Internal);
    //  add more as needed
}

impl<'a> Glob<'a> for Mutex<Internal> {
    fn access(&'a self) -> MutexGuard<'a, Internal> {
        self.lock().unwrap()
    }

    fn get(&'a self) -> Internal {
        self.access().clone()
    }

    fn set(&'a self, internal: Internal) {
        *self.access() = internal;
    }

}