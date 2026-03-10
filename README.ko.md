[English](README.md) | **한국어** | [中文](README.zh.md) | [日本語](README.ja.md)

# drift

[![Rust](https://img.shields.io/badge/built%20with-Rust-orange)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue)](LICENSE)
[![Build Status](https://img.shields.io/badge/build-passing-brightgreen)]()

**의존성이 죽기 전에 알려드립니다.**

Drift는 GitHub, npm, OSV의 데이터를 분석해 프로젝트 의존성의 건강 점수(0–100)를 산출하는 Rust CLI 도구입니다. 문제가 생기기 전에 죽어가는 라이브러리를 미리 경고해 드립니다.

Drift는 `package.json`을 스캔하고 여러 소스에서 실시간 데이터를 가져와 각 의존성을 위험 등급으로 분류합니다. 업그레이드, 대체, 기술 부채에 대해 근거 있는 결정을 내릴 수 있습니다.

---

## 왜 Drift인가?

**여러분의 프로젝트는 오픈소스 라이브러리에 의존합니다 — 그 라이브러리들이 아직 살아있나요?**

- 오픈소스 메인테이너의 60%가 번아웃을 경험합니다(Tidelift 2024). 라이브러리는 조용히 방치됩니다.
- `npm audit`은 보안 취약점만 체크합니다. 라이브러리가 죽어가는지는 알려주지 않습니다.
- 알아챘을 때는 이미 마이그레이션 비용이 커진 뒤입니다.

Drift는 단 하나의 질문에 답합니다: **"이 라이브러리가 6개월 뒤에도 유지보수 될까?"**

**이럴 때 쓰세요:**

- **새 프로젝트 시작** — 의존하려는 라이브러리가 건강한지 미리 확인
- **정기 점검** — 프로젝트를 주기적으로 스캔하여 쇠퇴하는 의존성을 조기에 포착
- **기술 부채 리뷰** — `--format json`으로 보고서를 생성해 마이그레이션 제안의 근거 확보
- **CI 파이프라인** — Risk 또는 Dead 의존성이 발견되면 자동으로 빌드 실패 처리

---

## 출력 예시

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

## 설치

```bash
git clone https://github.com/calintzy/drift.git
cd drift
cargo build --release
```

빌드된 바이너리는 `./target/release/drift` 경로에서 확인할 수 있습니다.

---

## 사용법

```bash
drift check                    # Scan all dependencies
drift check axios lodash       # Check specific packages
drift check --format json      # JSON output for CI
drift check --include-dev      # Include devDependencies
drift check --verbose          # Show detailed signal scores
```

### GitHub 토큰

토큰 없이 사용하면 GitHub API 요청이 시간당 60회로 제한됩니다. 토큰을 설정하면 시간당 5,000회까지 사용할 수 있습니다.

```bash
export GITHUB_TOKEN=ghp_your_token_here
drift check
```

---

## 점수 산정 방식

각 의존성은 7가지 독립 신호의 가중 평균으로 0–100점이 매겨집니다:

| 신호 | 가중치 | 출처 |
|------|--------|------|
| 마지막 커밋 | 20% | GitHub API |
| 릴리스 빈도 | 15% | GitHub Releases |
| 메인테이너 수 | 15% | GitHub Contributors |
| 이슈 응답 시간 | 15% | GitHub Issues |
| 다운로드 추세 | 15% | npm Registry |
| CVE 이력 | 10% | OSV API |
| 커뮤니티 (Stars + PR 병합률) | 10% | GitHub API |

**점수 산정 규칙:**

- 각 신호는 독립적으로 0–100점으로 평가됩니다
- API 오류로 신호를 가져올 수 없는 경우, 나머지 가중치를 재정규화합니다 (fail-open 방식)
- 유효한 점수를 산출하려면 최소 2개의 신호가 필요합니다
- deprecated 패키지는 자동으로 Dead 등급을 부여받습니다
- 아카이브된 저장소는 마지막 커밋 점수가 강제로 0점 처리됩니다

---

## 위험 등급

| 점수 | 등급 | 의미 |
|------|------|------|
| 80–100 | 🟢 Safe | 건강하고 활발하게 유지보수 중 |
| 60–79 | 🟡 Watch | 활동이 둔화되는 징후 |
| 40–59 | 🟠 Risk | 대안 검토 권장 |
| 0–39 | 🔴 Dead | 즉시 마이그레이션 필요 |

---

## CI 연동

Drift는 Risk 또는 Dead 패키지가 발견되면 종료 코드 `2`를 반환하므로 CI 파이프라인에 간편하게 통합할 수 있습니다.

```bash
drift check --format json
```

| 종료 코드 | 의미 |
|-----------|------|
| `0` | 모든 의존성이 Safe 또는 Watch 등급 |
| `1` | 오류 발생 (예: `package.json` 없음) |
| `2` | Risk 또는 Dead 패키지가 하나 이상 발견됨 |

---

## 환경 변수

| 변수 | 용도 |
|------|------|
| `GITHUB_TOKEN` | GitHub API 인증 (인증 시 5,000 req/h, 미인증 시 60 req/h) |
| `DRIFT_LOG` | 로그 레벨: `debug`, `info`, `warn` |
| `NO_COLOR` | 터미널 색상 출력 비활성화 |

---

## 기술 스택

Rust로 제작되었으며 다음을 사용합니다: `clap v4`, `tokio`, `reqwest`, `serde_json`, `comfy-table`, `colored`, `thiserror`, 그리고 [OSV API](https://osv.dev/).

---

## 로드맵

**v0.2**

- `drift suggest` — Dead/Risk 의존성에 대한 대체 패키지 추천
- `drift watch` — 의존성을 지속적으로 모니터링하고 변경 사항 알림
- API 호출 횟수 감소 및 성능 향상을 위한 로컬 캐싱
- 멀티 에코시스템 지원 (Cargo, PyPI, Go modules)

---

## 기여하기

기여를 환영합니다. 큰 변경 사항을 제출하기 전에 먼저 이슈를 열어 주세요.

1. 저장소를 Fork합니다
2. 기능 브랜치를 생성합니다 (`git checkout -b feature/your-feature`)
3. 변경 사항을 커밋합니다
4. Pull request를 엽니다

---

## 라이선스

MIT — 자세한 내용은 [LICENSE](LICENSE)를 참고하세요.
