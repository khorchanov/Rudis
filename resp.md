# RESP (RESP2) protocol notes

## 1. General principles

- RESP is a request/response protocol over TCP (Redis default port: 6379).
- Every value starts with a **one-byte type prefix** and ends with **CRLF** (`\r\n`, bytes 13 and 10).
- The first byte determines how to read the rest.
- Clients send commands as **arrays of bulk strings**. Servers reply with any type.
- Pipelining: a client may send many commands without waiting for replies. The server answers them **in order**.
- Lengths and counts are decimal ASCII numbers, not binary.

## 2. The five types

| Prefix | Type | Format |
|---|---|---|
| `+` | Simple string | `+<text>\r\n` |
| `-` | Error | `-<text>\r\n` |
| `:` | Integer | `:<number>\r\n` |
| `$` | Bulk string | `$<length>\r\n<bytes>\r\n` |
| `*` | Array | `*<count>\r\n<element 1>...<element N>` |

### Simple string (`+`)
- Example: `+OK\r\n`
- Text runs to the first CRLF and cannot contain `\r` or `\n`.
- Used for short status replies (`OK`, `PONG`).

### Error (`-`)
- Example: `-ERR unknown command 'FOO'\r\n`
- Same layout as a simple string.
- By convention the first word is an error kind in uppercase: `ERR`, `WRONGTYPE`, etc., followed by a message.
- Clients treat it as a failure, but the connection stays open.

### Integer (`:`)
- Example: `:1000\r\n`, `:-5\r\n`
- Signed 64-bit decimal, optional leading `-`.
- Used for counts and numeric results (`DEL`, `INCR`, `EXISTS`, `TTL`).

### Bulk string (`$`)
- Example: `$5\r\nhello\r\n`
- Header line gives the **byte length**. Then exactly that many raw bytes. Then a final CRLF.
- The trailing CRLF is **not** counted in the length.
- Binary-safe: the content may contain anything, including `\r\n`, so never search the payload for a terminator. Trust the length.
- Empty string: `$0\r\n\r\n`
- Null: `$-1\r\n`. There is no payload and **no** trailing CRLF.
- The length counts **bytes**, not characters (`é` is 2 bytes in UTF-8).

### Array (`*`)
- Example: `*2\r\n$3\r\nfoo\r\n$3\r\nbar\r\n`
- Header line gives the **element count**. Then that many complete RESP values, back to back.
- Elements can be of any type, including nested arrays and nulls.
- Empty array: `*0\r\n`
- Null array: `*-1\r\n`

## 3. Requests (client → server)

A command is an array of bulk strings. The first element is the command name (case-insensitive), the rest are arguments.

- `PING` → `*1\r\n$4\r\nPING\r\n`
- `GET name` → `*2\r\n$3\r\nGET\r\n$4\r\nname\r\n`
- `SET name bob` → `*3\r\n$3\r\nSET\r\n$4\r\nname\r\n$3\r\nbob\r\n`

Reading `SET name bob` in order: `*3` means three elements, then `$3 SET`, `$4 name`, `$3 bob`.

### Inline commands (optional)
- A plain text line such as `PING\r\n` or `SET a b\r\n`, split on spaces.
- Used by humans with `telnet` or `nc`.
- Detected when the first byte is **not** `*`.
- Safe to skip at first, add later for easier manual testing.

## 4. Replies (server → client)

| Situation | Reply |
|---|---|
| `PING` | `+PONG\r\n` |
| `PING hello` | bulk string `hello` |
| `SET` success | `+OK\r\n` |
| `GET`, key exists | bulk string with the value |
| `GET`, key missing | `$-1\r\n` (null bulk) |
| `DEL a b c` | integer: number of keys removed |
| `EXISTS` | integer: number of keys found |
| `INCR` | integer: the new value |
| Unknown command | `-ERR unknown command ...\r\n` |
| Wrong arg count | `-ERR wrong number of arguments for 'get' command\r\n` |
| Wrong value type | `-WRONGTYPE Operation against a key holding the wrong kind of value\r\n` |
| `LRANGE`, `KEYS`, etc. | array of bulk strings |

## 5. Parsing rules to implement

1. Read the first byte and branch on the prefix.
2. For `+`, `-`, `:`, and the headers of `$` and `*`, find the first CRLF. The text between the prefix and the CRLF is the line.
3. For `$`: parse the length. If it is `-1`, the value is null. Otherwise you need `length + 2` more bytes after the header (payload plus CRLF).
4. For `*`: parse the count. If it is `-1`, the value is null. Otherwise parse that many values recursively, one after another.
5. Negative lengths other than `-1` are a protocol error.
6. The two bytes after a bulk payload must be exactly `\r\n`, otherwise it is a protocol error.

### Incomplete data is normal
TCP delivers arbitrary chunks. A buffer may contain half a command, one command, or several commands glued together.

- If the buffer ends before the value is complete (missing CRLF, short payload, missing array elements), report **"need more data"**. This is not an error.
- On success, report **how many bytes were consumed** so the caller can discard them and keep the rest for the next command.
- Loop until the buffer holds no complete command.

### Real errors
- Unknown type prefix byte.
- Non-numeric length or count.
- Negative length other than `-1`.
- Bulk payload not followed by CRLF.
- Reasonable limits are good practice: Redis caps bulk strings at 512 MB and rejects huge array counts, so a malicious client can't make you allocate gigabytes.

On a protocol error, Redis sends an `-ERR Protocol error: ...` reply and **closes the connection**, since the stream can no longer be trusted. A command-level error (unknown command, wrong args) keeps the connection open.

## 6. Server-side flow

1. Accept a connection and keep a per-connection read buffer.
2. Read bytes and append them to the buffer.
3. Try to parse one complete value from the front of the buffer.
4. If complete: remove those bytes, verify it is a non-empty array of bulk strings, uppercase the first element, and dispatch the command.
5. Encode the reply in RESP and write it.
6. Repeat from step 3 until "need more data", then read again.

## 7. Command validation

- The request must be an array with at least one element, each element a bulk string.
- Command names are case-insensitive (`get`, `GET`, `Get`).
- Check the argument count per command before executing and return the standard error if it is wrong.
- Keys and values are arbitrary bytes, not necessarily UTF-8.

## 8. RESP3 (for later)

RESP3 adds more prefixes: `_` null, `#` boolean, `,` double, `%` map, `~` set, `>` push, `(` big number, `=` verbatim string. A client opts in with the `HELLO 3` command. Real clients still work if you only implement RESP2 and answer `HELLO` by refusing or ignoring it.

## 9. Testing checklist

- `redis-cli -p <port> ping` returns `PONG`.
- Pipelined input (two commands in one write) gives two replies in order.
- A command split across multiple writes still works.
- A value containing `\r\n` round-trips through `SET` and `GET`.
- A missing key returns `$-1\r\n`.
- Garbage input produces an error without crashing the server.

Official spec: https://redis.io/docs/latest/develop/reference/protocol-spec/