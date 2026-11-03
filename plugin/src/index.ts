/**
 * DeepSeek Harness Computer Use provider.
 * This is deliberately a THIN adapter. The Rust executable is an independent
 * MCP server; DSH's existing MCP bridge handles transport and tool projection.
 */
import type { Context, Fiber } from '@deepseek-ai/cordis'
import z from '@deepseek-ai/schemastery'
import { ComputerUseProviderName } from '@deepseek-ai/dsh-computer-use/brand'
import * as McpClient from '@deepseek-ai/dsh-mcp-client'
import type {} from '@deepseek-ai/dsh-computer-use'

export const name = 'computer-use-rust-mcp'
export const inject = ['computerUse', 'tools']

export interface Config {
  command: string
  args: string[]
  allowCapture: boolean
  allowInput: boolean
  toolCallTimeoutMs: number
}

export const Config: z<Partial<Config>, Config> = z.object({
  command: z.string().pattern(/\S/).default('dsh-native-cua'),
  args: z.array(String).default([]),
  allowCapture: z.boolean().default(false),
  allowInput: z.boolean().default(false),
  toolCallTimeoutMs: z.number().min(100).max(120_000).default(30_000),
})

export async function apply(ctx: Context, config: Config): Promise<void> {
  // The shared reservation prevents a second CUA provider from being mounted.
  // Retain the reservation until the owned MCP connection is fully disposed.
  const childConfig = McpClient.Config({
    transport: 'stdio',
    serverName: 'native_cua',
    command: config.command,
    args: config.args,
    env: {
      CUA_ALLOW_CAPTURE: config.allowCapture ? '1' : '0',
      CUA_ALLOW_INPUT: config.allowInput ? '1' : '0',
    },
    toolCallTimeoutMs: config.toolCallTimeoutMs,
    failOnStartupError: true,
  })

  let child!: Fiber
  ctx.effect(function* () {
    yield ctx.computerUse.register(ComputerUseProviderName('native-cua-rust-mcp'))
    child = ctx.plugin(McpClient, childConfig)
    yield child.dispose
  }, 'computer-use-rust-mcp.connection')

  await child.await()
}
