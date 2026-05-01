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
- `remote.bearer_token`: 远端 Dida MCP 的 Bearer token. 不要带 `Bearer ` 前缀.
- `tools.enable_get_current_time`: 是否启用本地时间工具.
- `tools.default_timezone`: `get_current_time` 的默认时区.

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
RUSTC_WRAPPER= cargo check
```

### 代码结构

- [src/main.rs](/Users/azazo1/pjs/rust/dida-mcp/src/main.rs:1): 程序入口.
- [src/config.rs](/Users/azazo1/pjs/rust/dida-mcp/src/config.rs:1): 配置加载和校验.
- [src/server.rs](/Users/azazo1/pjs/rust/dida-mcp/src/server.rs:1): HTTP 服务启动, 路由和鉴权.
- [src/proxy/mod.rs](/Users/azazo1/pjs/rust/dida-mcp/src/proxy/mod.rs:1): MCP 工具注册和远端转发.
- [src/proxy/types.rs](/Users/azazo1/pjs/rust/dida-mcp/src/proxy/types.rs:1): 工具参数和远端 payload 映射.

## 注意事项

- `remote.bearer_token` 不能为空, 否则服务启动时会直接报错.
- `server.base_path` 必须以 `/` 开头.
- `tools.default_timezone` 需要是合法的 IANA 时区名, 例如 `Asia/Shanghai`.
- `disable_host_validation = true` 更适合本地开发. 如果你要公开部署, 建议进一步收紧 Host / Origin 校验策略.
