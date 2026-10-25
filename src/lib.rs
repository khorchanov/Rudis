use anyhow::Result;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader, BufWriter};

use crate::{command::Command, protocol::serialize, service::Clock, store::Store};

mod command;
mod protocol;
mod service;
mod store;
mod utils;

pub use service::{Executor, SystemClock};
pub use store::HashMapStore;

pub async fn handle_connection<S: Store, C: Clock>(
    stream_reader: impl AsyncRead + Unpin,
    stream_writer: impl AsyncWrite + Unpin,
    service: &mut Executor<S, C>,
) -> Result<()> {
    println!("New Client Connected");
    let mut reader = BufReader::new(stream_reader);
    let mut writer = BufWriter::new(stream_writer);
    let mut line = String::new();
    let mut buf: Vec<u8> = Vec::with_capacity(4096);
    loop {
        line.clear();
        match reader.read_buf(&mut buf).await {
            Ok(0) => {
                println!("Client closed the connection");
                break;
            }
            Err(e) => return Err(e.into()),
            Ok(_) => {
                loop {
                    match Command::try_parse_resp(&buf) {
                        Ok(Some((command, bytes_processed))) => {
                            buf.drain(..bytes_processed);
                            let processing_result = service.process_command_resp(command).await?; //TODO graceful error handling needed here
                            write_and_flush(&mut writer, serialize(processing_result)).await?;
                        }
                        Ok(None) => break,
                        Err(e) => {
                            write_and_flush(&mut writer, format!("-ERR {e}\r\n").into_bytes())
                                .await?;
                            return Ok(());
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

async fn write_and_flush(
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
        io::{Cursor, Write},
        sync::{Arc, Mutex},
    };

    use tokio::io::{AsyncWrite, BufReader};

    use crate::{
        handle_connection,
        service::{Clock, Executor, SystemClock},
        store::{HashMapStore, Store},
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
    async fn test_multiple_ping() {
        let mut executor = Executor {
            store: HashMapStore::default(),
            clock: SystemClock {},
        };
        let response = send_and_get(
            b"*2\r\n$4\r\nPING\r\n$3\r\nmsg\r\n*1\r\n$4\r\nPING\r\n",
            &mut executor,
        )
        .await;
        assert_eq!(response, "$3\r\nmsg\r\n+PONG\r\n");
    }

    #[tokio::test]
    async fn test_echo() {
        let mut executor = Executor {
            store: HashMapStore::default(),
            clock: SystemClock {},
        };
        let response = send_and_get(b"*2\r\n$4\r\nECHO\r\n$3\r\nmsg\r\n", &mut executor).await;
        assert_eq!(response, "$3\r\nmsg\r\n");
    }

    #[tokio::test]
    async fn should_store_and_read() {
        let mut executor = Executor {
            //TODO: just default executor
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
    async fn should_store_with_get() {
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

        let response = send_and_get(
            b"*4\r\n$3\r\nSET\r\n$4\r\nname\r\n$4\r\nKHnv\r\n$3\r\nGET\r\n",
            &mut executor,
        )
        .await;
        assert_eq!(response, "$4\r\nMoez\r\n");
    }

    #[tokio::test]
    async fn should_honor_set_nx() {
        let mut executor = Executor {
            store: HashMapStore::default(),
            clock: SystemClock {},
        };

        let response = send_and_get(
            b"*4\r\n$3\r\nSET\r\n$4\r\nname\r\n$4\r\nMoez\r\n$2\r\nNX\r\n",
            &mut executor,
        )
        .await;
        assert_eq!(response, "+OK\r\n");

        let response = send_and_get(
            b"*4\r\n$3\r\nSET\r\n$4\r\nname\r\n$4\r\nMoez\r\n$2\r\nNX\r\n",
            &mut executor,
        )
        .await;
        assert_eq!(response, "$-1\r\n");
    }

    #[tokio::test]
    async fn should_honor_set_xx() {
        let mut executor = Executor {
            store: HashMapStore::default(),
            clock: SystemClock {},
        };

        let response = send_and_get(
            b"*4\r\n$3\r\nSET\r\n$4\r\nname\r\n$4\r\nMoez\r\n$2\r\nXX\r\n",
            &mut executor,
        )
        .await;
        assert_eq!(response, "$-1\r\n");

        let response = send_and_get(
            b"*3\r\n$3\r\nSET\r\n$4\r\nname\r\n$4\r\nMoez\r\n",
            &mut executor,
        )
        .await;
        assert_eq!(response, "+OK\r\n");

        let response = send_and_get(
            b"*4\r\n$3\r\nSET\r\n$4\r\nname\r\n$4\r\nMoez\r\n$2\r\nXX\r\n",
            &mut executor,
        )
        .await;
        assert_eq!(response, "+OK\r\n");
    }
}
