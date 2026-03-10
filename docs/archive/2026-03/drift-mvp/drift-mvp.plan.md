# Drift MVP Planning Document

> **Summary**: 프로젝트 의존성의 "생존 확률"을 점수화하고 죽어가는 라이브러리를 사전 경고하는 Rust CLI 도구
>
> **Project**: Drift
> **Version**: 0.1.0
> **Author**: Ryan
> **Date**: 2026-03-10
> **Status**: Draft

---

## Executive Summary

| Perspective | Content |
|-------------|---------|
| **Problem** | 개발자는 의존 라이브러리가 방치·사망 상태인지 사후에야 알게 되며, npm audit은 보안 취약점만 체크하고 건강 상태는 무시한다 |
| **Solution** | GitHub API + npm Registry API 데이터를 결합한 7개 신호 기반 건강 점수(0~100) 산정 CLI를 Rust로 구현한다 |
| **Function/UX Effect** | `drift check` 한 줄로 프로젝트 전체 의존성의 건강 점수·리스크 등급·핵심 신호를 컬러 테이블로 즉시 확인할 수 있다 |
| **Core Value** | "사후 대응"에서 "사전 예방"으로 전환 — 의존성이 죽기 전에 경고하여 마이그레이션 비용과 공급망 리스크를 최소화한다 |

---

## 1. Overview

### 1.1 Purpose

프로젝트 의존성의 건강 상태를 점수화(0~100)하여 방치·사망 위험이 있는 라이브러리를 사전에 식별하고 경고하는 CLI 도구를 개발한다.

### 1.2 Background

- 오픈소스 메인테이너 60%가 번아웃 경험 (Tidelift 2024 Survey)
- 매년 수천 개의 npm 패키지가 방치됨 — `event-stream` 사태(2018), `colors.js` 사태(2022) 반복
- `npm audit`은 보안 취약점만 체크하고 라이브러리의 건강 상태는 무시
- ICSE 2026에서 "Abandabot" 논문이 발표되었으나 실용 CLI 도구는 아직 없어 선점 기회 존재

### 1.3 Related Documents

- 기획서: `기획서.md`

---

## 2. Scope

### 2.1 In Scope (MVP v0.1)

- [x] `drift check` — package.json 파싱 후 전체 의존성 건강 점수 계산
- [x] `drift check <pkg1> <pkg2>` — 특정 패키지만 건강 점수 체크
- [x] GitHub REST API 연동 (마지막 커밋, 메인테이너 수, 이슈 응답율)
- [x] npm Registry API 연동 (다운로드 추세)
- [x] 7개 신호 가중치 기반 건강 점수 모델 (0~100)
- [x] 리스크 등급 분류 (Safe / Watch / Risk / Dead)
- [x] 터미널 컬러 테이블 출력
- [x] `--format json` 옵션 (CI 통합 기본 지원)

### 2.2 Out of Scope (v0.2+)

- `drift suggest` — 대안 라이브러리 추천
- `drift watch` — CI 지속 모니터링 모드
- `drift compare` — 라이브러리 비교
- 캐싱 시스템 (SQLite)
- pip, cargo, go.mod 등 다중 생태계 지원
- 웹 대시보드, GitHub Action
- ML 기반 예측 모델

---

## 3. Requirements

### 3.1 Functional Requirements

| ID | Requirement | Priority | Status |
|----|-------------|----------|--------|
| FR-01 | package.json 파싱하여 dependencies, devDependencies 목록 추출 | High | Pending |
| FR-02 | GitHub REST API로 리포지토리 메타데이터 수집 (마지막 커밋, 컨트리뷰터 수) | High | Pending |
| FR-03 | GitHub Issues API로 이슈 응답율 계산 (오픈→첫 응답 시간) | High | Pending |
| FR-04 | npm Registry API로 주간 다운로드 수 및 6개월 추세 수집 | High | Pending |
| FR-05 | 7개 신호 가중치 모델 기반 건강 점수(0~100) 계산 | High | Pending |
| FR-06 | 점수 기반 리스크 등급 분류 (≥80 Safe, ≥60 Watch, ≥40 Risk, <40 Dead) | High | Pending |
| FR-07 | 터미널 컬러 테이블 출력 (등급별 색상 구분) | Medium | Pending |
| FR-08 | `drift check` 전체 스캔 및 `drift check <pkg...>` 개별 스캔 | High | Pending |
| FR-09 | `--format json` 옵션으로 JSON 출력 지원 | Medium | Pending |
| FR-10 | Summary 행 출력 (총 의존성 수, 등급별 개수, 액션 필요 수) | Medium | Pending |
| FR-11 | GitHub API 토큰 환경변수(`GITHUB_TOKEN`) 지원 (rate limit 대응) | High | Pending |
| FR-12 | CVE 이력 조회 (OSV API) 및 점수 반영 | Medium | Pending |

