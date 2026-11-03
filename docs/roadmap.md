# Staged implementation plan

## V0.1 — minimal native desktop loop (source draft)

- [x] Thin DSH TypeScript provider + MCP connection source
- [x] Independent Rust MCP server source
- [x] Window/monitor enumeration, screenshot, action batch source
- [x] Disabled-by-default capture/input opt-ins
- [x] DeepSeek-facing `observe_ui` compact/JSON window inventory, bounded snapshot format, MCP server instructions (source draft; not compiled)
- [x] Synthetic Rust formatter unit tests written (not executed)
- [ ] Compile on supported operating systems
- [ ] Test with a real DSH runtime (start, tool discovery, call, unload, reconnect)
- [ ] Add CI and signed release artifacts

## V0.2 — accurate element targeting

- [ ] Windows UI Automation backend (element tree, stable attributes, bounding rectangles)
- [ ] Fill `observe_ui.elements` with real accessible controls, include focused window, enforce snapshot/element freshness
- [ ] `observe_window`, `resolve_element`, `act_element` tools with snapshot generation checks
- [ ] Foreground-window guard and per-application allowlist
- [ ] OS-permission diagnostics and consent UI
- [ ] Error taxonomy (unavailable/stale/not_visible/access_denied/partial_execution)

## V0.3 — performance

- [ ] Image crop/scale/diff and change detection with coordinate metadata
- [ ] Native postconditions and bounded retries
- [ ] Batching policies with stop conditions
- [ ] Browser-specific backend/CDP if justified
- [ ] Cross-platform implementations and contract tests

## Benchmarks

Test on real machines, not in the current build environment: latency to list UI elements, screenshot 1080p/window/region, encode JPEG, click+type, batch+verify; P50/P95/P99 CPU and RSS, correct target rate under DPI/multi-monitor/window movement.

## V0.1.2 — input safety preflight

- [x] Side-effect-free validation of entire action batch before input injection
- [x] Global physical coordinate validation across attached displays, including negative origins
- [x] Reject malformed/duplicate hotkey modifiers and unsupported keys
- [x] Unit tests for monitor bounds, overflow, invalid late steps and input limits
- [ ] Verify new Rust tests in Windows CI; real desktop targeting and DSH integration tests still pending

## Experimental V0.2 Windows implementation

- [x] Bounded foreground-window native UIA control enumeration (Windows only)
- [x] Snapshot-local element tokens, 10-second expiry and re-resolution
- [x] `resolve_element` exact, unique name/role matching
- [x] `act_element` click/focus with freshness, identity and foreground checks
- [x] Optional snapshot guard for raw `act`
- [x] JPEG quality/max-dimension configuration with explicit coordinate scaling
- [x] MCP permission and tool-catalog smoke tests (execution depends on CI)
- [ ] Real interactive Windows desktop testing and application-specific fixes
- [ ] Per-app consent GUI, process allowlist and secure foreground transitions
- [ ] Invoke/Value accessibility patterns for fully semantic editing
- [ ] Native postconditions and screenshot change/diff
- [ ] Linux AT-SPI and cross-platform accessibility parity
- [ ] Signed installer and official release process
