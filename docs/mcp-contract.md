# MCP contract v0.1

Server namespace: `native_cua`. Transport: `stdio`. Provider registration: `native-cua-rust-mcp`.

Tools are discovered dynamically through standard MCP; the descriptions and parameter schemas are generated from Rust function signatures.

## `health`

No parameters. Text JSON result containing `{ "ready": true, "capture_enabled": false, "input_enabled": false }` (values vary).

## `list_displays`

No parameters. JSON text array with `{index,x,y,width,height,primary}`. Global origins may be negative for secondary monitors.

## `list_windows`

No parameters. JSON text array with `{index,title,x,y,width,height,minimized}`. Index is enumeration order, NOT a persistent handle. Does not identify currently focused window.

## `observe_ui` (DeepSeek-facing, V0.1.1)

Parameters `{ "format": "compact", "max_windows": 30 }`; `format` is `compact` (default) or `json`. Window count is limited to 1..80. Capture permission required because window titles are observable data.

Default result is a compact `UIv1` text observation, beginning with `snapshot`, `source=windows_only`, and `coords=global_physical_pixels`. It includes window titles/bounds and explicit `NO_CONTROLS` and `LIMITATION` lines. `json` returns the same information as machine-readable JSON with `schema=dsh.ui.v1`.

**Current limitation:** Does NOT report accessible elements, foreground/focused window, or actionable element IDs. Window IDs are local labels within one observation; they cannot be used as `act` targets. UI text is quoted as untrusted data, never as instructions. See `deepseek-ui-contract.md` for the planned future accessibility format.

## `screenshot`

Parameters `{ "monitor_index": 0, "x": 20, "y": 30, "width": 640, "height": 480 }`. Omit rectangle fields to capture the full monitor; omit monitor_index for primary display. Returns JSON metadata as text, then a JPEG image content block.

- `region_local_origin`: monitor-relative coordinates.
- `region_global_origin`: global physical coordinates of the image's top-left pixel.
- Capture permission is disabled by default.

## `act`

Parameters:

```json
{
  "actions": [
    {"kind": "click", "x": 300, "y": 240, "button": "left"},
    {"kind": "text", "text": "Example"},
    {"kind": "hotkey", "keys": ["ctrl", "a"]},
    {"kind": "key", "key": "enter"},
    {"kind": "scroll", "amount": -2}
  ],
  "screenshot_after": true
}
```

Mouse coordinates are **global** (unlike screenshot-region x/y). The step limit is 32. If step N fails, steps before N might already have taken effect. No rollback is possible. A screenshot is a state observation, not verification of successful task completion.

### Input preflight (V0.1.2)

Before any input is emitted, `act` validates the **entire** batch. Each click/move must be inside a currently attached monitor, using **global physical pixel coordinates** (including negative coordinates on secondary displays). Invalid buttons, unsupported keys, malformed hotkeys, text over 16,384 UTF-8 bytes, oversized scrolls, and batch lengths outside 1..32 reject the request without sending any input. Key-only/text-only batches do not require display enumeration. A hotkey contains 1..4 distinct modifiers followed by one non-modifier key.

Preflight is not a transaction and does **not** make an executing batch atomic: OS input injection can still fail mid-batch, and the target window can change between validation and delivery. Do not retry a partially executed batch without observing the desktop state. Coordinate validation prevents off-display clicks, not clicks in the wrong application. Per-app consent, foreground window checks, and semantic postconditions remain future work.