### 3.2 Non-Functional Requirements

| Category | Criteria | Measurement Method |
|----------|----------|-------------------|
| Performance | 50개 의존성 체크 시 30초 이내 완료 | 실측 벤치마크 |
| Performance | API 호출 병렬화 (concurrent requests) | tokio 비동기 처리 확인 |
| Reliability | API 실패 시 graceful degradation (부분 점수 표시) | 에러 시나리오 테스트 |
| Portability | Linux, macOS, Windows 크로스 플랫폼 바이너리 | CI 빌드 매트릭스 |
| Usability | 설치 후 추가 설정 없이 `drift check` 즉시 실행 가능 | 사용자 테스트 |

---

## 4. Success Criteria

### 4.1 Definition of Done

- [ ] `drift check` 명령어로 package.json의 모든 의존성 건강 점수 출력
- [ ] 7개 신호 모두 점수에 반영
- [ ] 컬러 터미널 테이블 정상 렌더링
- [ ] JSON 출력 모드 동작
- [ ] 단위 테스트 작성 및 통과
- [ ] README 작성 (설치법, 사용법, 점수 산정 방식)

### 4.2 Quality Criteria

- [ ] 테스트 커버리지 80% 이상
- [ ] `cargo clippy` 경고 0개
- [ ] `cargo fmt` 적용
- [ ] `cargo build --release` 성공

---

## 5. Risks and Mitigation

| Risk | Impact | Likelihood | Mitigation |
|------|--------|------------|------------|
| GitHub API rate limit (비인증 60req/h) | High | High | `GITHUB_TOKEN` 지원(5000req/h), conditional requests(ETag), 병렬 호출 최적화 |
| npm Registry API 응답 지연/실패 | Medium | Medium | 타임아웃 설정, 부분 결과 표시, 재시도 로직 |
| 건강 점수 정확도 논란 | Medium | High | 점수 산정 공식 투명 공개, `--verbose`로 개별 신호 점수 노출 |
| npm 패키지→GitHub 리포 매핑 실패 | Medium | Medium | `repository` 필드 파싱, 매핑 실패 시 해당 신호 제외 후 부분 점수 |
| 크로스 플랫폼 빌드 이슈 | Low | Medium | GitHub Actions CI 매트릭스 (linux/macos/windows) |

---

## 6. Architecture Considerations

### 6.1 Project Level Selection

| Level | Characteristics | Recommended For | Selected |
|-------|-----------------|-----------------|:--------:|
| **Starter** | 단순 구조 | 정적 사이트, 포트폴리오 | ☐ |
| **Dynamic** | 기능 기반 모듈, API 통합 | CLI 도구, 웹앱 MVP | ☑ |
| **Enterprise** | 레이어 분리, DI, 마이크로서비스 | 대규모 시스템 | ☐ |

### 6.2 Key Architectural Decisions

| Decision | Options | Selected | Rationale |
|----------|---------|----------|-----------|
| 언어 | Rust / TypeScript | **Rust** | 빠른 실행 속도, 단일 바이너리 배포, 크로스 플랫폼 |
| CLI 프레임워크 | clap / structopt | **clap v4** | Rust CLI 표준, derive 매크로 지원 |
| HTTP 클라이언트 | reqwest / ureq | **reqwest** | 비동기 지원, Rust 생태계 표준 |
| 비동기 런타임 | tokio / async-std | **tokio** | reqwest 기본 런타임, 생태계 최대 |
| 테이블 출력 | comfy-table / tabled | **comfy-table** | 컬러 지원, 커스텀 스타일링 용이 |
| 컬러 출력 | colored / owo-colors | **colored** | 직관적 API, 널리 사용 |
| JSON 직렬화 | serde_json | **serde_json** | Rust 직렬화 표준 |
| 테스트 | cargo test + mockito | **cargo test + wiremock** | 비동기 HTTP 모킹 지원 |

### 6.3 Clean Architecture Approach

