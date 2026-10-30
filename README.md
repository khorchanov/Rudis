# Rudis

A toy, Redis-like in-memory store written in Rust. It speaks RESP, so you can talk to it with `redis-cli` or any Redis client.

This is a learning project, built without/minimal an AI coding agent help for now (minimal on some test generations). The goal is to have fun. I'm building the foundation bit by bit, in very small steps.

In the long term I want to focus on performance: as fast as possible, with as little memory as possible.

> **Status:** early and experimental. Not for production use.

## Getting started

You need a recent Rust toolchain (edition 2024).

```sh
cargo run
```

The server listens on `0.0.0.0:8080`. In another terminal:

```sh
redis-cli -p 8080
> SET greeting hello EX 60
OK
> GET greeting
"hello"
> TTL greeting
(integer) 59
```

Run the tests with:

```sh
cargo test
```
