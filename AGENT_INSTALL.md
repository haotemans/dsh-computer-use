# AGENT_INSTALL.md — DSH Computer Use Agent 自助安装手册

> 适用：Windows 10/11、PowerShell、交互式 Windows 桌面，DSH (DeepSeek Harness) + 本仓库的 V0.2 实验性 Rust MCP 插件。
>
> 仓库：https://github.com/haotemans/dsh-computer-use
> 当前调试分支：feat/windows-uia-debug-round（PR #3）。
>
> **给执行 Agent 的指令：** 用户已委托你安装和接入插件。依次执行检查、安装/构建、生成独立配置、启动或告知重启方式、验证。不要仅输出命令就声称安装完成。遇到不可逆修改、管理员权限、账号认证或开启桌面输入授权时，必须取得用户明确同意。

## 0. 目标与约束（必须遵守）

安装后，DeepSeek Harness 能通过 TypeScript/Cordis 适配器连接 Rust stdio MCP，并发现以下工具：

- health、list_displays、list_windows
- observe_ui、screenshot
- act、resolve_element、act_element、check_ui_condition

标准 MCP 名称前缀为 mcp__native_cua__。

**所有本插件的文件统一放在一个目录：**

- 默认目录：%LOCALAPPDATA%/DSH/computer-use（Windows 当前用户目录）。
- 源码：同一目录下的 plugin/、native-mcp/ 等。
- JS 输出：plugin/dist/index.js。
- Rust 输出：native-mcp/target/release/dsh-native-cua.exe。
- 专用 DSH overlay：cordis.agent.yml。
- 不要把程序散装在 Program Files、System32、桌面、临时目录等位置。
- DSH 本体若已安装在其他位置，**不要为了本插件移动或重装 DSH**。

安全和幂等性：

1. 初次安装时将 allowCapture=false、allowInput=false。用户明确同意读取桌面后才可开启 capture；用户明确同意注入鼠标/键盘后才可开启 input。
2. 不修改 PowerShell 执行策略，不关闭 Defender/UAC，不设置全局代理，不要求 API Key，不记录任何密码、Token、屏幕敏感信息。
3. 不修改已有 DSH 主配置；只创建本插件的独立 patch 文件。若发现已有 Computer Use provider 或同名注册，必须先检查并避免重复注册，不得盲目覆盖。
4. 不覆写一个已有且非本仓库的安装目录；遇到已安装实例先核对 Git 仓库、分支、现有 overlay，再决定继续。
5. 不自动合并 PR、提交/推送代码或操作用户的其他项目。安装中断时保留现有用户配置。
6. Rust CLI 通过 stdio 通信。不要用 print/echo 等输出污染 MCP 子进程的 stdout。
7. Windows 原生 UI Automation 是实验性功能；CI 编译通过 ≠ 真正完成真实桌面集成测试。

## 1. 环境探测

用 PowerShell（非管理员权限即可）运行：

~~~powershell
$ErrorActionPreference = 'Stop'
$Root = Join-Path $env:LOCALAPPDATA 'DSH/computer-use'
$Repo = 'https://github.com/haotemans/dsh-computer-use.git'
$Branch = 'feat/windows-uia-debug-round'
$Overlay = Join-Path $Root 'cordis.agent.yml'
$PluginJs = Join-Path $Root 'plugin/dist/index.js'
$ServerExe = Join-Path $Root 'native-mcp/target/release/dsh-native-cua.exe'

Write-Host "Install directory: $Root"
Write-Host "OS: $([Environment]::OSVersion.VersionString)"

foreach ($command in @('git', 'node', 'npm.cmd', 'cargo')) {
  $resolved = Get-Command $command -ErrorAction SilentlyContinue
  if ($null -eq $resolved) {
    Write-Warning "Missing prerequisite: $command"
  } else {
    Write-Host "$command : $($resolved.Source)"
  }
}

Get-Command npx.cmd -ErrorAction SilentlyContinue | Select-Object Name, Source
Get-Command pnpm.cmd -ErrorAction SilentlyContinue | Select-Object Name, Source
~~~

判断：

- 缺少 Git / Node.js / npm：通过官方发行渠道或机器已有的包管理器安装，重新打开终端确认命令可用。不得从不明镜像下载。
- 缺少 Rust / Cargo：优先询问用户是否允许安装官方 Rust 工具链；不允许时，可改用本仓库可信 GitHub Actions 成功运行产生的 Windows debug artifact，但须核对对应提交和测试状态（见第 3 节）。
- 如果用户已运行 DSH，先确认其启动方式（npm/npx 还是源码构建），以及当前是否有其它 Computer Use Provider。
- DSH 是 developer preview，可能发生破坏性 API 变更。遇到依赖版本不匹配，先记录精确版本/编译报错再适配，禁止胡乱覆盖 DSH 内部包。
- PowerShell 若把 npm.ps1 拦截，使用 npm.cmd、npx.cmd 和 pnpm.cmd，而不是修改执行策略。

