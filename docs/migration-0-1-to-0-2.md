# Migration guide: 0.1 -> 0.2

## Breaking changes

### `extract_replay_cursor` - signature change

**Before (0.1):**

```rust
use actix_v2a::extract_replay_cursor;
use actix_web::HttpRequest;

let cursor = extract_replay_cursor(req.headers())?;
```

**After (0.2) - Actix callers use the adapter:**

```rust
use actix_v2a::extract_actix_replay_cursor;
use actix_web::HttpRequest;

let cursor = extract_actix_replay_cursor(req.headers())?;
```

The domain function `extract_replay_cursor` now accepts `&[SseHeader]` and is
framework-independent. Actix Web callers must use `extract_actix_replay_cursor`
from the same crate root.

---

### `apply_event_stream_cache_control` - signature change

**Before (0.1):**

```rust
use actix_v2a::apply_event_stream_cache_control;
use actix_web::http::header::HeaderMap;

let mut headers = HeaderMap::new();
apply_event_stream_cache_control(&mut headers);
```

**After (0.2) - Actix callers use the adapter:**

```rust
use actix_v2a::apply_actix_event_stream_cache_control;
use actix_web::http::header::HeaderMap;

let mut headers = HeaderMap::new();
apply_actix_event_stream_cache_control(&mut headers);
```

The domain function `apply_event_stream_cache_control` now accepts
`&mut Vec<SseHeader>`. Actix Web callers must use
`apply_actix_event_stream_cache_control`.
