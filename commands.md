# Redis commands and expected replies (RESP2)

Reply types: `+` simple string, `-` error, `:` integer, `$` bulk, `*` array, `$-1` null bulk.

## Connection / server

| Command | Reply on the wire | Notes |
|---|---|---|
| `PING` | `+PONG\r\n` | |
| `PING msg` | `$3\r\nmsg\r\n` | echoes the argument as bulk |
| `ECHO msg` | `$3\r\nmsg\r\n` | bulk |
| `QUIT` | `+OK\r\n` | then close the connection |
| `SELECT n` | `+OK\r\n` | or `-ERR DB index is out of range\r\n` |
| `HELLO` / `HELLO 2` | array of server info (or error) | simplest: `-ERR unknown command 'HELLO'\r\n`, clients fall back to RESP2 |
| `CLIENT SETNAME x` | `+OK\r\n` | |
| `CLIENT SETINFO ...` | `+OK\r\n` | sent by many client libs on connect |
| `COMMAND` / `COMMAND DOCS` | `*0\r\n` | empty array is enough for redis-cli |
| `INFO` | bulk string with `key:value\r\n` lines | e.g. `$..\r\n# Server\r\nredis_version:0.1\r\n` |
| `CONFIG GET x` | array `[name, value]` or `*0\r\n` | |
| `FLUSHALL` / `FLUSHDB` | `+OK\r\n` | |
| `DBSIZE` | `:<count>\r\n` | |

## Strings

| Command | Reply | Notes |
|---|---|---|
| `SET k v` | `+OK\r\n` | overwrites any existing value |
| `SET k v NX` | `+OK\r\n` or `$-1\r\n` | null if key already exists |
| `SET k v XX` | `+OK\r\n` or `$-1\r\n` | null if key doesn't exist |
| `SET k v EX s` / `PX ms` | `+OK\r\n` | sets TTL |
| `GET k` | `$n\r\nvalue\r\n` or `$-1\r\n` | `-WRONGTYPE ...` if not a string |
| `GETDEL k` | bulk or `$-1\r\n` | |
| `SETNX k v` | `:1\r\n` set / `:0\r\n` not set | |
| `MSET k1 v1 k2 v2` | `+OK\r\n` | |
| `MGET k1 k2` | array of bulk/null: `*2\r\n$1\r\na\r\n$-1\r\n` | |
| `APPEND k v` | `:<new length>\r\n` | |
| `STRLEN k` | `:<length>\r\n` | `:0` if missing |
| `INCR k` / `DECR k` | `:<new value>\r\n` | missing key counts as 0 |
| `INCRBY k n` / `DECRBY k n` | `:<new value>\r\n` | |
| any of the above on non-integer | `-ERR value is not an integer or out of range\r\n` | |

## Keys

| Command | Reply | Notes |
|---|---|---|
| `DEL k1 k2` | `:<number deleted>\r\n` | |
| `EXISTS k1 k2` | `:<number existing>\r\n` | counts duplicates |
| `TYPE k` | `+string\r\n` / `+list\r\n` / `+none\r\n` | simple string |
| `KEYS pattern` | array of bulk strings | `*0\r\n` if none |
| `RENAME a b` | `+OK\r\n` | `-ERR no such key\r\n` if missing |
| `EXPIRE k s` | `:1\r\n` set / `:0\r\n` key missing | |
| `PEXPIRE k ms` | `:1\r\n` / `:0\r\n` | |
| `TTL k` | `:<seconds>\r\n`, `:-1` no expiry, `:-2` no key | |
| `PTTL k` | same, in milliseconds | |
| `PERSIST k` | `:1\r\n` / `:0\r\n` | |

## Lists

| Command | Reply | Notes |
|---|---|---|
| `LPUSH k a b` / `RPUSH k a b` | `:<new length>\r\n` | |
| `LPOP k` / `RPOP k` | bulk or `$-1\r\n` | |
| `LLEN k` | `:<length>\r\n` | |
| `LRANGE k 0 -1` | array of bulk | `*0\r\n` if empty |
| `LINDEX k i` | bulk or `$-1\r\n` | |

## Hashes

| Command | Reply | Notes |
|---|---|---|
| `HSET k f v` | `:<number of new fields>\r\n` | |
| `HGET k f` | bulk or `$-1\r\n` | |
| `HDEL k f` | `:<number removed>\r\n` | |
| `HEXISTS k f` | `:1\r\n` / `:0\r\n` | |
| `HGETALL k` | flat array `[f1, v1, f2, v2]` | `*0\r\n` if none |
| `HLEN k` | `:<count>\r\n` | |

## Sets

| Command | Reply | Notes |
|---|---|---|
| `SADD k a b` | `:<number added>\r\n` | |
| `SREM k a` | `:<number removed>\r\n` | |
| `SISMEMBER k a` | `:1\r\n` / `:0\r\n` | |
| `SMEMBERS k` | array of bulk | |
| `SCARD k` | `:<count>\r\n` | |

## Pub/Sub and transactions

| Command | Reply | Notes |
|---|---|---|
| `SUBSCRIBE ch` | array `["subscribe", "ch", 1]` | then pushed `["message", "ch", payload]` |
| `UNSUBSCRIBE ch` | array `["unsubscribe", "ch", 0]` | |
| `PUBLISH ch msg` | `:<number of receivers>\r\n` | |
| `MULTI` | `+OK\r\n` | |
| `QUEUED` (for commands inside MULTI) | `+QUEUED\r\n` | |
| `EXEC` | array of each command's reply | `$-1`/`*-1` if aborted |
| `DISCARD` | `+OK\r\n` | |

## Standard errors

| Situation | Reply |
|---|---|
| Unknown command | `-ERR unknown command 'foo', with args beginning with: \r\n` |
| Wrong arg count | `-ERR wrong number of arguments for 'get' command\r\n` |
| Wrong value type | `-WRONGTYPE Operation against a key holding the wrong kind of value\r\n` |
| Not an integer | `-ERR value is not an integer or out of range\r\n` |
| Syntax error | `-ERR syntax error\r\n` |
| Protocol error | `-ERR Protocol error: ...\r\n`, then close the connection |

## Rules that apply everywhere

- Command names are case-insensitive. Keys and values are case-sensitive and binary-safe.
- Command-level errors keep the connection open. Only protocol errors close it.
- Replies come in the same order as requests, even when pipelined.
- A missing key returns null (`$-1`) for reads, `:0` for counts, and an empty array (`*0`) for collection reads.

Check any command's exact behavior with `redis-cli COMMAND DOCS <name>` or the page at https://redis.io/docs/latest/commands/<name>/. A real Redis running beside yours is the best oracle for byte-for-byte comparison.