import { test } from 'node:test'
import assert from 'node:assert/strict'
import { checkToolPolicy } from '../dist/policy.js'

test('unrelated MCP tools remain unaffected', () => {
  assert.equal(checkToolPolicy('mcp__github__fetch', false, false), undefined)
  assert.equal(checkToolPolicy('mcp__other_cua__click', true, true), undefined)
})

test('read-only diagnostics require no desktop permissions', () => {
  assert.equal(checkToolPolicy('mcp__native_cua__health', false, false), undefined)
  assert.equal(checkToolPolicy('mcp__native_cua__doctor', false, false), undefined)
})

test('observation requires capture opt-in', () => {
  for (const tool of ['observe_ui', 'get_app_state', 'screenshot', 'resolve_element']) {
    assert.match(checkToolPolicy(`mcp__native_cua__${tool}`, false, false), /capture is disabled/)
    assert.equal(checkToolPolicy(`mcp__native_cua__${tool}`, true, false), undefined)
  }
})

test('no input when input permission was not granted', () => {
  for (const tool of ['act', 'act_element']) {
    assert.match(checkToolPolicy(`mcp__native_cua__${tool}`, true, false), /input is disabled/)
    assert.match(checkToolPolicy(`mcp__native_cua__${tool}`, false, true), /requires capture/)
    assert.equal(checkToolPolicy(`mcp__native_cua__${tool}`, true, true), undefined)
  }
})

test('unknown native desktop capabilities fail closed', () => {
  assert.match(checkToolPolicy('mcp__native_cua__run_shell', true, true), /refused/)
  assert.match(checkToolPolicy('mcp__native_cua__arbitrary_tool', true, true), /refused/)
})

test('Kimi CU namespace is not claimed by our independent plugin', () => {
  assert.equal(checkToolPolicy('mcp__kimi_cu__click', false, false), undefined)
})
