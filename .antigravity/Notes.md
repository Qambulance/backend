# Qambulance Backend – Dev Notes

## 2026-11-12 — `/quick` returning bare 500 (fix applied)

### Root cause
`blank-web` v0.2.1's convenience response methods (e.g. `internal_server_error()`, `bad_request()`) **set both the HTTP status code AND overwrite the body** with a plain-text string (e.g. `"Internal Server Error"`).

The three 500 error paths in `src/main.rs` had the chain in the **wrong order**:

```rust
// BROKEN — internal_server_error() runs last, overwrites JSON body
out.json(OVERPASS_REQUEST_FAILED).internal_server_error();
```

This meant clients received HTTP 500 with body `Internal Server Error` (plain text, not JSON) instead of our custom JSON payload.

### Fix (branch `fix/debug-quick-500`, commit `e0a001b`)
Swap the chain order so `.json(...)` is called **last**:

```rust
// CORRECT — json() runs last, body is our custom JSON
out.internal_server_error().json(OVERPASS_REQUEST_FAILED);
```

Applied to all three 500 paths:
- `OVERPASS_REQUEST_FAILED` (Overpass unreachable)
- `OVERPASS_RESPONSE_READ_FAILED` (body read error)
- `OVERPASS_RESPONSE_PARSE_FAILED` (JSON parse error)

The 400 paths (`out.bad_request().json(...)`) were already correct — `.json()` was already last.

### Framework gotcha (blank-web)
> **Rule**: In `blank-web`, always call status-setting helpers **before** body-setting helpers in a chain, because each helper overwrites whatever was set before it. The last body call wins.

### xk note
When testing from this machine, Overpass (`maps.mail.ru`) is unreachable. The `/quick` endpoint will return HTTP 500 with:
```json
{"ok": false, "hospitals": [], "msg": "Failed to contact Overpass API"}
```
This is correct error-handling behaviour — not a code bug. The Overpass URL is `https://maps.mail.ru/osm/tools/overpass/api/interpreter`.

### Open questions / follow-up
- Consider adding a fallback Overpass mirror (e.g. `overpass-api.de`) for resilience.
- The `OVERPASS_RESPONSE_LIMIT` is 2 MB — may need tuning for dense urban areas.
- `CREDITS` constant contains invalid JSON (object values use `":"` pairs which aren't valid JSON — this will silently serve malformed JSON on `/credits`).

## 2026-11-12 — blank-web framework fix (upstream)

The root issue was in the **framework itself** (`/home/kyam/blank`). Error-status convenience methods like `internal_server_error()` were calling both `self.status(...)` **and** `self.text(BODY_*)`, silently overwriting any body the caller had already set.

**Fix committed and pushed to `dev` branch** (commit `c5fe2f1`):  
→ https://github.com/its-kyam/blank/tree/dev

All 12 error-status methods now **only set the status** (consistent with `ok()` and `created()`), leaving the body untouched so chaining `.json(...)` or `.text(...)` after always works as expected.

Methods changed: `bad_request`, `unauthorized`, `forbidden`, `not_found`, `method_not_allowed`, `request_timeout`, `internal_server_error`, `payload_too_large`, `too_many_requests`, `bad_gateway`, `gateway_timeout`, `service_unavailable`.

**160 tests pass, 0 warnings after fix.**

Once the PR is merged and a new version is published to crates.io, the workaround in `qambulance-backend/src/main.rs` (branch `fix/debug-quick-500`) can be reverted — both call orders will work correctly from that point on.