```
Selected Level: Dynamic

Folder Structure Preview:
┌─────────────────────────────────────────────────────┐
│ drift/                                              │
│ ├── Cargo.toml                                      │
│ ├── src/                                            │
│ │   ├── main.rs              # 진입점, CLI 설정      │
│ │   ├── cli.rs               # clap 명령어 정의      │
│ │   ├── scorer/                                     │
│ │   │   ├── mod.rs           # 건강 점수 엔진        │
│ │   │   ├── model.rs         # 점수 모델/가중치       │
│ │   │   └── signals.rs       # 7개 신호 정의         │
│ │   ├── providers/                                  │
│ │   │   ├── mod.rs           # 프로바이더 트레잇      │
│ │   │   ├── github.rs        # GitHub API 클라이언트  │
│ │   │   ├── npm.rs           # npm Registry 클라이언트│
│ │   │   └── osv.rs           # OSV API (CVE 조회)    │
│ │   ├── parsers/                                    │
│ │   │   ├── mod.rs           # 파서 트레잇           │
│ │   │   └── package_json.rs  # package.json 파서     │
│ │   ├── output/                                     │
│ │   │   ├── mod.rs           # 출력 포맷터           │
│ │   │   ├── table.rs         # 터미널 테이블          │
│ │   │   └── json.rs          # JSON 출력             │
│ │   └── types.rs             # 공통 타입 정의         │
│ └── tests/                                          │
│     ├── integration/         # 통합 테스트            │
│     └── fixtures/            # 테스트 데이터          │
└─────────────────────────────────────────────────────┘
```

---

## 7. Convention Prerequisites

### 7.1 Existing Project Conventions

- [ ] `CLAUDE.md` has coding conventions section
- [ ] Rust edition 2021
- [ ] `rustfmt.toml` 설정
- [ ] `clippy.toml` 설정

### 7.2 Conventions to Define/Verify

| Category | Current State | To Define | Priority |
|----------|---------------|-----------|:--------:|
| **Naming** | Missing | snake_case (함수/변수), PascalCase (타입/구조체) | High |
| **Folder structure** | Missing | 위 6.3 구조 준수 | High |
| **Error handling** | Missing | `thiserror` 기반 커스텀 에러 타입, `anyhow` for main | High |
| **Import order** | Missing | std → external → crate → self | Medium |
| **로깅** | Missing | `tracing` 크레이트 사용, `--verbose` 플래그 | Medium |

### 7.3 Environment Variables Needed

| Variable | Purpose | Scope | To Be Created |
|----------|---------|-------|:-------------:|
| `GITHUB_TOKEN` | GitHub API 인증 (rate limit 확장) | Runtime | ☑ |
| `DRIFT_LOG` | 로그 레벨 설정 (debug/info/warn) | Runtime | ☑ |
| `NO_COLOR` | 컬러 출력 비활성화 (표준 규약) | Runtime | ☑ |

---

## 8. 건강 점수 모델 상세

### 8.1 신호별 가중치 및 계산 방식

| Signal | Weight | 100점 기준 | 0점 기준 | Data Source |
|--------|--------|-----------|---------|-------------|
| 마지막 커밋 | 20% | 30일 이내 | 365일+ 또는 archived | GitHub API |
| 릴리스 주기 | 15% | 월 1회 이상 규칙적 | 12개월+ 릴리스 없음 | GitHub Releases API |
| 메인테이너 수 | 15% | 5명+ 활성 커미터 | 1명 (bus factor=1) | GitHub Contributors API |
| 이슈 응답율 | 15% | 중앙값 24시간 이내 | 응답 없는 이슈 70%+ | GitHub Issues API |
| 다운로드 추세 | 15% | 6개월 상승 추세 | 6개월 하락 추세 50%+ | npm Registry API |
| CVE 이력 | 10% | CVE 없음 또는 즉시 패치 | 미패치 CVE 존재 | OSV API |
| 커뮤니티 신호 | 10% | Star 상승, PR 머지율 높음 | Star 정체, PR 방치 | GitHub API |

### 8.2 리스크 등급

| Score Range | Grade | Label | Color | Action |
|-------------|-------|-------|-------|--------|
| 80~100 | A | 🟢 Safe | Green | 유지 |
| 60~79 | B | 🟡 Watch | Yellow | 주시 필요 |
| 40~59 | C | 🟠 Risk | Orange | 대안 검토 권장 |
| 0~39 | D | 🔴 Dead | Red | 즉시 마이그레이션 권장 |

---

## 9. Next Steps

1. [ ] Design 문서 작성 (`drift-mvp.design.md`)
2. [ ] Rust 프로젝트 초기화 (`cargo init`)
3. [ ] GitHub API / npm API 프로토타입 테스트
4. [ ] 구현 시작

---

## Version History

| Version | Date | Changes | Author |
|---------|------|---------|--------|
| 0.1 | 2026-03-10 | Initial draft based on 기획서.md | Ryan |
