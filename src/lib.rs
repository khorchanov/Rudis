use anyhow::Result;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader, BufWriter};

use crate::{
    command::Command,
    protocol::serialize,
    service::{Clock, Executor},
    store::Store,
};

pub mod command;
mod protocol;
pub mod service;
pub mod store;

pub async fn handle_connection<S: Store, C: Clock>(
    stream_reader: impl AsyncRead + Unpin,
    stream_writer: impl AsyncWrite + Unpin,
    service: &mut Executor<S, C>,
) -> Result<()> {
    println!("New Client Connected");
    let mut reader = BufReader::new(stream_reader);
    let mut writer = BufWriter::new(stream_writer);
    let mut line = String::new();
    loop {
        line.clear();
        let mut buf: Vec<u8> = Vec::with_capacity(4096);

        let read_ops = reader.read_buf(&mut buf).await;
        match read_ops {
            Ok(0) => {
                println!("Client closed the connection");
                break;
            }
            Err(e) => return Err(e.into()),
            Ok(_) => {
                let command = if let Ok(command) = Command::try_parse_resp(&buf) {
                    command
                } else {
                    write(&mut writer, format!("ERR {}\n", 400).into_bytes()).await?;
                    continue;
                };
                let processing_result = service.process_command_resp(command).await?;
                write(&mut writer, serialize(processing_result)).await?;
            }
        }
    }
    Ok(())
}

async fn write(
    writer: &mut BufWriter<impl AsyncWrite + Unpin>,
    response: Vec<u8>,
) -> Result<(), anyhow::Error> {
    writer.write_all(response.as_slice()).await?; //TODO: remove this as slice
    writer.flush().await?;
    Ok(())
}

#[cfg(test)]
mod test {

    use std::{
        cell::RefCell,
        collections::HashMap,
        io::{Cursor, Write},
        sync::{Arc, Mutex},
        time::{Duration, Instant},
    };

    use tokio::io::{AsyncWrite, BufReader};

    use crate::{
        handle_connection,
        service::{Clock, Executor, SystemClock},
        store::{Entry, HashMapStore, Store},
    };

    // Test utilities
    #[derive(Clone, Default)]
    struct TestBuffer(Arc<Mutex<Vec<u8>>>);

    impl AsyncWrite for TestBuffer {
        fn poll_write(
            self: std::pin::Pin<&mut Self>,
            _: &mut std::task::Context<'_>,
            buf: &[u8],
        ) -> std::task::Poll<std::io::Result<usize>> {
            let n = self.0.lock().unwrap().write(buf).unwrap();
            std::task::Poll::Ready(Ok(n))
        }

        fn poll_flush(
            self: std::pin::Pin<&mut Self>,
            _: &mut std::task::Context<'_>,
        ) -> std::task::Poll<std::io::Result<()>> {
            std::task::Poll::Ready(Ok(()))
        }

        fn poll_shutdown(
            self: std::pin::Pin<&mut Self>,
            _: &mut std::task::Context<'_>,
        ) -> std::task::Poll<std::io::Result<()>> {
            std::task::Poll::Ready(Ok(()))
        }
    }

    #[derive(Clone)]
    pub struct TestClock {
        simulated_time: Arc<RefCell<Instant>>,
    }

    impl Default for TestClock {
        fn default() -> Self {
            Self {
                simulated_time: Arc::new(RefCell::new(Instant::now())),
            }
        }
    }

    impl TestClock {
        fn advance_time_by(&self, duration: Duration) {
            let old_value = self.simulated_time.borrow().clone();
            *self.simulated_time.borrow_mut() = old_value + duration;
        }
    }

    impl Clock for TestClock {
        fn now(&self) -> Instant {
            *self.simulated_time.borrow()
        }
    }

