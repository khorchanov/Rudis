use std::{
    io::{BufRead, BufReader, BufWriter, Read, Write},
    net::TcpListener,
};

use anyhow::Result;

use crate::{
    command::Command,
    executor::{Clock, Executor, SystemClock},
    store::{HashMapStore, Store},
};

mod command;
mod executor;
mod store;

fn main() -> Result<()> {
    let listener = TcpListener::bind("0.0.0.0:8080")?;
    let mut executor = Executor {
        store: HashMapStore::new(),
        clock: SystemClock {},
    };

    println!("Server listening on port 8080");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => handle_request(stream.try_clone()?, stream, &mut executor)?, //TODO: this is ugly
            Err(err) => eprintln!("Connection failed {err}"),
        }
    }

    Ok(())
}

fn handle_request<S: Store, C: Clock>(
    stream_reader: impl Read,
    stream_writer: impl Write,
    executor: &mut Executor<S, C>,
) -> Result<()> {
    let mut reader = BufReader::new(stream_reader);
    let mut writer = BufWriter::new(stream_writer);
    let mut line = String::new();
    loop {
        line.clear();
        let read_ops = reader.read_line(&mut line); //TODO: beware of blocking ops
        match read_ops {
            Ok(0) => {
                println!("Client closed the connection");
                break;
            }
            Err(e) => return Err(e.into()),
            Ok(_) => {
                if line.trim().is_empty() {
                    println!("Empty line received");
                    break;
                }
                let command = Command::try_parse(&line)?;
                let processing_result = executor.process(command)?;
                writer.write_all(
                    format!("OK {}\n", processing_result.unwrap_or_default())
                        .into_bytes()
                        .as_slice(),
                )?;
                writer.flush()?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod test {

    use std::{
        cell::RefCell,
        collections::HashMap,
        io::{Cursor, Result as IoResult, Write},
        rc::Rc,
        sync::{Arc, Mutex},
        time::{Duration, Instant},
    };

    use crate::{
        executor::{Clock, Executor, SystemClock},
        handle_request,
        store::{Entry, HashMapStore, Store},
    };

    // Test utilities
    #[derive(Clone, Default)]
    struct TestBuffer(Arc<Mutex<Vec<u8>>>);

    impl Write for TestBuffer {
        fn write(&mut self, buf: &[u8]) -> IoResult<usize> {
            self.0.lock().unwrap().write(buf)
        }
        fn flush(&mut self) -> IoResult<()> {
            self.0.lock().unwrap().flush()
        }
    }

    #[derive(Clone)]
    pub struct TestClock {
        simulated_time: Rc<RefCell<Instant>>,
    }

    impl Default for TestClock {
        fn default() -> Self {
            Self {
                simulated_time: Rc::new(RefCell::new(Instant::now())),
            }
        }
    }

    impl TestClock {
        fn advance_time_with(&self, duration: Duration) {
            let old_value = self.simulated_time.borrow().clone();
            *self.simulated_time.borrow_mut() = old_value + duration;
        }
    }

    impl Clock for TestClock {
        fn now(&self) -> Instant {
            *self.simulated_time.borrow()
        }
    }

    fn send_and_get<S: Store, C: Clock>(request: &str, store: &mut Executor<S, C>) -> String {
        let input = Cursor::new(request.as_bytes());
        let reader = std::io::BufReader::new(input);
        let writer: TestBuffer = TestBuffer::default();
        let writer_handle = writer.clone();
        handle_request(reader, writer, store).unwrap();
        let output = writer_handle.0.lock().unwrap();
        String::from_utf8(Vec::from(&output[..])).unwrap()
    }

    #[derive(Clone)]
    struct TestStore(Rc<RefCell<HashMap<String, Entry>>>);

    impl Store for TestStore {
        fn put(&mut self, key: String, value: crate::store::Entry) -> anyhow::Result<()> {
            self.0.borrow_mut().insert(key, value);
            Ok(())
        }

        fn get(&self, key: &str) -> anyhow::Result<Option<crate::store::Entry>> {
            Ok(self.0.borrow().get(key).cloned())
        }

        fn delete(&mut self, key: &str) -> anyhow::Result<Option<crate::store::Entry>> {
            Ok(self.0.borrow_mut().remove(key))
        }
    }

    impl Default for TestStore {
        fn default() -> Self {
            Self(Rc::new(RefCell::new(HashMap::new())))
        }
    }

    // End Test utilities

    #[test]
    fn test_ping() {
        let mut executor = Executor {
            store: HashMapStore::new(),
            clock: SystemClock {},
        };
        let response = send_and_get("PING\n", &mut executor);
        assert_eq!(response, "OK PONG\n");
    }

    #[test]
    pub fn should_store_and_read() {
        let mut executor = Executor {
            store: HashMapStore::new(),
            clock: SystemClock {},
        };

        let response = send_and_get("PUT name Moez\n", &mut executor);
        assert_eq!(response, "OK \n");

        let response = send_and_get("GET name\n", &mut executor);
        assert_eq!(response, "OK Moez\n");
    }

    #[test]
    pub fn should_store_and_delete() {
        let mut executor = Executor {
            store: HashMapStore::new(),
            clock: SystemClock {},
        };

        let response = send_and_get("PUT name Moez\n", &mut executor);
        assert_eq!(response, "OK \n");

        let response = send_and_get("GET name\n", &mut executor);
        assert_eq!(response, "OK Moez\n");

        let response = send_and_get("DEL name\n", &mut executor);
        assert_eq!(response, "OK Moez\n");

        let response = send_and_get("GET name\n", &mut executor);
        assert_eq!(response, "OK \n");
    }

    #[test]
    pub fn expired_entries_should_not_be_returned() {
        let clock = TestClock::default();
        let store = TestStore::default();

        let mut executor = Executor {
            store: store.clone(),
            clock: clock.clone(),
        };

        let response = send_and_get("PUT name Moez\n", &mut executor);
        assert_eq!(response, "OK \n");

        clock.advance_time_with(Duration::from_mins(10));

        let response = send_and_get("GET name\n", &mut executor);
        assert_eq!(response, "OK \n");

        assert!(matches!(store.get("name"), Ok(None)));
    }
}
