# Logging Guidelines

## 1. Scope / Trigger

API and migration startup, application tracing events and scheduled workers.
Both binaries load dotenv before calling `infra::logging::init()`.

## 2. Signatures

- `init()` installs `ChineseLogFormat` and an `EnvFilter`.
- Output: UTC RFC3339 milliseconds, Chinese level, Chinese application message,
  labeled event fields, and the stable technical module path, on one line.
- Empty or plain `RUST_LOG=info` becomes
  `warn,exchange_api=info,exchange_migrate=info`; explicit module filters remain
  effective. Invalid filters fall back to the same bounded default.

## 3. Contracts

Use INFO for startup and meaningful work summaries, WARN for recoverable
failures, ERROR for failed batches, DEBUG for routine high-frequency rounds.
Idle scans do not emit INFO. Settlement summaries include counts and elapsed
milliseconds; startup prints the effective scan interval.

Keep stable IDs, provider names, module paths and underlying diagnostic errors
intact. Translate application messages and known field labels, not API error
contracts or arbitrary external strings. Extend `field_label` for new recurring
operational fields; unknown names remain visible.

Do not attach passwords, tokens, complete callback payloads or private request
URLs to events. The formatter omits request spans, redacts known sensitive field
names, sanitizes database URI passwords and collapses control characters. This
is defense in depth, not permission to log arbitrary secret-containing text.

## 4. Validation & Error Matrix

| Input | Output |
| --- | --- |
| INFO settlement with counts | Chinese level/message/count labels |
| Idle worker | No INFO line |
| Explicit diagnostic filter | Filter remains effective |
| Payload/token fields | Hidden marker, never source text |
| URI password or newline in error | Password masked, one physical line |
| Unknown technical field/error | Original diagnostic retained |

## 5. Good/Base/Bad Cases

Good: one completed batch with scanned/settled/failed/elapsed fields.
Base: startup configuration is visible without third-party INFO noise.
Bad: a per-second idle message, multiline payload, or token-bearing request span.

## 6. Tests Required

`tests/unit_src/src_infra_logging_tests.rs` executes the formatter with a memory
writer, asserting Chinese labels, single-line output, request-span omission,
payload/token redaction and database-password masking. Run the library,
architecture, documentation, formatting and strict Clippy gates.

## 7. Wrong Vs Correct

```text
Wrong: INFO on every empty poll; raw callback payload; request URI with token
Correct: INFO only for work; identifiers/counts; sanitized diagnostic error
```
