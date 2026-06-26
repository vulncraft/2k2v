# Record format

The Write ahead log is a sequence of Records that detail the operations applied to the kv store. A record is defined as:

| Field     | Type    | Description                                                                 |
|-----------|---------|-----------------------------------------------------------------------------|
| `len`     | `u32`   | Total byte count of the record, including this field and the trailing CRC32 |
| `type`    | `u16`    | Record type (see below)                                                     |
| `version` | `u16`    | Schema version this record follows (currently `1`)                          |
| `seq`     | `u64`   | Monotonically increasing sequence number                                    |
| `data`    | `[u8]`  | rkyv-serialized payload (length = `len` - 16 - 4)                          |
| `crc32`   | `u32`   | CRC32 over all preceding bytes in the record (from `len` up to `data` end) |

Header size is 16 bytes (`u32` + `u16` + `u16` + `u64`), trailer is 4 bytes (`u32` CRC32).

```
Offset  Size  Field
──────────────────────────────────────────────────
0       4     len
4       2     type
6       2     version
8       8     seq
16      N     data
16+N    4     crc32
──────────────────────────────────────────────────
Total: 16 + N + 4 bytes

+──────────+──────+─────────+───────────────────────+
│  len (4) │ T(2) │ ver (2) │      seq (8)          │ 
+──────────+──────+─────────+───────────────────────+ 
│                      data (N bytes)               │
+───────────────────────────────────────────────────+
│                      crc32 (4)                    │
+───────────────────────────────────────────────────+
```

## Record types

| Value  | Name         | Description                                      |
|--------|--------------|--------------------------------------------------|
| `0x01` | `DATA`       | A store operation (Put, Delete, etc.)            |
| `0x02` | `NOOP_FENCE` | No-op barrier used to force a sync point         |

## Torn write detection

On recovery, each record is validated by:
1. Reading `len` and checking it is plausible.
2. Reading the remaining `len - 4` bytes.
3. Recomputing CRC32 over all bytes from `len` through `data` and comparing against the stored `crc32`.

A CRC mismatch or a short read indicates a torn write. Recovery truncates the WAL at the last fully valid record.
