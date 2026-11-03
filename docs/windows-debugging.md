# Windows UI Automation debugging round (experimental V0.2)

This branch is for **interactive Windows debugging**. It builds on PR #2's
whole-batch preflight. UIA may be incomplete in custom-rendered applications;
the fallback is window-only observation plus explicit screenshots.

## Setup

1. Use an interactive Windows desktop session. Enable Windows UI Automation.
2. Build native server: `cd native-mcp && cargo build --release`.
3. Build TS adapter: `cd plugin && npm install && npm run build`.
4. Set the native executable absolute path in `examples/cordis.patch.yml`.
   Set `allowCapture: true`; set `allowInput: true` **only while testing**.
5. Start DSH with the Cordis overlay and only one registered computer-use provider.
6. Try `health`, `observe_ui(format="compact")`, `screenshot`,
   `act_element`, `observe_ui` in that order. Check the observed state after
   every operation; an input-delivery result is not task completion.

## API

- `observe_ui(format="compact", max_windows=30)`: on Windows, supplements
  window inventory with up to 120 controls in the current foreground window.
  Coverage is `accessibility_foreground` when successful, or
  `windows_only` on fallback. Tree depth is capped at six.
- `act_element(snapshot_id="sN", element_token="esN:I", action="click")`:
  requires capture/input permission, the latest snapshot, and a target not
  older than ten seconds. Supports `click` and `focus`. Re-resolves live UIA
  runtime ID and checks process, foreground window, name, role, and bounding
  rectangle before delivering an action. It refuses password/sensitive elements
  and rejects stale/hidden/disabled controls.
- `act(actions=[...])`: original global-pixel low-level input, with full
  batch preflight and attached-monitor bounds validation.
- `screenshot`: JPEG full monitor / local region, with global offset metadata.
  There is no native visual-diff or browser/CDP backend in this branch.

### Example debug session (not a scripted unattended macro)

```json
{"name":"observe_ui","arguments":{"format":"json","max_windows":12}}
```

Read `snapshot_id`, `coverage` and a real `element_token` from the response;
then, only if `coverage=accessibility_foreground`:

```json
{"name":"act_element","arguments":{"snapshot_id":"<from observation>","element_token":"<from observation>","action":"focus"}}
```

The placeholders above must be substituted with the **same** observation's
values, not invented. A new observation invalidates prior tokens.

## Limitations / troubleshooting

- Windows UIA providers may be slow, broken, sparse or unavailable. If
  `observe_ui` reports `windows_only`, use explicit screenshots. Never
  infer buttons from titles.
- Some applications need their own accessibility implementation. UIA does
  not automatically expose a complete DOM or game-engine UI tree.
- The initial element action path provides `click`/`focus`; it does not
  implement typed value-pattern edits, drag-and-drop, drop-down selection,
  automatic screenshot diff or native task-success assertions.
- The element scanner works only on the foreground window. It does not
  grant per-process consent or prevent another process from changing focus
  outside the tool's critical section.
- Read-only observation is still sensitive: labels and window titles may
  contain personal information. Capture and input remain disabled by default.
- No native user-consent window, installer, signed release or unattended
  automation approval is provided. Windows secure desktop and elevation
  prompts are outside this integration.
- Element click dispatch can fail after validation; treat the result as input
  delivery only. Re-observe before retrying.

## Manual acceptance matrix

| Case | Expected |
| --- | --- |
| No capture permission | `observe_ui` denied |
| No input permission | `act` and `act_element` denied |
| Notepad / native dialog | UIA roles/names where provider exposes them |
| Custom-rendered app | Partial or window-only observation, no fabricated elements |
| Two monitors with a negative origin | Global coordinate preflight accepts valid points |
| Change active app between observation/action | `STALE` |
| Re-observe before acting on old token | `STALE` |
| Wait more than ten seconds after observing | `STALE` |
| Disable or move target after observing | `STALE` / `NOT_VISIBLE` |
| Password field | No value exposed; `act_element` refused |
| Invalid second action of a batch | No input delivered by the entire batch |
| Successful click/focus | Delivery reported, then a new observation required |

Report the Windows version, DSH version, command output, tool arguments,
actual tool responses (with secrets removed), and screenshots if safe.

## Additional debug parameters

To save image bandwidth, call `screenshot(max_dimension=1600, quality=70)`.
The metadata includes `region_global_origin` and `pixel_to_global_scale`.
Convert image pixel `(x,y)` to physical desktop coordinates with
`global_x = region_global_origin[0] + x * pixel_to_global_scale[0]` and likewise for y.

To resolve a uniquely named UI control without manually picking a token, call
`resolve_element(snapshot_id="sN", name="Save", role="button")` after observing.
Do not guess IDs. Raw `act` can accept the same `snapshot_id` as a foreground guard;
it invalidates the observation after execution.

## Native UI condition checks

After `act_element` or `act`, call `check_ui_condition` with an exact
accessible name and role to inspect the current foreground window:

```json
{"name":"check_ui_condition","arguments":{"name":"Save","role":"button","expected_present":true}}
```

Read `verdict`, `matches`, `truncated` and the scope. `inconclusive`
means the accessibility provider did not supply sufficient evidence. A
satisfied control-presence check does not prove an application saved a file.

## Windows executable from CI

After a successful PR GitHub Actions run, open its **Artifacts** section and
retrieve `dsh-native-cua-windows-debug`. It is an **unsigned debug binary**
for developer testing only. Keep the TypeScript adapter installation and
Cordis overlay configuration alongside the native binary. No signed installer
or automatic app permission grant is included.
