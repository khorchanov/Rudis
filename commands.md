# Redis commands and expected replies (RESP2 only)

Rudis speaks **RESP2 only**. RESP3 types (`_`, `%`, `~`, `#`, `,`, `=`, `>`) are never sent, and inline commands are not supported.

## Protocol

**Requests** are always an array of bulk strings: `*2\r\n$3\r\nGET\r\n$1\r\nk\r\n`. Inline (plain text) commands are rejected.

**Reply types**

| Prefix | Type | Example |
|---|---|---|
| `+` | simple string | `+OK\r\n` |
| `-` | simple error | `-ERR syntax error\r\n` |
| `:` | integer | `:1\r\n` |
| `$` | bulk string | `$3\r\nfoo\r\n` |
| `$-1` | null bulk string | `$-1\r\n` (missing key, failed `NX`/`XX`) |
| `*` | array | `*2\r\n:1\r\n:2\r\n` |
| `*-1` | null array | `*-1\r\n` (aborted `EXEC`) |

Maps and sets do not exist in RESP2. They are flat arrays: a map is `[k1, v1, k2, v2]`, a set is a plain array of members.

**Handshake.** Clients may open with `HELLO`. Only RESP2 is available, so `HELLO 3` is refused with `-NOPROTO unsupported protocol version\r\n` and the client falls back to RESP2 (go-redis, Lettuce and node-redis do this). `HELLO 2` and a bare `HELLO` are accepted. Clients that never send `HELLO` (redis-py, ioredis, Jedis by default) just work.

## Tier 1: required for SDK compatibility

Clients send these on connect or on every use. Without them most SDKs cannot connect.

### Connection / server

| Command | Reply | Notes |
|---|---|---|
| `HELLO` / `HELLO 2 [SETNAME name]` | flat array `["server","redis","version","8.0.0","proto",2,"id",<n>,"mode","standalone","role","master","modules",[]]` | a map flattened to key/value pairs |
| `HELLO 3` | `-NOPROTO unsupported protocol version\r\n` | any version other than 2 |
| `HELLO 2 AUTH u p` | `-ERR AUTH <password> called without any password configured...` | no auth support; clients without credentials never send it |
| `PING` | `+PONG\r\n` | |
| `PING msg` | `$3\r\nmsg\r\n` | echoes the argument as bulk |
| `ECHO msg` | `$3\r\nmsg\r\n` | |
| `QUIT` | `+OK\r\n` | then close the connection |
| `RESET` | `+RESET\r\n` | back to the initial connection state |
| `CLIENT SETNAME x` | `+OK\r\n` | |
| `CLIENT SETINFO LIB-NAME x` / `LIB-VER x` | `+OK\r\n` | sent by most clients on connect |
| `CLIENT ID` | `:<id>\r\n` | |
| `SELECT 0` | `+OK\r\n` | `-ERR DB index is out of range\r\n` for any other index (single database) |
| `INFO [section]` | bulk string | `key:value\r\n` lines, at least `redis_version` and `redis_mode:standalone` |

### Keys and strings

| Command | Reply | Notes |
|---|---|---|
| `SET k v [NX\|XX] [GET] [EX s\|PX ms\|EXAT ts\|PXAT ts\|KEEPTTL]` | `+OK\r\n` | overwrites any value and clears the TTL unless `KEEPTTL` |
| `SET ... NX` / `XX` | `+OK\r\n` or `$-1\r\n` | null if the condition fails |
| `SET ... GET` | old value as bulk, or `$-1\r\n` | |
| `GET k` | `$n\r\nvalue\r\n` or `$-1\r\n` | `-WRONGTYPE ...` once other types exist |
| `DEL k1 k2` | `:<number deleted>\r\n` | |
| `EXISTS k1 k2` | `:<number existing>\r\n` | counts duplicates |

`SET` option rules: `NX` with `XX`, or more than one expiry option, is `-ERR syntax error`. A zero/negative expiry is `-ERR invalid expire time in 'set' command`. A non-integer is `-ERR value is not an integer or out of range`.

## Tier 2: common, implement when an app needs them

