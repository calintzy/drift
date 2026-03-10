# drift

[![Rust](https://img.shields.io/badge/built%20with-Rust-orange)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue)](LICENSE)
[![Build Status](https://img.shields.io/badge/build-passing-brightgreen)]()

**Know before your dependencies die.**

Drift is a Rust CLI tool that scores your project's dependency health (0–100) by analyzing signals from GitHub, npm, and OSV. It warns you about dying libraries before they become your problem.

Drift scans your `package.json`, fetches live data from multiple sources, and classifies each dependency into a risk grade — so you can make informed decisions about upgrades, replacements, and technical debt.

---

## Example Output

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

## Installation

```bash
git clone https://github.com/calintzy/drift.git
cd drift
cargo build --release
```

The binary will be available at `./target/release/drift`.

---

## Usage

```bash
drift check                    # Scan all dependencies
drift check axios lodash       # Check specific packages
drift check --format json      # JSON output for CI
drift check --include-dev      # Include devDependencies
drift check --verbose          # Show detailed signal scores
```

### GitHub Token

Without a token, the GitHub API is rate-limited to 60 requests/hour. With a token, the limit is 5,000 requests/hour.

```bash
export GITHUB_TOKEN=ghp_your_token_here
drift check
```

---

## How Scoring Works

Each dependency is scored 0–100 using a weighted average of 7 independent signals:

| Signal | Weight | Source |
|--------|--------|--------|
| Last Commit | 20% | GitHub API |
| Release Frequency | 15% | GitHub Releases |
| Maintainer Count | 15% | GitHub Contributors |
| Issue Response Time | 15% | GitHub Issues |
| Download Trend | 15% | npm Registry |
| CVE History | 10% | OSV API |
| Community (Stars + PR merge rate) | 10% | GitHub API |

**Scoring rules:**

- Each signal is scored independently (0–100)
- If a signal is unavailable due to API failure, remaining weights are renormalized (fail-open)
- A minimum of 2 signals is required for a valid score
- Deprecated packages are automatically assigned the Dead grade
- Archived repositories force the Last Commit score to 0

---

## Risk Grades

| Score | Grade | Meaning |
|-------|-------|---------|
| 80–100 | 🟢 Safe | Healthy, actively maintained |
| 60–79 | 🟡 Watch | Signs of slowing down |
| 40–59 | 🟠 Risk | Consider alternatives |
| 0–39 | 🔴 Dead | Migrate immediately |

---

## CI Integration

Drift exits with code `2` when any Risk or Dead packages are found, making it easy to integrate into CI pipelines.

```bash
drift check --format json
```

| Exit Code | Meaning |
|-----------|---------|
| `0` | All dependencies are Safe or Watch |
| `1` | Error (e.g. missing `package.json`) |
| `2` | One or more Risk or Dead packages found |

---

## Environment Variables

| Variable | Purpose |
|----------|---------|
| `GITHUB_TOKEN` | GitHub API authentication (5,000 req/h vs. 60 req/h unauthenticated) |
| `DRIFT_LOG` | Log level: `debug`, `info`, `warn` |
| `NO_COLOR` | Disable colored terminal output |

---

## Tech Stack

Built with Rust using: `clap v4`, `tokio`, `reqwest`, `serde_json`, `comfy-table`, `colored`, `thiserror`, and the [OSV API](https://osv.dev/).

---

## Roadmap

**v0.2**

- `drift suggest` — recommend replacement packages for Dead/Risk dependencies
- `drift watch` — monitor dependencies continuously and alert on changes
- Local caching to reduce API calls and improve performance
- Multi-ecosystem support (Cargo, PyPI, Go modules)

---

## Contributing

Contributions are welcome. Please open an issue before submitting a pull request for significant changes.

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/your-feature`)
3. Commit your changes
4. Open a pull request

---

## License

MIT — see [LICENSE](LICENSE) for details.
