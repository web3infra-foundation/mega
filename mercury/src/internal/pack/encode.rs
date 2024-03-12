//!
//!
//!
//!

use std::sync::mpsc::Receiver;

use venus::{errors::GitError, internal::pack::entry::Entry};

use crate::internal::pack::Pack;

impl Pack {
    pub fn encode(&self, receiver: Receiver<Entry>) -> Result<(), GitError> {
        for entry in receiver {
            println!("receive entry: {}", entry.hash);
            self.pool.execute(move || {
                // do something complex work
            })
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::{path::PathBuf, str::FromStr, sync::mpsc, thread, time::Duration};

    use venus::{
        hash::SHA1,
        internal::{object::types::ObjectType, pack::entry::Entry},
    };

    use crate::internal::pack::Pack;

    #[test]
    fn test_encode() {
        let (sender, receiver) = mpsc::channel();
        let tmp = PathBuf::from("/tmp/.cache_temp");
        let p = Pack::new(None, Some(1024 * 1024 * 1024 * 4), Some(tmp.clone()));
        let handler = thread::spawn(move || {
            p.encode(receiver).unwrap();
        });
        thread::spawn(move || {
            for _ in 0..10 {
                let entry = Entry {
                    obj_type: ObjectType::Blob,
                    data: Vec::new(),
                    hash: SHA1::from_str("8ab686eafeb1f44702738c8b0f24f2567c36da6d").unwrap(),
                };
                sender.send(entry).unwrap();
                thread::sleep(Duration::from_millis(500))
            }
        });
        handler.join().unwrap();
    }
}
