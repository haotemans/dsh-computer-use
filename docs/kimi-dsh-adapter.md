# Kimi Computer Use backend for DeepSeek Harness (experimental)

This is a **DSH adapter for a separately installed Kimi CU runtime**, not a fork of Kimi's desktop implementation and not a Kimi Code model integration.

## Runtime paths

- Windows x64: use a separately installed official Kimi Windows CU executable, generally `%LOCALAPPDATA%/KimiCU/kimi-cu.exe` (verify on the user's machine). Kimi's own capability probe expects an executable responding to `doctor` with `mcp=true` and `helper=embedded`.
- macOS: `/Applications/KimiCU.app/Contents/MacOS/kimi-cu`.
- For both platforms: MCP stdio transport with `args: ['mcp']`.

Kimi Code's official Windows runtime and plugin installation are **external prerequisites**. This adapter neither downloads nor installs binaries or bypasses operating system permissions. A Linux or headless host is not assumed to provide a usable Kimi desktop.

## Configure

1. Build `plugin/` using `npm install && npm run check`.
2. Copy `examples/kimi.windows.patch.yml` or `examples/kimi.macos.patch.yml` and replace every placeholder with a verified absolute path.
3. If another computer-use provider is active in the same DSH host, disable it first. Do not register the `@deepseek-ai/dsh-computer-use` service twice.
4. Start the existing DSH with `--patch <overlay>`; do not start a second Host.
5. Check that the current Agent discovers `mcp__kimi_cu__list_apps` and `mcp__kimi_cu__get_app_state`. Verify actual tool names with MCP discovery; the allowlist intentionally blocks unknown tools.
6. Enable **only capture** when the user has approved reading desktop content. Leave input disabled until the user separately approves interactive operation. Apply DSH's native ask/approval policy for impactful actions.

## Security model

The plugin installs a DSH `ctx.tools.guard` that denies missing capture or input permissions for all known Kimi tool names and refuses unknown names even when both permissions are enabled. Other MCP namespaces are unaffected. DSH native/parallel and PTC nested tool invocations use the same guard pipeline.

The guard prevents DSH model-invoked calls **only**. It does not restrict direct use of the Kimi CU executable by other programs, cannot inspect what happens inside a Kimi-provided implementation, and cannot distinguish a harmless click from submitting an irreversible transaction. DSH approval policy must require user confirmation for send, submit, purchase, delete, publish and account changes. Do not give this Agent unattended administrative access.

Both `allowCapture` and `allowInput` default to false. A process restart or config reload is required for changes to these flags. The Kimi server does not use the Rust `CUA_ALLOW_*` environment variables; enforcement is performed by DSH before dispatch.

## Faster and safer execution

- One MCP stdio child, not a Node-to-Rust-to-Kimi chain. The DSH-managed client owns tool discovery, transport, image projection, cancellation and reconnection.
- Prefer `get_app_state` and accessible element indexes to whole-screen screenshots when supported.
- Send one purposeful action per state transition, then re-observe; batch only deterministic operations where the runtime guarantees ordering.
- Do not blindly retry a timed-out click or typed input: its result is uncertain.
- Never reuse an element index across interface changes; never invent a control token.
- Host-level Computer Use registration is exclusive, but **separate DSH sessions can share the physical desktop**. Serialize complete observe-act-verify workflows in the calling Agent.
- The MCP bridge and OS integration have not yet been benchmarked against Kimi Code. **No measured speed claim is made**.

## Diagnostics

| Symptom | What to inspect |
| --- | --- |
| `ERR_PACKAGE_PATH_NOT_EXPORTED` | DSH runtime/adapter dependency mismatch; rebuild the TS plugin after updating dependencies |
| `module not found` | Verify absolute `plugin/dist/index.js` path and its runtime dependencies |
| `mcp-client(kimi_cu)` startup error | Verify `command`, `args: ['mcp']`, executable architecture and whether `kimi-cu.exe doctor` reports MCP support |
| Kimi tools absent | Check the DSH patch/profile is actually active and the initial MCP tool-list sync completed |
| Tools visible but denied | Inspect `allowCapture` / `allowInput`; denial is expected with default settings |
| Kimi tools return no desktop | Verify a real interactive desktop is active, Kimi runtime is healthy and required OS permissions were granted |
| Browser webpage access unavailable | Kimi CU controls desktop applications; it is not a browser-specific CDP integration. Use dedicated browser tools when appropriate |

No Windows interactive-desktop test can be inferred from CI compilation and stdio fixture tests. Perform a separate GUI acceptance test on an actual machine.
