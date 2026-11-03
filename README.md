# DeepSeek Harness Computer Use: TypeScript adapter + Rust MCP

> Status: **V0.1 development scaffold**. Not production-ready. The new Windows UIA version has not been integration-tested on an interactive desktop in the authoring environment (no local Rust toolchain or GUI). GitHub Actions verifies compilation and smoke tests. No official release or signed installer is included; successful CI runs upload an unsigned Windows debug executable artifact.

## Architecture and separation of concerns

```
DeepSeek Harness (Cordis)
  └── plugin/                   TypeScript ONLY: reserve provider and mount DSH's MCP client
        │                       No native bindings, FFI, screenshot processing, or Rust compilation
        └── MCP stdio
              └── native-mcp/   Rust standalone executable (RMCP)
                    ├── xcap   display / window enumeration and screenshot
                    └── enigo  mouse / keyboard
```

The TS plugin is a tiny connection layer; it deliberately reuses DSH's existing `@deepseek-ai/dsh-mcp-client` instead of reimplementing MCP. The Rust executable is separately built and released.

## What V0.1 currently implements in source

| MCP tool | Behavior |
| --- | --- |
| `health` | Reports active opt-in switches |
| `list_displays` | Monitor indices, bounds and primary flag |
| `list_windows` | Window titles and bounds (enumeration indexes are **not** stable handles) |
| `observe_ui` | DeepSeek-facing `UIv1` compact window inventory (or JSON); bounded snapshot IDs, explicit `windows_only` coverage and pixel coordinates |
| `screenshot` | Entire monitor / monitor-local region, JPEG image and coordinate metadata |
| `act` | Batch up to 32 mouse/keyboard actions with optional screenshot-after |

Input actions: `move`, `click`, `text`, `key`, `hotkey`, `scroll`. All actions are ordered and fail fast; already delivered input is **not** rolled back. A successful delivery is not proof of the intended UI result.

**Windows experimental V0.2:** `observe_ui` attempts a bounded UI Automation scan of the foreground window; `resolve_element` and `act_element` support exact-name lookup and guarded click/focus with expiring snapshot tokens. `act` optionally accepts a foreground snapshot guard. Screenshots support quality/resize options and report pixel-to-global coordinate scale. UIA failure returns the explicit window-only fallback; the platform has not been end-to-end validated on a real interactive desktop. CI and the unsigned Windows debug artifact are provided for iterative testing. **Not implemented:** Linux AT-SPI control enumeration, per-app consent UI, robust foreground policy for unguarded raw actions, invoke/value patterns, automatic semantic verification, image diffing, browser CDP, signed installer and measured performance.

## Build the Rust MCP server

Prerequisites: a desktop session; current stable Rust toolchain; Windows first. Linux may require native xcap/enigo packages and a supported X11/Wayland session. Headless servers cannot operate a desktop unless a display session is available.

```bash
cd native-mcp
cargo build --release
```

Binary:

- Windows: `native-mcp/target/release/dsh-native-cua.exe`
- Linux/macOS: `native-mcp/target/release/dsh-native-cua`

The program speaks MCP JSON-RPC over stdio. Do not write debug text to stdout.

## Build TypeScript connector

1. Check `dsh --version`, and align `@deepseek-ai/*` dependencies to the installed DSH version. The `package.json` versions are **examples**, not a guarantee of compatibility.
2. From `plugin/`, install dependencies in a network-enabled environment and run `npm run build`.
3. Add a Cordis profile overlay as shown in `examples/cordis.patch.yml`, replacing the paths with actual absolute paths and enabling any desired permissions.
4. Start DSH using the profile overlay, after disabling any conflicting Computer Use provider.

The plugin registers an exclusive Computer Use provider and starts the Rust MCP server through the existing official client, which registers `mcp__native_cua__*` tools. DeepSeek receives the Rust MCP server's short usage instructions via DSH's normal MCP bridge; TS remains a thin connection layer. `observe_ui` currently reports windows only, **not** button/input control trees. Screenshots require an image-capable model route and DSH attachment support.

## Security and consent

- Both capture and input **default to disabled** (`allowCapture=false`, `allowInput=false`). Enabling them is an explicit local decision, not a grant for arbitrary applications.
- Configure DSH's tool approval policy before giving an agent access to desktop tools. This V0.1 scaffold does **not** implement per-application consent and should only be used in a controlled desktop/testing environment.
- Never allow the agent to run privileged/system-sensitive actions unattended. Stop the agent before password entry, financial transactions, account changes, or deletion.
- `stdio` is local; no network listener is exposed by the Rust server. Do not enable remote desktop MCP without authentication and authorization.
- Desktop state is shared between agents and the human. Serialize complete observe-act-verify workflows at the caller level, not merely individual MCP calls.

## Engineering rules

1. Always prefer native element interaction when an accessibility backend exists; do not invent element tokens now.
2. `screenshot` returns monitor origin and region offset; convert to global physical coordinates before calling `act`.
3. Batch only safe deterministic sequences; obtain a fresh observation after uncertain UI transitions.
4. Verify the *outcome*, not just an input event, before reporting completion.
5. Benchmark on real target platforms. Do not claim 99% accuracy, <100 ms P95 or <10 MB runtime memory without data.

Further design: `docs/architecture.md`, `docs/deepseek-ui-contract.md`, `docs/mcp-contract.md`, `docs/roadmap.md`.

## Upstream references

- DSH first plugin: https://github.com/deepseek-ai/deepseek-harness/blob/master/docs/user/develop/basic/index.md
- DSH Computer Use provider: https://github.com/deepseek-ai/deepseek-harness/blob/master/packages/experimental/computer-use-cua-driver-mcp/src/index.ts
- DSH MCP client: https://github.com/deepseek-ai/deepseek-harness/blob/master/packages/mcp/mcp-client/README.md
- Rust RMCP SDK: https://github.com/modelcontextprotocol/rust-sdk
- xcap: https://docs.rs/xcap/
- enigo: https://docs.rs/enigo/

## Input validation (V0.1.2 source)

`act` now performs **whole-batch preflight** before sending any input. Click/move coordinates must fall within a currently connected display in global physical pixels, including negative origins. The validator rejects invalid buttons, hotkeys, keys, text length, scroll amount and oversized batches, so an invalid late action does not execute earlier actions. Keyboard-only requests do not require a display lookup.

This does **not** guarantee transactionality once execution starts: OS injection errors, application focus changes or UI transitions can still result in partial execution. It also does **not** enforce a foreground-window policy, app allowlist or per-application consent. Re-observe before retries; see `docs/mcp-contract.md`.

## Windows UIA experimental V0.2

The Windows backend now attempts a **bounded foreground-window UI Automation observation** in `observe_ui`. When successful, the response includes actual controls and snapshot-scoped `element_token` values. `act_element` supports guarded `click` and `focus`; each operation checks token age, foreground HWND, process, runtime identity and live bounds. Password fields are not actionable. On UIA failure, the server falls back to the explicit `windows_only` observation. Capture/input remain opt-in.

See [Windows interactive debugging instructions](docs/windows-debugging.md). This is still an **experimental Windows implementation**, not a production-ready unattended automation system. There is no signed installer, per-app consent UI, CDP backend, automatic postcondition verification or Linux AT-SPI element backend yet. CI compilation and real-desktop integration are distinct checks; a successful compile alone does not prove real desktop operation.