**Agent 必须保留探测结果，以供最终报告。**

## 2. 获取项目（不覆盖已有安装）

~~~powershell
$ErrorActionPreference = 'Stop'
$Root = Join-Path $env:LOCALAPPDATA 'DSH/computer-use'
$Repo = 'https://github.com/haotemans/dsh-computer-use.git'
$Branch = 'feat/windows-uia-debug-round'

if (-not (Test-Path $Root)) {
    git clone --single-branch --branch $Branch $Repo $Root
    if ($LASTEXITCODE -ne 0) { throw 'Git clone failed.' }
} elseif (Test-Path (Join-Path $Root '.git')) {
    $origin = (git -C $Root remote get-url origin).Trim()
    if ($origin -notmatch 'haotemans/dsh-computer-use(\.git)?$') {
        throw "Existing directory points to a different Git repository: $origin"
    }
    $current = (git -C $Root branch --show-current).Trim()
    if ($current -ne $Branch) {
        throw "Existing checkout is on $current, not $Branch. Do not switch with uncommitted changes."
    }
    Write-Host 'Reusing existing plugin checkout; not overwriting user modifications.'
} else {
    throw "Directory already exists and is not a Git checkout: $Root"
}

git -C $Root rev-parse HEAD
~~~

默认复用已存在的有效检出。**不要自动 git reset --hard、git clean -fd 或强制 checkout。**
如确需更新，先检查工作区差异、备份用户改动，再在获得授权后采用 fast-forward 更新。

## 3. 编译两个组件

### A. TypeScript / Cordis 插件

~~~powershell
$Root = Join-Path $env:LOCALAPPDATA 'DSH/computer-use'
Push-Location (Join-Path $Root 'plugin')
try {
    if (Test-Path 'package-lock.json') {
        npm.cmd ci --no-audit --no-fund
    } else {
        npm.cmd install --no-audit --no-fund
    }
    if ($LASTEXITCODE -ne 0) { throw 'npm dependency installation failed.' }

    npm.cmd run build
    if ($LASTEXITCODE -ne 0) { throw 'TypeScript build failed.' }
} finally {
    Pop-Location
}
$PluginJs = Join-Path $Root 'plugin/dist/index.js'
if (-not (Test-Path $PluginJs -PathType Leaf)) {
    throw "TS build output missing: $PluginJs"
}
Write-Host "TypeScript plugin ready: $PluginJs"
~~~

### B. Rust MCP 原生程序（推荐在本机编译）

~~~powershell
$Root = Join-Path $env:LOCALAPPDATA 'DSH/computer-use'
Push-Location (Join-Path $Root 'native-mcp')
try {
    cargo test
    if ($LASTEXITCODE -ne 0) { throw 'Rust unit tests failed.' }

    cargo build --release
    if ($LASTEXITCODE -ne 0) { throw 'Rust release build failed.' }
} finally {
    Pop-Location
}
$ServerExe = Join-Path $Root 'native-mcp/target/release/dsh-native-cua.exe'
if (-not (Test-Path $ServerExe -PathType Leaf)) {
    throw "Native MCP executable missing: $ServerExe"
}
Write-Host "Rust MCP ready: $ServerExe"
~~~

若 Cargo 暂不可用，允许从该仓库经过成功 CI 的 GitHub Actions Artifacts 下载 dsh-native-cua-windows-debug；**它是未签名调试版本，不是正式发行版**。将其解压到同一安装根目录的 native-mcp/bin/，然后把下文的 command 改为该位置。不要自动挑选失败或不明来源的二进制文件。

CI 入口：https://github.com/haotemans/dsh-computer-use/actions

可选的无桌面权限 MCP smoke test（需要 Python）：

~~~powershell
$Root = Join-Path $env:LOCALAPPDATA 'DSH/computer-use'
$ServerExe = Join-Path $Root 'native-mcp/target/release/dsh-native-cua.exe'
$SmokeTest = Join-Path $Root 'native-mcp/tests/smoke_mcp.py'
if (Get-Command py -ErrorAction SilentlyContinue) {
    py -3 $SmokeTest $ServerExe
    if ($LASTEXITCODE -ne 0) { throw 'MCP smoke test failed.' }
} else {
    Write-Host 'Python launcher unavailable; skip optional local MCP stdio smoke test.'
}
~~~

