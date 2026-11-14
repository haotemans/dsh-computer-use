import { test } from 'node:test'
import assert from 'node:assert/strict'
import { checkToolPolicy } from '../dist/policy.js'

test('unrelated tool names pass without interference', () => {
  assert.equal(checkToolPolicy('mcp__github__fetch', 'kimi', false, false), undefined)
  assert.equal(checkToolPolicy('mcp__native_cua__act', 'kimi', false, false), undefined)
})

test('safe diagnostics are available with zero permissions', () => {
  for (const backend of ['native', 'kimi']) {
    const name = backend === 'native' ? 'native_cua' : 'kimi_cu'
    assert.equal(checkToolPolicy(`mcp__${name}__health`, backend, false, false), undefined)
  }
})

test('capture requires explicit opt-in', () => {
  assert.match(checkToolPolicy('mcp__kimi_cu__get_app_state', 'kimi', false, false), /capture is disabled/)
  assert.equal(checkToolPolicy('mcp__kimi_cu__get_app_state', 'kimi', true, false), undefined)
  assert.match(checkToolPolicy('mcp__native_cua__observe_ui', 'native', false, false), /capture is disabled/)
})

test('read-only capability cannot click or type', () => {
  assert.match(checkToolPolicy('mcp__kimi_cu__click', 'kimi', true, false), /input is disabled/)
  assert.match(checkToolPolicy('mcp__kimi_cu__type_text', 'kimi', true, false), /input is disabled/)
  assert.match(checkToolPolicy('mcp__native_cua__act', 'native', true, false), /input is disabled/)
})

test('input still requires observation permission', () => {
  assert.match(checkToolPolicy('mcp__kimi_cu__click', 'kimi', false, true), /cannot act without capture/)
})

test('all unknown capabilities fail closed even with full permissions', () => {
  assert.match(checkToolPolicy('mcp__kimi_cu__arbitrary_tool', 'kimi', true, true), /not on/)
  assert.match(checkToolPolicy('mcp__native_cua__run_shell', 'native', true, true), /not on/)
})

test('explicitly enabled allowlisted actions are callable', () => {
  assert.equal(checkToolPolicy('mcp__kimi_cu__set_value', 'kimi', true, true), undefined)
  assert.equal(checkToolPolicy('mcp__native_cua__act_element', 'native', true, true), undefined)
})
