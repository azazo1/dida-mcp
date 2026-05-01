use std::{
    env,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use chrono_tz::Tz;
use serde::Deserialize;

/// 默认配置文件名. 当命令行没有显式传入路径时, 会读取当前工作目录下的这个文件.
pub const DEFAULT_CONFIG_PATH: &str = "config.toml";

/// 顶层配置. 对应 `config.toml` 的根结构.
#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    /// 本地 HTTP MCP 服务自身的监听和协议配置.
    #[serde(default)]
    pub server: ServerConfig,
    /// 远端 MCP 服务的地址和认证信息.
    pub remote: RemoteConfig,
    /// 本地补充工具的开关和默认行为配置.
    #[serde(default)]
    pub tools: ToolConfig,
}

/// 本地 HTTP MCP 服务配置.
#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    /// 本地监听地址. 格式通常是 `host:port`, 例如 `127.0.0.1:8787`.
    #[serde(default = "default_listen")]
    pub listen: String,
    /// MCP HTTP 路径前缀. 一般保持为 `/mcp`.
    #[serde(default = "default_base_path")]
    pub base_path: String,
    /// 是否启用有状态会话模式. 为 `true` 时, 服务会维护 MCP session, 并允许 `GET` 和 `DELETE` 请求.
    #[serde(default = "default_stateful_mode")]
    pub stateful_mode: bool,
    /// 是否关闭 `Host` 白名单校验. 为 `true` 时更宽松, 但公开部署时安全性更低.
    #[serde(default = "default_disable_host_validation")]
    pub disable_host_validation: bool,
    /// SSE keep-alive 心跳间隔, 单位为秒. 仅对流式响应连接生效.
    #[serde(default = "default_sse_keep_alive_secs")]
    pub sse_keep_alive_secs: u64,
    /// 可选的本地入站 Bearer token. 留空或 `None` 表示不校验调用方的 `Authorization` 请求头.
    pub inbound_bearer_token: Option<String>,
}

/// 远端 MCP 服务配置.
#[derive(Debug, Clone, Deserialize)]
pub struct RemoteConfig {
    /// 远端 MCP 服务地址. 默认可指向 `https://mcp.dida365.com`.
    pub url: String,
    /// 远端 Bearer 的发送模式.
    ///
    /// 可选值:
    /// - `fixed`: 固定使用 `bearer_token`.
    /// - `passthrough`: 从入站请求头透传 Bearer token.
    /// - `passthrough_or_fixed`: 优先透传, 透传缺失时回退到 `bearer_token`.
    /// - `none`: 不发送 `Authorization` 请求头.
    #[serde(default = "default_remote_bearer_mode")]
    pub bearer_mode: RemoteBearerMode,
    /// 在透传模式下, 从哪个入站 HTTP 请求头读取 Bearer token.
    ///
    /// 默认值是 `Authorization`.
    /// 如果你既要保护本地中转服务, 又要把远端 token 独立透传, 可以改成例如 `X-Remote-Authorization`.
    #[serde(default = "default_incoming_bearer_header")]
    pub incoming_bearer_header: String,
    /// 固定模式或回退模式下使用的 Bearer token. 配置值本身不要包含 `Bearer ` 前缀.
    #[serde(default)]
    pub bearer_token: String,
}

/// 本地补充工具配置.
#[derive(Debug, Clone, Deserialize)]
pub struct ToolConfig {
    /// 是否启用本地 `get_current_time` 工具.
    #[serde(default = "default_enable_get_current_time")]
    pub enable_get_current_time: bool,
    /// `get_current_time` 默认使用的 IANA 时区名称, 例如 `Asia/Shanghai`. 留空时回退到 `UTC`.
    pub default_timezone: Option<String>,
}

/// 远端 Bearer token 的处理模式.
#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RemoteBearerMode {
    Fixed,
    Passthrough,
    PassthroughOrFixed,
    None,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            listen: default_listen(),
            base_path: default_base_path(),
            stateful_mode: default_stateful_mode(),
            disable_host_validation: default_disable_host_validation(),
            sse_keep_alive_secs: default_sse_keep_alive_secs(),
            inbound_bearer_token: None,
        }
    }
}

impl Default for ToolConfig {
    fn default() -> Self {
        Self {
            enable_get_current_time: default_enable_get_current_time(),
            default_timezone: None,
        }
    }
}

impl Default for RemoteBearerMode {
    fn default() -> Self {
        default_remote_bearer_mode()
    }
}

pub fn resolve_config_path() -> PathBuf {
    env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CONFIG_PATH))
}

pub fn load_config(path: &Path) -> Result<AppConfig> {
    let contents = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read config file: {}", path.display()))?;
    toml::from_str(&contents)
        .with_context(|| format!("failed to parse config file: {}", path.display()))
}

pub fn validate_config(config: &AppConfig) -> Result<()> {
    if !config.server.base_path.starts_with('/') {
        bail!("server.base_path must start with `/`");
    }

    if config.remote.url.trim().is_empty() {
        bail!("remote.url cannot be empty");
    }

    if config.remote.incoming_bearer_header.trim().is_empty() {
        bail!("remote.incoming_bearer_header cannot be empty");
    }

    if config.remote.bearer_mode == RemoteBearerMode::Fixed
        && config.remote.bearer_token.trim().is_empty()
    {
        bail!("remote.bearer_token cannot be empty when remote.bearer_mode = `fixed`");
    }

    if let Some(timezone) = config.tools.default_timezone.as_deref() {
        timezone
            .parse::<Tz>()
            .with_context(|| format!("invalid tools.default_timezone: {timezone}"))?;
    }

    Ok(())
}

pub fn normalized_token(token: Option<&str>) -> Option<&str> {
    token.and_then(|value| {
        let trimmed = value.trim();
        (!trimmed.is_empty()).then_some(trimmed)
    })
}

fn default_listen() -> String {
    "127.0.0.1:8787".to_owned()
}

fn default_base_path() -> String {
    "/mcp".to_owned()
}

fn default_stateful_mode() -> bool {
    true
}

fn default_disable_host_validation() -> bool {
    true
}

fn default_sse_keep_alive_secs() -> u64 {
    15
}

fn default_enable_get_current_time() -> bool {
    true
}

fn default_remote_bearer_mode() -> RemoteBearerMode {
    RemoteBearerMode::Fixed
}

fn default_incoming_bearer_header() -> String {
    "Authorization".to_owned()
}
