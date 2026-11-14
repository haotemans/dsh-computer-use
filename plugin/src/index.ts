/**
 * Cordis Computer Use MCP provider: native Rust or an installed Kimi CU runtime.
 * A single DSH MCP bridge owns discovery and tool projection. The guard is
 * monotonic and defaults to deny: a prompt cannot grant desktop permissions.
 */
import type { Context, Fiber } from '@deepseek-ai/cordis'
import z from '@deepseek-ai/schemastery'
import { brandString, type Branded } from '@deepseek-ai/dsh-brand'
import * as McpClient from '@deepseek-ai/dsh-mcp-client'
import type {} from '@deepseek-ai/dsh-computer-use'
import { checkToolPolicy } from './policy.js'

export const name = 'computer-use-mcp'
export const inject = ['computerUse', 'tools']

export interface Config {
  backend: 'native' | 'kimi'
  command: string
  args: string[]
  allowCapture: boolean
  allowInput: boolean
  toolCallTimeoutMs: number
}

export const Config: z<Partial<Config>, Config> = z.object({
  backend: z.union([z.const('native'), z.const('kimi')]).default('native'),
  command: z.string().pattern(/\S/).default('dsh-native-cua'),
  args: z.array(String).default([]),
  allowCapture: z.boolean().default(false),
  allowInput: z.boolean().default(false),
  toolCallTimeoutMs: z.number().min(100).max(120_000).default(30_000),
})

export async function apply(ctx: Context, config: Config): Promise<void> {
  const backend = config.backend
  // Kimi requires the actual Kimi CU executable, not the Kimi Code CLI.
  // Set command to the absolute path and args: ['mcp'] in the overlay.
  const serverName = backend === 'kimi' ? 'kimi_cu' : 'native_cua'
  const childConfig = McpClient.Config({
    transport: 'stdio',
    serverName,
    command: config.command,
    args: config.args,
    env: backend === 'native'
      ? {
          CUA_ALLOW_CAPTURE: config.allowCapture ? '1' : '0',
          CUA_ALLOW_INPUT: config.allowInput ? '1' : '0',
        }
      : {},
    toolCallTimeoutMs: config.toolCallTimeoutMs,
    failOnStartupError: true,
  })

  let child!: Fiber
  ctx.effect(function* () {
    // Only one desktop provider may be active in a Host.
    yield ctx.computerUse.register(
      brandString<Branded<'ComputerUseProviderName'>>(`${serverName}-mcp`),
    )
    // All calls, including PTC nested calls, traverse the DSH tool pipeline.
    // Install the guard before tool discovery, and fail closed on unknown tools.
    yield ctx.tools.guard((execution) =>
      checkToolPolicy(execution.name, backend, config.allowCapture, config.allowInput),
    )
    child = ctx.plugin(McpClient, childConfig)
    yield child.dispose
  }, 'computer-use-mcp.connection')

  await child.await()
}
