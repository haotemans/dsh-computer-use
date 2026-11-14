/**
 * Independent DSH Computer Use provider.
 *
 * One native Rust MCP server owns desktop observation and input. DSH owns the
 * session, tool policies, MCP transport and image projection. No Kimi runtime
 * or executable is used: Kimi's observe-act-verify pattern is design input only.
 */
import type { Context, Fiber } from '@deepseek-ai/cordis'
import z from '@deepseek-ai/schemastery'
import { brandString, type Branded } from '@deepseek-ai/dsh-brand'
import * as McpClient from '@deepseek-ai/dsh-mcp-client'
import type {} from '@deepseek-ai/dsh-computer-use'
import { checkToolPolicy } from './policy.js'

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
    // The current DSH service allows one Computer Use provider per Host.
    yield ctx.computerUse.register(
      brandString<Branded<'ComputerUseProviderName'>>('native-cua-rust-mcp'),
    )
    // DSH's monotonic guard blocks denied tools even when an Agent asks
    // another executor (including PTC) to call them. The Rust process also
    // checks capture/input independently at the operation boundary.
    yield ctx.tools.guard((execution) =>
      checkToolPolicy(execution.name, config.allowCapture, config.allowInput),
    )
    child = ctx.plugin(McpClient, childConfig)
    yield child.dispose
  }, 'computer-use-rust-mcp.connection')

  await child.await()
}
