# dida-mcp

一个面向任务场景的 Dida365 / TickTick MCP HTTP 中转服务.

它做了两件事:

- 对外暴露一个精简的 HTTP MCP 服务.
- 把任务和项目相关的工具调用, 转发到远端 `https://mcp.dida365.com`.

另外, 本地还额外提供了一个 `get_current_time` 工具.

## 功能概览

- 基于 [modelcontextprotocol/rust-sdk](https://github.com/modelcontextprotocol/rust-sdk) 的 `rmcp` 实现本地 HTTP MCP 服务.
- 使用 `Authorization: Bearer <token>` 调用远端 Dida MCP 服务.
- 只保留高频任务相关工具, 便于模型使用和维护.
- 支持通过 `config.toml` 配置本地监听地址, MCP 路径, 远端地址和认证信息.
- 支持可选的本地入站 Bearer 校验.

## 远端版本

当前 `README` 记录的远端 Dida / TickTick MCP 版本信息是:

```text
1.26.0 TickTick MCP Server
```

## 当前提供的工具

- `list_projects`
- `get_project_by_id`
- `get_project_with_undone_tasks`
- `create_task`
- `update_task`
- `get_task_by_id`
- `search_task`
- `list_undone_tasks_by_date`
- `complete_task`
- `get_current_time`

## 简化思路

这版服务的简化不是“随便删一些工具”, 而是围绕“创建任务和相关内容”做收缩.

原始 `assets/tools.json` 里有 32 个工具. 这版只保留任务主链路和少量必要辅助能力:

- 任务写入主链路: `create_task`, `update_task`, `complete_task`
- 任务读取辅助: `get_task_by_id`, `search_task`, `list_undone_tasks_by_date`
- 项目上下文: `list_projects`, `get_project_by_id`, `get_project_with_undone_tasks`
- 本地补充工具: `get_current_time`

具体简化体现在下面几个方面:

1. 工具范围简化

- 去掉 habits, focus, fetch, search, batch, move, completed, filter, project 写操作等非核心能力.
- 不追求做成“远端 TickTick MCP 的完整镜像”, 而是保留最常被模型调用的任务工具集合.

2. 参数结构简化

- `create_task` 和 `update_task` 不要求调用方手动构造完整嵌套 `task` 对象.
- 本地接口改成更容易调用的扁平参数, 再由中转服务映射为远端所需结构.

3. 责任边界简化

- 本地服务主要负责 3 件事: 接入 HTTP MCP, 处理 Bearer 策略, 转发远端工具调用.
- 不在本地重做 TickTick 的完整业务层, 尽量避免本地状态和远端状态分叉.

4. 配置面简化

- 所有关键行为集中在一个 `config.toml`.
- 只保留监听地址, MCP 路径, stateful 开关, 本地入站 Bearer, 远端 Bearer 模式, 远端地址, 默认时区这些核心配置.

这样做的目标是:

- 让模型更容易选中正确工具.
- 让人类更容易理解和维护.
- 把复杂度放在“认证和转发策略”上, 而不是放在“本地复刻整套 TickTick 能力”上.

## 快速上手

### 1. 准备配置文件

如果你想从示例开始, 可以先复制一份:

```bash
cp config.toml.example config.toml
```

然后编辑 `config.toml`, 至少填好下面这个配置:

```toml
[remote]
url = "https://mcp.dida365.com"
bearer_mode = "fixed"
bearer_token = "your-dida-mcp-bearer-token"
```

如果你不需要保护本地中转服务, 可以把下面这个值留空:

```toml
[server]
inbound_bearer_token = ""
```

### 2. 启动服务

在项目根目录执行:

```bash
RUSTC_WRAPPER= cargo run -- config.toml
```

默认会监听:

```text
127.0.0.1:8787
```

默认 MCP 路径是:

```text
/mcp
```

所以完整入口默认是:

```text
http://127.0.0.1:8787/mcp
```

### 3. 检查服务是否启动成功

健康检查:

```bash
curl http://127.0.0.1:8787/healthz
```

如果返回类似下面的 JSON, 说明服务已经起来了:

```json
{"status":"ok","server_time":"2026-05-01T12:00:00Z"}
```

### 4. 让 MCP 客户端连接

把你的 MCP 客户端指向:

```text
http://127.0.0.1:8787/mcp
```

如果你配置了 `server.inbound_bearer_token`, 客户端还需要在请求头里附带:

```text
Authorization: Bearer <your-local-token>
```

## 配置说明

配置文件结构如下:

```toml
[server]
listen = "127.0.0.1:8787"
base_path = "/mcp"
stateful_mode = true
disable_host_validation = true
sse_keep_alive_secs = 15
inbound_bearer_token = ""

[remote]
url = "https://mcp.dida365.com"
bearer_mode = "fixed"
incoming_bearer_header = "Authorization"
bearer_token = ""

[tools]
enable_get_current_time = true
default_timezone = "Asia/Shanghai"
```

各配置项的详细说明见 [src/config.rs](/Users/azazo1/pjs/rust/dida-mcp/src/config.rs:8).

常用项说明:

- `server.listen`: 本地监听地址.
- `server.base_path`: MCP HTTP 路径前缀.
- `server.stateful_mode`: 是否启用有状态会话模式. 为 `true` 时, 更适合标准 MCP 客户端.
- `server.inbound_bearer_token`: 本地服务的入站 Bearer token. 留空表示不校验.
- `remote.url`: 远端 MCP 服务地址. 默认是 `https://mcp.dida365.com`.
- `remote.bearer_mode`: 远端 Bearer 的处理模式.
- `remote.incoming_bearer_header`: 透传模式下, 从哪个入站请求头读取 Bearer token.
- `remote.bearer_token`: 固定模式或回退模式使用的远端 Bearer token. 不要带 `Bearer ` 前缀.
- `tools.enable_get_current_time`: 是否启用本地时间工具.
- `tools.default_timezone`: `get_current_time` 的默认时区.

### 2 个 Bearer token 的区别

这个服务里可能同时出现 2 类 Bearer token. 它们不是一回事:

1. 本地入站 Bearer token
   作用: 保护你自己的中转服务, 防止别人直接调用 `http://127.0.0.1:8787/mcp`.
   配置项: `server.inbound_bearer_token`
   方向: `MCP 客户端 -> dida-mcp`

2. 远端出站 Bearer token
   作用: 让 dida-mcp 去调用远端 `https://mcp.dida365.com`.
   配置项: `remote.bearer_mode`, `remote.incoming_bearer_header`, `remote.bearer_token`
   方向: `dida-mcp -> mcp.dida365.com`

可以把它理解成:

- `server.inbound_bearer_token` 是“谁能调用我这个中转服务”.
- `remote.*` 是“我调用远端 Dida MCP 时, 应该带哪个 token”.

一个常见误区是:

- `server.inbound_bearer_token` 不会自动变成远端 token.
- 只有在 `remote.bearer_mode = "passthrough"` 或 `remote.bearer_mode = "passthrough_or_fixed"` 时, dida-mcp 才会从入站请求头里取 token 再转发给远端.

### Bearer 模式总览

下面这张表可以先帮助你建立直觉:

| 场景 | bearer_mode | 本地校验用哪个配置 | 远端最终带哪个 Bearer |
| --- | --- | --- | --- |
| 只想固定用 1 个远端 token | `fixed` | `server.inbound_bearer_token` 可空 | `remote.bearer_token` |
| 想把客户端传进来的 token 原样转发 | `passthrough` | `server.inbound_bearer_token` 可空 | `remote.incoming_bearer_header` 对应请求头里的 token |
| 想优先透传, 没传时再走固定 token | `passthrough_or_fixed` | `server.inbound_bearer_token` 可空 | 先读入站请求头, 没有再用 `remote.bearer_token` |
| 远端不需要认证 | `none` | `server.inbound_bearer_token` 可空 | 不发送 `Authorization` |

### 各种远端 auth 模式

`remote.bearer_mode` 支持下面几种模式:

- `fixed`: 固定使用 `remote.bearer_token`.
- `passthrough`: 从入站请求头透传 Bearer token.
- `passthrough_or_fixed`: 优先透传, 透传缺失时回退到 `remote.bearer_token`.
- `none`: 不发送 `Authorization` 请求头.

#### 模式 1. `fixed`

适用场景:

- 你已经有一个固定的 Dida MCP token.
- 所有客户端都共用同一个远端身份.

配置示例:

```toml
[server]
inbound_bearer_token = ""

[remote]
url = "https://mcp.dida365.com"
bearer_mode = "fixed"
bearer_token = "your-dida-mcp-bearer-token"
```

客户端请求示例:

```text
POST /mcp
Authorization: Bearer anything-or-empty
```

远端实际收到:

```text
Authorization: Bearer your-dida-mcp-bearer-token
```

说明:

- 客户端传什么 `Authorization`, 不影响远端.
- 远端永远使用 `remote.bearer_token`.

#### 模式 2. `passthrough`

适用场景:

- 每个客户端都有自己的远端 Dida MCP token.
- 你希望中转服务不要保存固定远端 token.

最简单的配置示例:

```toml
[server]
inbound_bearer_token = ""

[remote]
url = "https://mcp.dida365.com"
bearer_mode = "passthrough"
incoming_bearer_header = "Authorization"
```

客户端请求示例:

```text
POST /mcp
Authorization: Bearer client-remote-token
```

远端实际收到:

```text
Authorization: Bearer client-remote-token
```

说明:

- dida-mcp 会读取 `incoming_bearer_header` 指定的请求头.
- 然后把其中的 Bearer token 原样转发给远端.

#### 模式 2 的重要辨析

如果你同时配置了:

```toml
[server]
inbound_bearer_token = "local-gateway-token"

[remote]
bearer_mode = "passthrough"
incoming_bearer_header = "Authorization"
```

那么会发生:

- 客户端发给 dida-mcp 的 `Authorization` 会先用于本地校验.
- 同一个 `Authorization` 也会继续被拿去转发到远端.

也就是说, 这种写法等价于“本地校验 token 和远端 token 是同一个值”.

只有当你的本地校验 token 和远端 Dida token 本来就是同一个值时, 这种配置才合理.

#### 模式 2. 推荐的双 token 写法

如果你希望“本地校验 token” 和 “远端透传 token” 是两个不同值, 推荐这样配:

```toml
[server]
inbound_bearer_token = "local-gateway-token"

[remote]
url = "https://mcp.dida365.com"
bearer_mode = "passthrough"
incoming_bearer_header = "X-Remote-Authorization"
```

此时:

- 客户端访问本地中转服务时, `Authorization` 用于本地鉴权.
- 客户端额外传 `X-Remote-Authorization: Bearer <remote-token>`, 用于远端透传.

客户端请求示例:

```text
POST /mcp
Authorization: Bearer local-gateway-token
X-Remote-Authorization: Bearer client-remote-token
```

远端实际收到:

```text
Authorization: Bearer client-remote-token
```

#### 模式 3. `passthrough_or_fixed`

适用场景:

- 大多数时候想用固定远端 token.
- 但偶尔允许某些客户端临时覆盖成自己的远端 token.

配置示例:

```toml
[server]
inbound_bearer_token = "local-gateway-token"

[remote]
url = "https://mcp.dida365.com"
bearer_mode = "passthrough_or_fixed"
incoming_bearer_header = "X-Remote-Authorization"
bearer_token = "default-remote-token"
```

情况 A. 客户端没有传 `X-Remote-Authorization`

客户端请求:

```text
POST /mcp
Authorization: Bearer local-gateway-token
```

远端实际收到:

```text
Authorization: Bearer default-remote-token
```

情况 B. 客户端传了 `X-Remote-Authorization`

客户端请求:

```text
POST /mcp
Authorization: Bearer local-gateway-token
X-Remote-Authorization: Bearer override-remote-token
```

远端实际收到:

```text
Authorization: Bearer override-remote-token
```

#### 模式 4. `none`

适用场景:

- 远端服务本身不需要 `Authorization`.
- 或者你当前在接另一个不需要 Bearer 的 MCP 端点做测试.

配置示例:

```toml
[server]
inbound_bearer_token = "local-gateway-token"

[remote]
url = "https://mcp.dida365.com"
bearer_mode = "none"
bearer_token = ""
```

客户端请求:

```text
POST /mcp
Authorization: Bearer local-gateway-token
```

远端实际收到:

```text
# 不会发送 Authorization 请求头
```

### 推荐配置

如果你只是自己本地用, 最不容易出错的是固定模式:

```toml
[server]
inbound_bearer_token = ""

[remote]
url = "https://mcp.dida365.com"
bearer_mode = "fixed"
bearer_token = "your-dida-mcp-bearer-token"
```

如果你要把本地网关 token 和远端 Dida token 分开, 推荐:

```toml
[server]
inbound_bearer_token = "local-gateway-token"

[remote]
url = "https://mcp.dida365.com"
bearer_mode = "passthrough"
incoming_bearer_header = "X-Remote-Authorization"
```

## create_task 和 update_task 的输入风格

为了便于模型调用, 本地暴露的 `create_task` 和 `update_task` 使用了扁平化参数, 而不是把所有字段包在 `task` 对象里.

例如创建任务时, 常见字段可以直接传:

- `project_id`
- `title`
- `content`
- `start_date`
- `due_date`
- `priority`
- `tags`
- `repeat_flag`
- `checklist_items`

服务内部会自动把这些参数转换成远端 Dida MCP 所需的 `task` 结构.

## 开发说明

### 编译检查

```bash
cargo check
```

### 代码结构

- [src/main.rs](/Users/azazo1/pjs/rust/dida-mcp/src/main.rs:1): 程序入口.
- [src/config.rs](/Users/azazo1/pjs/rust/dida-mcp/src/config.rs:1): 配置加载和校验.
- [src/server.rs](/Users/azazo1/pjs/rust/dida-mcp/src/server.rs:1): HTTP 服务启动, 路由和鉴权.
- [src/proxy/mod.rs](/Users/azazo1/pjs/rust/dida-mcp/src/proxy/mod.rs:1): MCP 工具注册和远端转发.
- [src/proxy/types.rs](/Users/azazo1/pjs/rust/dida-mcp/src/proxy/types.rs:1): 工具参数和远端 payload 映射.

## 注意事项

- 当 `remote.bearer_mode = "fixed"` 时, `remote.bearer_token` 不能为空.
- 透传模式下, 指定的入站请求头必须满足 `Bearer <token>` 格式.
- `server.base_path` 必须以 `/` 开头.
- `tools.default_timezone` 需要是合法的 IANA 时区名, 例如 `Asia/Shanghai`.
- `disable_host_validation = true` 更适合本地开发. 如果你要公开部署, 建议进一步收紧 Host / Origin 校验策略.
