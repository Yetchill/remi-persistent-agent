# Remi — 持久化个人 LLM Agent

[English](README.md) | [简体中文](README.zh-CN.md)

Remi 是一个以桌面伙伴为交互载体、优先支持 macOS 的持久化个人 LLM Agent。项目使用 Tauri 2、React、TypeScript、Rust 和 SQLite 构建。

## 目前能力

- 透明、置顶的 macOS 桌面角色，支持本地移动和切换 Pet Pack。
- 统一 Agent Runtime：用户聊天与自主 Heartbeat 都经过同一运行时。
- 可配置 OpenAI-compatible Provider、API 地址和模型。
- 通过可编辑的 `SOUL.md`、Pet State 和 Context Builder 保存身份与状态。
- 支持重启后仍然保留的 Working、Semantic、Episodic 和 Relationship Memory。
- 可以查看、修正、置顶、归档、恢复和删除记忆。
- 主动行为受到用户偏好、免打扰、安静时段、冷却时间和每小时上限约束。
- 支持 Companion Profile 导出与导入，且不包含 Provider 凭证或 API Key。
- 从一开始记录 event、action、memory operation 和 LLM trace。

## 配置 LLM Provider

### 在应用内配置

1. 启动 Remi，右键点击桌面角色。
2. 打开 **Settings → Providers**。
3. 添加 OpenAI-compatible Base URL、API Key 和一个或多个 Model ID。
4. 启用 Provider，并为需要使用的模型选择 **Set Active**。

API Key 只保存在当前进程内，不会写入 SQLite，也不会包含在 Companion Profile 导出文件中。

### 使用环境变量

以 `.env.example` 为参考，在启动 Tauri 的同一个终端中设置：

```bash
export REMI_LLM_BASE_URL="https://api.openai.com/v1"
export REMI_LLM_MODEL="your-model-id"
export REMI_LLM_API_KEY="your-api-key"
npm run tauri -- dev
```

目标服务需要提供兼容 OpenAI `/chat/completions` 的接口。

## 本地运行

环境要求：

- macOS
- Node.js 20 或更新版本
- Rust stable
- Apple Command Line Tools（`xcode-select --install`）

```bash
git clone https://github.com/Yetchill/remi-persistent-agent.git
cd remi-persistent-agent
npm install
npm run tauri -- dev
```

## macOS 构建与部署

运行检查并生成应用：

```bash
npm install
npm run typecheck
npm test
npm run tauri -- build
```

构建产物位于：

```text
src-tauri/target/release/bundle/macos/Remi.app
```

本机安装时，将 `Remi.app` 复制到 `/Applications` 即可。

如果要在 Mac App Store 之外公开分发，需要在钥匙串中安装 **Developer ID Application** 证书，然后查找并设置签名身份：

```bash
security find-identity -v -p codesigning
export APPLE_SIGNING_IDENTITY="Developer ID Application: Your Name (TEAM_ID)"
```

可以使用 App Store Connect API Key 完成公证：

```bash
export APPLE_API_ISSUER="your-issuer-id"
export APPLE_API_KEY="your-key-id"
export APPLE_API_KEY_PATH="/absolute/path/to/AuthKey_KEYID.p8"
npm run tauri -- build
```

也可以使用 Tauri 支持的 `APPLE_ID`、Apple 专用密码 `APPLE_PASSWORD` 和 `APPLE_TEAM_ID`。不要提交证书、私钥、签名密码、API Key 或 `.env.local`。详细步骤见 [Tauri 官方 macOS 签名与公证文档](https://v2.tauri.app/zh-cn/distribute/sign/macos/)。

应用数据保存在 `dev.remi.personal-agent` 对应的 macOS app-data 目录，其中包括 `SOUL.md`、SQLite 记忆与 trace、Pet State、导入的 Pet Pack 和本地 Profile 备份。

## 未来方向

- 研究长期记忆的修订、替代、巩固和遗忘机制。
- 改进 Heartbeat 策略以及对个人偏好的适配。
- 增加更丰富的角色外观、动画系统和跨平台支持。
- 探索语音交互、外部工具接入和更完整的隐私控制。
- 完成经过签名和公证的 macOS 正式发布流程。
