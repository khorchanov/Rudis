use std::{
    io::{BufRead, BufReader, BufWriter, Read, Write},
    net::TcpListener,
};

use anyhow::Result;

use crate::{
    command::{Clock, Command, Executor, SystemClock},
    store::{HashMapStore, Store},
};

mod command;
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
    let mut buffer = String::new();
    let read_ops = reader.read_line(&mut buffer);
    match read_ops {
        Ok(0) => {}
        Err(e) => return Err(e.into()),
        Ok(_) => {
            let command = Command::try_parse(buffer)?;
            let processing_result = executor.process(command)?;
            let mut writer = BufWriter::new(stream_writer);
            writer.write_all(
                format!("OK {}\n", processing_result.unwrap_or_default())
                    .into_bytes()
                    .as_slice(),
            )?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod test {

    use std::{
        io::{Cursor, Result as IoResult, Write},
        sync::{Arc, Mutex},
    };

    use crate::{
        command::{Clock, Executor, SystemClock}, handle_request, store::{HashMapStore, Store},
    };

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

    fn send_and_get<S: Store, C: Clock>(request: &str, store: &mut Executor<S, C>) -> String {
        let input = Cursor::new(request.as_bytes());
        let reader = std::io::BufReader::new(input);
        let writer: TestBuffer = TestBuffer::default();
        let writer_handle = writer.clone();
        handle_request(reader, writer, store).unwrap();
        let output = writer_handle.0.lock().unwrap();
        String::from_utf8(Vec::from(&output[..])).unwrap()
    }

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
    pub fn should_expire() {
        let mut executor = Executor {
            store: HashMapStore::new(),
            clock: SystemClock {},
        };

        let response = send_and_get("PUT name Moez\n", &mut executor);
        assert_eq!(response, "OK \n");

        let response = send_and_get("GET name\n", &mut executor);
        assert_eq!(response, "OK Moez\n");
    }
}