    async fn send_and_get<S: Store, C: Clock>(
        request: &[u8],
        store: &mut Executor<S, C>,
    ) -> String {
        let input = Cursor::new(request);
        let reader = BufReader::new(input);
        let writer: TestBuffer = TestBuffer::default();
        let writer_handle = writer.clone();
        handle_connection(reader, writer, store).await.unwrap();
        let output = writer_handle.0.lock().unwrap();
        String::from_utf8(Vec::from(&output[..])).unwrap()
    }

    #[derive(Clone)]
    struct TestStore(Arc<RefCell<HashMap<String, Entry>>>);

    impl Store for TestStore {
        async fn put(&mut self, key: String, value: crate::store::Entry) -> anyhow::Result<()> {
            self.0.borrow_mut().insert(key, value);
            Ok(())
        }

        async fn get(&self, key: &str) -> anyhow::Result<Option<Entry>> {
            Ok(self.0.borrow().get(key).cloned())
        }

        async fn delete(&mut self, key: &str) -> anyhow::Result<Option<Entry>> {
            Ok(self.0.borrow_mut().remove(key))
        }
    }

    impl Default for TestStore {
        fn default() -> Self {
            Self(Arc::new(RefCell::new(HashMap::new())))
        }
    }

    // End Test utilities

    #[tokio::test]
    async fn test_ping() {
        let mut executor = Executor {
            store: HashMapStore::default(),
            clock: SystemClock {},
        };
        let response = send_and_get(b"*1\r\n$4\r\nPING\r\n", &mut executor).await;
        assert_eq!(response, "+PONG\r\n");
    }

    #[tokio::test]
    async fn should_store_and_read() {
        let mut executor = Executor {
            store: HashMapStore::default(),
            clock: SystemClock {},
        };

        let response = send_and_get(
            b"*3\r\n$3\r\nSET\r\n$4\r\nname\r\n$4\r\nMoez\r\n",
            &mut executor,
        )
        .await;
        assert_eq!(response, "+OK\r\n");

        let response = send_and_get(b"*2\r\n$3\r\nGET\r\n$4\r\nname\r\n", &mut executor).await;
        assert_eq!(response, "$4\r\nMoez\r\n");
    }

    #[tokio::test]
    async fn should_store_and_delete() {
        let mut executor = Executor {
            store: HashMapStore::default(),
            clock: SystemClock {},
        };

        let response = send_and_get(
            b"*3\r\n$3\r\nSET\r\n$4\r\nname\r\n$4\r\nMoez\r\n",
            &mut executor,
        )
        .await;
        assert_eq!(response, "+OK\r\n");

        let response = send_and_get(b"*2\r\n$3\r\nGET\r\n$4\r\nname\r\n", &mut executor).await;
        assert_eq!(response, "$4\r\nMoez\r\n");

        let response = send_and_get(b"*2\r\n$3\r\nDEL\r\n$4\r\nname\r\n", &mut executor).await;
        assert_eq!(response, ":1\r\n");

        let response = send_and_get(b"*2\r\n$3\r\nGET\r\n$4\r\nname\r\n", &mut executor).await;
        assert_eq!(response, "$-1\r\n");
    }

    #[tokio::test]
    async fn expired_entries_should_not_be_returned() {
        let clock = TestClock::default();
        let store = TestStore::default();

        let mut executor = Executor {
            store: store.clone(),
            clock: clock.clone(),
        };

        let response = send_and_get(
            b"*3\r\n$3\r\nSET\r\n$4\r\nname\r\n$4\r\nMoez\r\n",
            &mut executor,
        )
        .await;
        assert_eq!(response, "+OK\r\n");

        dbg!(clock.now());
        clock.advance_time_by(Duration::from_mins(10));
        dbg!(clock.now());

        let response = send_and_get(b"*2\r\n$3\r\nGET\r\n$4\r\nname\r\n", &mut executor).await;
        assert_eq!(response, "$-1\r\n");

        let result = dbg!(store.get("name").await);
        assert!(matches!(result, Ok(None)));
    }
}