## 4. 生成 DSH Cordis patch（只创建独立文件）

**先核对 DSH 配置是否已经启用 @deepseek-ai/dsh-computer-use 注册服务和其他 Computer Use Provider。**
如果已存在共享注册服务，不能重复注册；应根据当前 DSH 版本调整本 patch 的条目。若无法确认，停止修改并把冲突情况报告用户。

在没有冲突的标准配置下：

~~~powershell
$ErrorActionPreference = 'Stop'
$Root = Join-Path $env:LOCALAPPDATA 'DSH/computer-use'
$PluginJs = (Join-Path $Root 'plugin/dist/index.js').Replace('\', '/')
$ServerExe = (Join-Path $Root 'native-mcp/target/release/dsh-native-cua.exe').Replace('\', '/')
$Overlay = Join-Path $Root 'cordis.agent.yml'

if (-not (Test-Path $PluginJs -PathType Leaf)) { throw 'Missing TypeScript plugin.' }
if (-not (Test-Path $ServerExe -PathType Leaf)) { throw 'Missing Rust MCP executable.' }

# YAML quoted strings; double a single quote in any filesystem path.
$PluginPathYaml = $PluginJs.Replace("'", "''")
$ServerPathYaml = $ServerExe.Replace("'", "''")

$yaml = @"
- insert:
    - id: computer-use-registry
      name: '@deepseek-ai/dsh-computer-use'
    - id: rust-native-cua
      name: '$PluginPathYaml'
      config:
        command: '$ServerPathYaml'
        args: []
        allowCapture: false
        allowInput: false
        toolCallTimeoutMs: 30000
"@

if (Test-Path $Overlay) {
    Write-Host "Existing overlay preserved: $Overlay"
    Get-Content $Overlay
} else {
    Set-Content -Path $Overlay -Value $yaml -Encoding utf8
    Write-Host "Created overlay: $Overlay"
}
~~~

**不要把 allowInput 默认改成 true。** 读取桌面窗口标题和截图也可能含有私人信息，因此 allowCapture 同样默认 false。用户明确同意读取桌面时才可将 allowCapture 改为 true；鼠标键盘操作另外申请允许后才可将 allowInput 改为 true。

如果从预编译 GitHub artifact 使用 native-mcp/bin/ 下的 EXE，修改该 patch 的 command 绝对路径即可，不要把二进制移到其他随机目录。

## 5. 接入并启动 DSH（选择用户已有的运行方式）

**不要重装已经运行正常的 DSH。** 先记录原有启动命令、现有端口与 Provider，必要时让用户确认可以重启。

### npm / npx 安装的 DSH

~~~powershell
$Root = Join-Path $env:LOCALAPPDATA 'DSH/computer-use'
$Overlay = Join-Path $Root 'cordis.agent.yml'
npx.cmd @deepseek-ai/dsh web --patch $Overlay
~~~

### 源码方式运行的 DSH

在用户已有的 deepseek-harness 仓库根目录执行（无需复制本插件到 DSH 源码目录）：

~~~powershell
$Root = Join-Path $env:LOCALAPPDATA 'DSH/computer-use'
$Overlay = Join-Path $Root 'cordis.agent.yml'
pnpm.cmd dsh web --patch $Overlay
~~~

标准 Web UI 地址：http://127.0.0.1:3080

- 只使用上述 **一种** DSH 启动方式；不要同时启动第二个 DSH 占用同一端口。
- Agent 若不能控制用户的现有 DSH 会话，应只生成配置、给出准确重启命令，并把结果标记为“等待用户重启验证”；绝不能声称插件已加载。
- 插件通过绝对 JS 文件路径加载；MCP 子进程使用独立 EXE，stdio 连接。
- 若 DSH 启动时报 Computer Use Provider 冲突，排查现有配置并让用户决定保留哪个 Provider。不得未经允许卸载其它插件。

## 6. 验收要求（不许伪报成功）

按顺序验证：

1. 文件检查：plugin/dist/index.js、native-mcp/target/release/dsh-native-cua.exe、cordis.agent.yml 都存在，或者记录使用的实际 artifact 路径。
2. 构建检查：npm.cmd run build 与 cargo test/build 返回 exit code 0；如使用 artifact，记录来源 CI run、提交 SHA、可执行文件实际路径。
3. DSH 启动：带 --patch 的 DSH 正常启动，没有插件解析错误、重复 Provider 或 MCP 子进程启动失败。
4. 工具发现：在 DSH 工具列表找到 mcp__native_cua__health 与 mcp__native_cua__observe_ui 等 MCP 工具。
5. 默认权限：health 显示 capture_enabled=false、input_enabled=false；无许可时 observe_ui 和 act 均被拒绝。
6. **只在用户同意读取桌面后**，开启 allowCapture=true 并重启 DSH；检查 observe_ui。Windows UIA 可能是 accessibility_foreground，也可能降级为 windows_only，后者不代表安装失败。
7. **只在用户另外同意输入操作后**，在可丢弃测试窗口中启用 allowInput=true，执行非破坏性 click/focus 测试；操作后重新 observe_ui 或 check_ui_condition。不得在真实账号、密码框、支付、文件删除或管理设置上自动测试。

如果无法通过第 3、4、5 步，不得说“安装完成”；应报告构建完成但集成验证失败/待验证。

## 7. 故障恢复

| 表现 | Agent 应执行 |
| --- | --- |
| npm.ps1 被 PowerShell 拦截 | 改用 npm.cmd / npx.cmd / pnpm.cmd；不修改 ExecutionPolicy |
| npm 安装/TS 编译错误 | 记录 DSH 版本、Node 版本、npm 日志及依赖冲突；只修改本插件兼容性 |
| cargo / Rust 编译失败 | 记录 rustc、cargo、错误日志；尝试本仓库受信任且 CI 成功的 artifact |
| DSH 显示 module not found | 核对绝对路径、plugin/dist/index.js 及 plugin/node_modules |
| MCP process failed | 核对 EXE 是否存在、是否与 Windows 架构匹配、是否具有运行权限 |
| computerUse provider already registered | 检查现有 DSH profile/overlay；不自动删除其它 Provider |
| observe_ui 报权限拒绝 | 预期默认行为；只有得到同意才能开启 allowCapture |
| act 报权限拒绝 | 预期默认行为；只有得到同意才能开启 allowInput |
| UIA 控件无法读取 | 确认交互式桌面、前台应用和权限；用 windows_only 或截图回退，不虚构控件 |
| snapshot stale | 重新 observe_ui，获取新 snapshot_id 与 element_token |
| UI 操作执行但目标没变化 | 重新观察并报告真实结果；不要重复盲点、不要报告虚假成功 |

## 8. 回滚

- 停止通过本 patch 启动的 DSH 进程，再用用户原来的启动命令（不带此 patch）启动 DSH。
- 不修改其他 Provider、DSH 系统配置、账号、全局系统设置。
- 若用户要求删除本插件，**先确认安装目录**后再移除；不要删除用户其它软件或 DSH 本体。
- 如用户要继续开发，保留仓库、日志、用户生成的 overlay 和构建结果，避免破坏调试现场。

## 9. 执行完成后必须交付的报告

使用下面的结构报告，缺项标注“未验证”，不要猜：

~~~text
DSH Computer Use 安装结果
- 安装状态：成功 / 构建成功但等待 DSH 重启 / 失败
- 安装路径：
- Git 分支与提交 SHA：
- 系统版本、Node/npm、Rust/cargo、DSH 版本：
- TypeScript 构建：通过 / 失败
- Rust 构建与单元测试：通过 / 失败 / 使用已验证 CI artifact
- 配置路径：
- DSH 启动命令及进程状态：
- MCP 工具发现：通过 / 失败 / 待验证
- health 权限状态：
- Windows UIA 桌面验证：通过 / 降级 / 待授权 / 待验证
- 未解决问题与日志位置：
- 安全提醒：capture/input 默认关闭，未授权不操作桌面
~~~

## 10. 官方依据与进一步调试

- 本仓库调试文档：https://github.com/haotemans/dsh-computer-use/blob/feat/windows-uia-debug-round/docs/windows-debugging.md
- 本仓库配置示例：https://github.com/haotemans/dsh-computer-use/blob/feat/windows-uia-debug-round/examples/cordis.patch.yml
- DSH 官方第一个插件指南：https://github.com/deepseek-ai/deepseek-harness/blob/main/docs/user/develop/basic/index.md
- DSH 官方运行方式：https://github.com/deepseek-ai/deepseek-harness/blob/main/README.md

**最终验收标准：** 在真实 DSH 会话中成功加载 Cordis 插件、MCP 工具可发现、health 与默认权限检查通过；真实 Windows 桌面功能必须另行实测。若 Agent 环境没有终端或桌面控制能力，应提供已生成的配置和精确阻塞点，而不是声称已经安装。
