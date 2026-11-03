# Architecture decision record 0001: Thin TS plugin, native Rust MCP

## Decision

Use one Cordis provider plugin with one standard MCP stdio connection. TypeScript does not import, link or compile Rust. Rust is a separate executable exposing native desktop tools.

- DSH owns the agent, session, approvals, tool registry, output retention and image projection.
- TypeScript reserves the `computerUse` slot; starts/configures the DSH-maintained MCP client; releases the server connection *before* provider release on unload.
- Rust owns local desktop discovery, screenshot, model-facing UI observations, input injection and future accessibility APIs. The model description format is native-side; TS does not serialize UI trees.
- No second in-process runtime or RPC proxy.

## Why

The performance bottlenecks of Computer Use are desktop API latency, data transfer and model round trips rather than the language used for simple connection logic. A Rust MCP can return cropped/encoded images and batch deterministic actions without coupling to Harness internals.

## Concurrency

Rust serializes individual screenshot/action calls on a per-process mutex; this cannot serialize *multi-step workflows* across DSH sessions or independent DSH processes. Coordinate workflows in the calling agent, or later add lease/token support with expiry.

## Correctness and safety

- Physical desktop coordinates must be explicit. Image returned from a monitor-local region includes global origin.
- Element identifiers (future) must be tied to the window, generation, and a fresh state check.
- Partial execution of action batches is possible and is accurately reported.
- Critical clicks/writes need explicit approvals and postconditions; this scaffold intentionally exposes only the primitive backend.
- The plugin enforces mutual exclusion at the provider level, not per-app permissions. A production build must add per-app consent.

## Performance instrumentation proposal

Measure `get_displays`, `list_windows`, `capture_region`, `jpeg_encode`, `mcp_serialization`, `input_injection` and end-to-end `act` latency separately. Benchmark P50/P95/P99 and memory with realistic desktops. Avoid artificial Rust-vs-TS claims.

## Model-facing UI description (ADR-0002)

The Rust MCP server attaches short task guidance to its server instructions, which the DSH MCP bridge adds to model context. `observe_ui` uses a bounded, labelled `UIv1` snapshot with compact rendering by default. The current data source is **window inventory only**; no accessibility control tree is falsely implied. See `docs/deepseek-ui-contract.md`.

## Input preflight boundary (V0.1.2)

`native-mcp/src/input_validation.rs` is a side-effect-free, unit-tested validator. It converts a batch of deserialized actions into typed, prepared Enigo actions before creating an input engine. It rejects invalid later steps before earlier steps can produce side effects. Pointer coordinates are checked against current monitor rectangles using overflow-safe 64-bit arithmetic; the current desktop lock covers validation and execution within one Rust process. An OS/device error **after** preflight can still produce partial execution, and external window focus changes are not prevented. This is not a focus guard, per-app permission system or an atomic transaction.
