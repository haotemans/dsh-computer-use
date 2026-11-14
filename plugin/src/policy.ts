/**
 * Fail-closed DSH tool admission. This controls the MODEL-facing tool route,
 * including calls from the PTC executor. It is not a sandbox or a replacement
 * for DSH's user approval policy or the operating system's access controls.
 */
export type ComputerBackend = 'native' | 'kimi'

const READ_TOOLS: Record<ComputerBackend, ReadonlySet<string>> = {
  native: new Set([
    'list_displays', 'list_windows', 'observe_ui', 'screenshot',
    'resolve_element', 'check_ui_condition',
  ]),
  kimi: new Set([
    'list_apps', 'get_app_state', 'screenshot', 'list_windows',
    'get_window_state', 'get_app_list',
  ]),
}

const WRITE_TOOLS: Record<ComputerBackend, ReadonlySet<string>> = {
  native: new Set(['act', 'act_element']),
  kimi: new Set([
    'click', 'type_text', 'press_key', 'scroll', 'set_value',
    'perform_secondary_action', 'select_text', 'drag', 'open_app',
    'activate_app', 'focus', 'hotkey', 'move', 'mouse_click',
  ]),
}

export function checkToolPolicy(
  qualifiedName: string,
  backend: ComputerBackend,
  allowCapture: boolean,
  allowInput: boolean,
): string | undefined {
  const namespace = backend === 'kimi' ? 'mcp__kimi_cu__' : 'mcp__native_cua__'
  if (!qualifiedName.startsWith(namespace)) return undefined

  const tool = qualifiedName.slice(namespace.length)
  if (tool === 'health' || tool === 'doctor') return undefined
  if (READ_TOOLS[backend].has(tool)) {
    return allowCapture ? undefined : `Computer Use capture is disabled (tool: ${tool}). Enable allowCapture explicitly.`
  }
  if (WRITE_TOOLS[backend].has(tool)) {
    if (!allowCapture) return 'Computer Use cannot act without capture permission and fresh state verification.'
    return allowInput ? undefined : `Computer Use input is disabled (tool: ${tool}). Enable allowInput explicitly.`
  }
  return `Computer Use tool "${tool}" is not on the ${backend} allowlist. Refusing unknown desktop capability.`
}
