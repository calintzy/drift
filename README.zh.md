[English](README.md) | [한국어](README.ko.md) | **中文** | [日本語](README.ja.md)

# drift

[![Rust](https://img.shields.io/badge/built%20with-Rust-orange)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue)](LICENSE)
[![Build Status](https://img.shields.io/badge/build-passing-brightgreen)]()

**在依赖死亡之前，提前预警。**

Drift 是一款 Rust CLI 工具，通过分析来自 GitHub、npm 和 OSV 的信号，对项目的依赖健康状况进行评分（0–100）。它能在那些濒死的库成为你的麻烦之前，提前向你发出警告。

Drift 扫描你的 `package.json`，从多个来源获取实时数据，并将每个依赖分类为相应的风险等级——让你能够在升级、替换和技术债务方面做出明智的决策。

---

## 示例输出

```
$ drift check

Dependency Health Report for my-project
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Package           Health  Risk     Signal
─────────────────────────────────────────
react             98/100  🟢 Safe  Active, 1,200+ contributors
axios             72/100  🟡 Watch Fewer maintainers, slower releases
moment            23/100  🔴 Dead  Deprecated, use dayjs/date-fns
event-emitter3    45/100  🟠 Risk  Solo maintainer, no activity 8mo
custom-lib        12/100  🔴 Dead  Archived, 0 downloads trend

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Summary: 47 deps | 38 safe | 6 watch | 3 risk
Action Required: 2 critical replacements suggested
```

---

## 安装

```bash
git clone https://github.com/calintzy/drift.git
cd drift
cargo build --release
```

二进制文件将位于 `./target/release/drift`。

---

## 使用方法

```bash
drift check                    # Scan all dependencies
drift check axios lodash       # Check specific packages
drift check --format json      # JSON output for CI
drift check --include-dev      # Include devDependencies
drift check --verbose          # Show detailed signal scores
```

### GitHub Token

不使用 token 时，GitHub API 的请求限制为每小时 60 次。使用 token 后，限制提升至每小时 5,000 次。

```bash
export GITHUB_TOKEN=ghp_your_token_here
drift check
```

---

## 评分机制

每个依赖使用 7 个独立信号的加权平均值进行 0–100 评分：

| 信号 | 权重 | 来源 |
|------|------|------|
| 最近提交时间 | 20% | GitHub API |
| 发布频率 | 15% | GitHub Releases |
| 维护者数量 | 15% | GitHub Contributors |
| Issue 响应时间 | 15% | GitHub Issues |
| 下载趋势 | 15% | npm Registry |
| CVE 历史记录 | 10% | OSV API |
| 社区活跃度（Stars + PR 合并率） | 10% | GitHub API |

**评分规则：**

- 每个信号独立评分（0–100）
- 若某信号因 API 故障无法获取，剩余权重将自动重新归一化（故障开放策略）
- 有效评分至少需要 2 个信号
- 已废弃的包会自动被标记为 Dead 等级
- 已归档的仓库将强制将最近提交分数设为 0

---

## 风险等级

| 分数 | 等级 | 含义 |
|------|------|------|
| 80–100 | 🟢 Safe | 健康，积极维护中 |
| 60–79 | 🟡 Watch | 出现放缓迹象 |
| 40–59 | 🟠 Risk | 建议考虑替代方案 |
| 0–39 | 🔴 Dead | 请立即迁移 |

---

## CI 集成

当发现任何 Risk 或 Dead 包时，Drift 会以退出码 `2` 退出，便于集成到 CI 流水线中。

```bash
drift check --format json
```

| 退出码 | 含义 |
|--------|------|
| `0` | 所有依赖均为 Safe 或 Watch |
| `1` | 错误（例如缺少 `package.json`） |
| `2` | 发现一个或多个 Risk 或 Dead 包 |

---

## 环境变量

| 变量 | 用途 |
|------|------|
| `GITHUB_TOKEN` | GitHub API 认证（认证后 5,000 次/小时，未认证 60 次/小时） |
| `DRIFT_LOG` | 日志级别：`debug`、`info`、`warn` |
| `NO_COLOR` | 禁用终端彩色输出 |

---

## 技术栈

使用 Rust 构建，依赖库包括：`clap v4`、`tokio`、`reqwest`、`serde_json`、`comfy-table`、`colored`、`thiserror`，以及 [OSV API](https://osv.dev/)。

---

## 路线图

**v0.2**

- `drift suggest` — 为 Dead/Risk 依赖推荐替代包
- `drift watch` — 持续监控依赖变化并发出警报
- 本地缓存以减少 API 调用并提升性能
- 多生态系统支持（Cargo、PyPI、Go modules）

---

## 贡献

欢迎贡献代码。在提交重大变更的 pull request 之前，请先开一个 issue 进行讨论。

1. Fork 本仓库
2. 创建功能分支（`git checkout -b feature/your-feature`）
3. 提交你的更改
4. 开启一个 pull request

---

## 许可证

MIT — 详情请参阅 [LICENSE](LICENSE)。
