

# nwd1-quic

**QUIC transport for [`nwd1`](https://crates.io/crates/nwd1) binary frames.**

`nwd1-quic` provides async helpers to send and receive `nwd1` frames over QUIC streams, using the [`quinn`](https://crates.io/crates/quinn) library.
It integrates seamlessly with [`netid64`](https://crates.io/crates/netid64) identifiers and uses Tokio runtime.

---

## 🧱 Frame Layout

Each frame follows the [`nwd1`](https://crates.io/crates/nwd1) binary format:

```text
MAGIC (4B) | LEN (4B) | ID (8B) | KIND (1B) | VER (8B) | PAYLOAD (variable)
```

- `MAGIC`: constant `b"NWD1"` header
- `LEN`: total byte length following header (u32, BE)
- `ID`: 64-bit network ID ([netid64](https://crates.io/crates/netid64))
- `KIND`: frame type discriminator
- `VER`: protocol version
- `PAYLOAD`: binary application data

---

## ⚙️ Example

```rust
use nwd1_quic::{send_frame, recv_frame};
use nwd1::Frame;
use netid64::NetId64;
use bytes::Bytes;
use quinn::{RecvStream, SendStream};

async fn example(mut send: SendStream, mut recv: RecvStream) -> anyhow::Result<()> {
    let frame = Frame {
        id: NetId64::make(1, 7, 42),
        kind: 1,
        ver: 1,
        payload: Bytes::from_static(b"hello"),
    };

    // Send frame
    send_frame(&mut send, &frame).await?;

    // Receive frame
    if let Some(received) = recv_frame(&mut recv).await? {
        assert_eq!(received.id.raw(), frame.id.raw());
    }
    Ok(())
}
```

---

## 🧩 Design

- Fully async, `tokio` + `quinn`
- Uses `read_exact_opt()` helper for compact error handling
- Checks frame `MAGIC` early to avoid wasteful allocations
- Enforces maximum frame length (`MAX_FRAME_LEN = 8 MiB`) for safety

---

## 📦 License

Licensed under either of:

- MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)

at your option.