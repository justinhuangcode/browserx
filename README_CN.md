# BrowserX

[English](./README.md) | **中文**

[![CI](https://github.com/justinhuangcode/browserx/actions/workflows/ci.yml/badge.svg)](https://github.com/justinhuangcode/browserx/actions/workflows/ci.yml)
[![Release](https://github.com/justinhuangcode/browserx/actions/workflows/release.yml/badge.svg)](https://github.com/justinhuangcode/browserx/actions/workflows/release.yml)
[![Crates.io](https://img.shields.io/crates/v/browserx?style=flat-square)](https://crates.io/crates/browserx)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg?style=flat-square)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg?style=flat-square&logo=rust&logoColor=white)](https://www.rust-lang.org)
[![GitHub Stars](https://img.shields.io/github/stars/justinhuangcode/browserx?style=flat-square&logo=github)](https://github.com/justinhuangcode/browserx/stargazers)
[![Last Commit](https://img.shields.io/github/last-commit/justinhuangcode/browserx?style=flat-square)](https://github.com/justinhuangcode/browserx/commits/main)
[![Issues](https://img.shields.io/github/issues/justinhuangcode/browserx?style=flat-square)](https://github.com/justinhuangcode/browserx/issues)
[![Platform](https://img.shields.io/badge/platform-macOS%20%7C%20Linux%20%7C%20Windows-lightgrey?style=flat-square)](https://github.com/justinhuangcode/browserx)

跨平台浏览器 Cookie 提取命令行工具，支持会话重放、身份认证与自动化。 🍪

单一 Rust 二进制文件，读取 macOS、Linux、Windows 上 9 款浏览器的 Cookie 数据库，使用原生 OS API 解密加密值，支持 5 种输出格式。专为 AI Agent、CLI 自动化与编程化身份认证访问设计。

## 为什么选择 BrowserX？

AI Agent 和 CLI 工具在发起需身份认证的 HTTP 请求时，需要**复用浏览器登录会话**。它们需要提取 Cookie、构建请求头、检查会话是否有效、失效时重新获取——而这一切不需要人类手动打开 DevTools。

现有工具无法满足这个工作流：

| | browserx | sweet-cookie | cookie-editor | EditThisCookie |
|---|---|---|---|---|
| 为 AI Agent 设计 | 是 | 否（仅库） | 否（扩展） | 否（扩展） |
| 单二进制，零运行时 | 是（Rust） | 否（Node >= 22） | 否（Chrome） | 否（Chrome） |
| CLI + 库 | CLI + Rust crate | 仅库 | 仅扩展 | 仅扩展 |
| 原生 OS 加密 | Keychain / DPAPI / D-Bus | Shell 调用（`security`、`powershell`） | N/A | N/A |
| 浏览器支持 | 9 款 | 4 款 | 1 款 | 1 款 |
| 输出格式 | 5 种（JSON/curl/Netscape/ENV/表格） | 1 种（JSON） | N/A | N/A |
| 加密保险库 | ChaCha20-Poly1305，TTL | 无 | 无 | 无 |
| 会话健康检查 | 是 | 否 | 否 | 否 |
| Inline 优先设计 | 是 | 是 | 否 | 否 |

**AI Agent 使用 browserx 的典型工作流：**

```
Agent 需要调用需认证的 API
        |
browserx get --url https://github.com --format curl
        |
Agent 使用 Cookie 请求头发起 HTTP 请求
        |
browserx health --url https://github.com
        |
Agent 检查会话是否有效，失效则重新提取
```

无需浏览器扩展。无需 Node.js 运行时。无需测试框架样板代码。只需一个二进制文件，读取你的 Cookie 并以所需格式输出。

## 特性

- **9 款浏览器** -- Chrome、Edge、Firefox、Safari、Brave、Arc、Vivaldi、Opera、Chromium
- **3 大平台** -- macOS、Linux、Windows，使用平台专属原生加密
- **5 种输出格式** -- JSON、curl、Netscape cookie.txt、环境变量、人类可读表格
- **Inline 优先设计** -- Inline Cookie 数据（JSON、base64、文件）优先于本地浏览器数据库检查，命中即短路返回
- **加密保险库** -- 使用 ChaCha20-Poly1305 加密本地存储提取的 Cookie，支持 TTL 自动过期
- **会话健康检查** -- 检查 Cookie 状态（活跃、即将过期、已过期），无需发起 HTTP 请求
- **原生 OS 加密** -- macOS Keychain（`security-framework`）、Windows DPAPI（`windows-sys`）、Linux Secret Service（D-Bus），无子进程调用
- **安全默认** -- Cookie 值包装在 `SecretValue` 中（`Debug`/`Display` 显示 `***`，drop 时通过 `zeroize` 清零），不会写入日志
- **WAL 安全读取** -- 复制 SQLite 数据库 + WAL/SHM 附属文件到临时目录后读取，不锁定浏览器活跃数据库

## 安装

### 预编译二进制（推荐）

从 [GitHub Releases](https://github.com/justinhuangcode/browserx/releases) 下载适合你平台的最新二进制文件。

### Homebrew（macOS / Linux）

```bash
brew tap justinhuangcode/tap
brew install browserx
```

### 通过 Cargo（crates.io）

```bash
cargo install browserx
```

### 从源码编译

```bash
git clone https://github.com/justinhuangcode/browserx
cd browserx
cargo install --path crates/browserx-cli
```

**依赖：** 仅需 Rust 1.75+。无需安装浏览器——browserx 直接从磁盘读取 Cookie 数据库。

## 快速上手

```bash
# 从所有检测到的浏览器提取 Cookie
browserx get --url https://github.com

# 从 Chrome 提取，输出为 curl 兼容格式
browserx get --url https://github.com --browser chrome --format curl

# 直接与 curl 配合使用
curl -b "$(browserx get --url https://github.com --format curl)" https://github.com/api

# 按 Cookie 名称过滤
browserx get --url https://x.com --names "auth_token,ct0"

# 导出为 Netscape cookie.txt
browserx get --url https://github.com --format netscape > cookies.txt

# 列出检测到的浏览器和配置文件
browserx browsers

# 检查会话健康状况
browserx health --url https://github.com

# 将 Cookie 存入加密保险库
browserx vault store --url https://github.com --ttl 24h

# 从保险库检索（无需访问浏览器）
browserx vault get --url https://github.com

# 清理过期条目
browserx vault clean
```

## 命令

| 命令 | 说明 |
|---|---|
| `get` | 从浏览器提取指定 URL 的 Cookie |
| `browsers` | 列出检测到的浏览器及其配置文件 |
| `health` | 检查会话健康状况（活跃、即将过期、已过期） |
| `vault store` | 将提取的 Cookie 存入加密本地保险库 |
| `vault get` | 从保险库检索 Cookie |
| `vault list` | 列出所有保险库条目及状态 |
| `vault clean` | 清除过期的保险库条目 |

## 支持的浏览器

| 浏览器 | macOS | Linux | Windows | 引擎 | 加密方式 |
|---|---|---|---|---|---|
| Google Chrome | 是 | 是 | 是 | Chromium | AES-128-CBC（macOS/Linux）、AES-256-GCM（Windows） |
| Microsoft Edge | 是 | 是 | 是 | Chromium | AES-128-CBC（macOS/Linux）、AES-256-GCM（Windows） |
| Mozilla Firefox | 是 | 是 | 是 | Gecko | 无（明文） |
| Apple Safari | 是 | -- | -- | WebKit | 无（二进制格式） |
| Brave | 是 | 是 | 是 | Chromium | AES-128-CBC（macOS/Linux）、AES-256-GCM（Windows） |
| Arc | 是 | -- | 是 | Chromium | AES-128-CBC（macOS/Linux）、AES-256-GCM（Windows） |
| Vivaldi | 是 | 是 | 是 | Chromium | AES-128-CBC（macOS/Linux）、AES-256-GCM（Windows） |
| Opera | 是 | 是 | 是 | Chromium | AES-128-CBC（macOS/Linux）、AES-256-GCM（Windows） |
| Chromium | 是 | 是 | 是 | Chromium | AES-128-CBC（macOS/Linux）、AES-256-GCM（Windows） |

## 故障排除

详见 [TROUBLESHOOTING.md](TROUBLESHOOTING.md)。

## 贡献

欢迎贡献！请参阅 [CONTRIBUTING.md](CONTRIBUTING.md)。

## 许可证

[MIT](LICENSE)
