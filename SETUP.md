# Cloudflare Worker 注册功能部署说明

## 1) 创建并绑定 D1

1. 在 Cloudflare 控制台创建 D1 数据库，或使用 CLI:
   - `wrangler d1 create generator-db`
2. 将返回的 `database_id` 写入 `wrangler.toml` 的 `[[d1_databases]]`。
3. 初始化表结构:
   - `wrangler d1 execute generator-db --file=./schema.sql`

## 2) 创建并绑定 KV

1. 创建 KV 命名空间:
   - `wrangler kv namespace create KV_LIMITER`
2. 将返回的 `id` 写入 `wrangler.toml` 的 `[[kv_namespaces]]`。

> 当前实现已使用 KV 做窗口限速；如果你额外配置了 Cloudflare Rate Limiter 绑定（名称 `REG_RATE_LIMITER`），后端会自动叠加该限速。

## 3) 配置 Secret（邀请码、密码 Pepper、DeepSeek 密钥）

- `wrangler secret put INVITE_CODE`
- `wrangler secret put PASSWORD_PEPPER`
- `wrangler secret put DEEPSEEK_API_KEY`

如果不想用 Secret，也可用 `[vars]` 配置同名变量，但不建议。

## 4) 可选 Var 配置

在 `wrangler.toml` 的 `[vars]` 中已预置:

- `SUPPORT_EMAIL`：邀请码提示邮箱（默认 `krvyft@pm.me`）
- `CORS_ORIGIN`：允许跨域来源（默认 `*`）
- `RATE_LIMIT_MAX`：窗口内最大请求数（默认 `5`）
- `RATE_LIMIT_WINDOW_SECS`：限速窗口秒数（默认 `60`）
- `SESSION_TTL_SECS`：登录会话有效期（秒，默认 `86400`）

> 当前实现为后端直连 DeepSeek（`https://api.deepseek.com/chat/completions`），不再依赖中间代理 Worker。

## 5) 数据表更新

`schema.sql` 已新增 `documents` 表（保存用户工作区文档）。

如果你之前已初始化过数据库，记得重新执行：

- `wrangler d1 execute generator-db --file=./schema.sql`

## 6) 前后端分离调用

- 后端 API：`POST /api/register`
- 请求体 JSON:

```json
{
  "username": "alice",
  "password": "your-password",
  "invite_code": "your-invite-code"
}
```

新增一体化页面 `app.html`，流程为：

- 注册或登录
- 进入功能界面并交互编辑
- 保存/加载工作区
- 后端生成并下载 PDF

前端页面文件已统一放在 `public/` 目录：

- `public/app.html`
- `public/presentation.html`