| Command | Reply | Notes |
|---|---|---|
| `EXPIRE k s` / `PEXPIRE k ms` | `:1\r\n` / `:0\r\n` | `:0` if the key is missing |
| `TTL k` / `PTTL k` | `:<n>\r\n` | `:-1` no expiry, `:-2` no key |
| `PERSIST k` | `:1\r\n` / `:0\r\n` | |
| `MSET k1 v1 k2 v2` | `+OK\r\n` | |
| `MGET k1 k2` | array of bulk / null: `*2\r\n$1\r\na\r\n$-1\r\n` | |
| `INCR k` / `DECR k` | `:<new value>\r\n` | missing key counts as 0 |
| `INCRBY k n` / `DECRBY k n` | `:<new value>\r\n` | |
| `GETDEL k` | bulk or `$-1\r\n` | |
| `APPEND k v` | `:<new length>\r\n` | |
| `STRLEN k` | `:<length>\r\n` | `:0` if missing |
| `UNLINK k1 k2` | `:<number deleted>\r\n` | alias of `DEL` |
| `TYPE k` | `+string\r\n` / `+none\r\n` | |
| `KEYS pattern` | array of bulk | `*0\r\n` if none |
| `SCAN cursor [MATCH p] [COUNT n]` | `*2` of `[cursor bulk, array of keys]` | SDK iterators use this instead of `KEYS` |
| `DBSIZE` | `:<count>\r\n` | |
| `FLUSHALL` / `FLUSHDB` | `+OK\r\n` | |
| `COMMAND DOCS` | `*0\r\n` | an empty array is enough for redis-cli |
| `CONFIG GET x` | flat array `[name, value]` | or `*0\r\n` |

Integer commands on a non-integer value reply `-ERR value is not an integer or out of range\r\n`.

## Tier 3: data structures, pub/sub, transactions (only if needed)

| Command | Reply | Notes |
|---|---|---|
| `LPUSH k a b` / `RPUSH k a b` | `:<new length>\r\n` | |
| `LPOP k` / `RPOP k` | bulk or `$-1\r\n` | |
| `LLEN k` | `:<length>\r\n` | |
| `LRANGE k 0 -1` | array of bulk | `*0\r\n` if empty |
| `HSET k f v [f v ...]` | `:<number of new fields>\r\n` | |
| `HGET k f` | bulk or `$-1\r\n` | |
| `HDEL k f [f ...]` | `:<number removed>\r\n` | |
| `HGETALL k` | flat array `[f1, v1, f2, v2]` | `*0\r\n` if none |
| `SADD k a b` | `:<number added>\r\n` | |
| `SREM k a` | `:<number removed>\r\n` | |
| `SISMEMBER k a` | `:1\r\n` / `:0\r\n` | |
| `SMEMBERS k` | array of bulk | `*0\r\n` if none |
| `SUBSCRIBE ch` | array `["subscribe", "ch", 1]` | then pushed `["message", "ch", payload]`; a subscribed connection only accepts (P)(UN)SUBSCRIBE, `PING` and `QUIT` |
| `UNSUBSCRIBE ch` | array `["unsubscribe", "ch", 0]` | |
| `PUBLISH ch msg` | `:<number of receivers>\r\n` | |
| `MULTI` | `+OK\r\n` | commands inside reply `+QUEUED\r\n` |
| `EXEC` | array of each command's reply | `*-1\r\n` if aborted |
| `DISCARD` | `+OK\r\n` | |

## Not implemented on purpose

- RESP3 (`HELLO 3`) and inline commands.
- Deprecated commands, which `SET` options replace: `SETNX`, `SETEX`, `PSETEX`, `GETSET`. Also `HMSET` (use `HSET`), `RPOPLPUSH` (use `LMOVE`), `SUBSTR` (use `GETRANGE`).
- `AUTH` and ACLs, multiple databases, cluster, replication, persistence, Lua scripting, streams, modules.

## Standard errors

| Situation | Reply |
|---|---|
| Unknown command | `-ERR unknown command 'foo', with args beginning with: \r\n` |
| Wrong arg count | `-ERR wrong number of arguments for 'get' command\r\n` |
| Wrong value type | `-WRONGTYPE Operation against a key holding the wrong kind of value\r\n` |
| Not an integer | `-ERR value is not an integer or out of range\r\n` |
| Syntax error | `-ERR syntax error\r\n` |
| Unsupported protocol (`HELLO 3`) | `-NOPROTO unsupported protocol version\r\n` |
| Protocol error | `-ERR Protocol error: ...\r\n`, then close the connection |

## Rules that apply everywhere

- Command names are case-insensitive. Keys and values are case-sensitive and binary-safe.
- Command-level errors keep the connection open. Only protocol errors close it.
- Replies come in the same order as requests, even when pipelined. Pub/sub messages may arrive between replies on a subscribed connection.
- A missing key returns a null bulk string (`$-1`) for reads, `:0` for counts, and an empty array (`*0`) for collection reads.

Check any command's exact behavior with `redis-cli COMMAND DOCS <name>` or https://redis.io/docs/latest/commands/<name>/. A real Redis 8 running beside yours (`redis-cli` talks RESP2 by default) is the best oracle for byte-for-byte comparison.
