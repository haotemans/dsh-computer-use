/**
 * Computer Use permission boundary inside DSH's model tool pipeline.
 * Native Rust rechecks environment permissions so bypassing the model-facing
 * guard cannot accidentally enable direct desktop input.
 *
 * Scope: only mcp__native_cua__*; other applications/tools are unaffected.
 * Not a substitute for per-task DSH approvals, user consent, or OS isolation.
 */

const PREFIX = 'mcp__native_cua__'

const READ_TOOLS = new Set([
  'list_displays', 'list_windows', 'observe_ui', 'get_app_state',
  'screenshot', 'resolve_element', 'check_ui_condition',
])

const WRITE_TOOLS = new Set(['act', 'act_element'])

export function checkToolPolicy(
  qualifiedName: string,
  allowCapture: boolean,
  allowInput: boolean,
): string | undefined {
  if (!qualifiedName.startsWith(PREFIX)) return undefined
  const tool = qualifiedName.slice(PREFIX.length)
  if (tool === 'health' || tool === 'doctor') return undefined

  if (READ_TOOLS.has(tool)) {
    return allowCapture ? undefined
      : `Computer Use capture is disabled (tool: ${tool}). Enable allowCapture explicitly.`
  }
  if (WRITE_TOOLS.has(tool)) {
    if (!allowCapture) return 'Computer Use input requires capture permission to verify desktop state.'
    return allowInput ? undefined
      : `Computer Use input is disabled (tool: ${tool}). Enable allowInput explicitly.`
  }
  return `Unknown Computer Use tool "${tool}" was refused by the desktop allowlist.`
}
