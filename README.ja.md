[English](README.md) | [한국어](README.ko.md) | [中文](README.zh.md) | **日本語**

# drift

[![Rust](https://img.shields.io/badge/built%20with-Rust-orange)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue)](LICENSE)
[![Build Status](https://img.shields.io/badge/build-passing-brightgreen)]()

**依存関係が死ぬ前に、知らせます。**

Drift は Rust 製の CLI ツールです。GitHub、npm、OSV から得たシグナルを分析し、プロジェクトの依存関係の健全性を 0〜100 でスコアリングします。廃れつつあるライブラリが問題になる前に警告します。

Drift は `package.json` をスキャンし、複数のソースからリアルタイムデータを取得して、各依存関係をリスクグレードに分類します。アップグレード、代替ライブラリへの移行、技術的負債について、根拠のある意思決定を下せるようになります。

---

## なぜ Drift なのか？

**あなたのプロジェクトはオープンソースライブラリに依存しています——それらはまだ生きていますか？**

- オープンソースメンテナーの60%がバーンアウトを経験しています（Tidelift 2024）。ライブラリは静かに放棄されます。
- `npm audit` はセキュリティ脆弱性しかチェックしません。ライブラリが死にかけているかどうかは教えてくれません。
- 気づいた時には、マイグレーションコストはすでに膨らんでいます。

Drift はたった一つの問いに答えます：**「このライブラリは6ヶ月後もメンテナンスされているか？」**

**こんな時に使えます：**

- **新規プロジェクト開始時** — 依存しようとしているライブラリが健全かを事前に確認
- **定期メンテナンス** — プロジェクトを定期的にスキャンし、衰退する依存関係を早期に検出
- **技術的負債レビュー** — `--format json` でレポートを生成し、マイグレーション提案の根拠に
- **CI パイプライン** — Risk または Dead の依存関係が検出された場合、自動的にビルドを失敗させる

---

## 出力例

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

## インストール

```bash
git clone https://github.com/calintzy/drift.git
cd drift
cargo build --release
```

ビルド後のバイナリは `./target/release/drift` に配置されます。

---

## 使い方

```bash
drift check                    # すべての依存関係をスキャン
drift check axios lodash       # 特定のパッケージを確認
drift check --format json      # CI 向けに JSON 形式で出力
drift check --include-dev      # devDependencies も含める
drift check --verbose          # シグナルスコアの詳細を表示
```

### GitHub トークン

トークンなしの場合、GitHub API のレート制限は 60 リクエスト/時間です。トークンを設定すると、5,000 リクエスト/時間まで利用できます。

```bash
export GITHUB_TOKEN=ghp_your_token_here
drift check
```

---

## スコアリングの仕組み

各依存関係は、7 つの独立したシグナルの加重平均によって 0〜100 でスコアリングされます。

| シグナル | 重み | データソース |
|----------|------|--------------|
| 最終コミット日 | 20% | GitHub API |
| リリース頻度 | 15% | GitHub Releases |
| メンテナー数 | 15% | GitHub Contributors |
| Issue 対応時間 | 15% | GitHub Issues |
| ダウンロード傾向 | 15% | npm Registry |
| CVE 履歴 | 10% | OSV API |
| コミュニティ（スター数 + PR マージ率） | 10% | GitHub API |

**スコアリングのルール：**

- 各シグナルは独立して 0〜100 でスコアリングされます
- API 障害によりシグナルが取得できない場合、残りの重みが再正規化されます（フェイルオープン方式）
- 有効なスコアには最低 2 つのシグナルが必要です
- 非推奨パッケージは自動的に Dead グレードが割り当てられます
- アーカイブ済みリポジトリは最終コミットスコアが強制的に 0 になります

---

## リスクグレード

| スコア | グレード | 意味 |
|--------|----------|------|
| 80〜100 | 🟢 Safe | 健全で活発にメンテナンスされている |
| 60〜79 | 🟡 Watch | 活動が鈍化しているサイン |
| 40〜59 | 🟠 Risk | 代替ライブラリの検討を推奨 |
| 0〜39 | 🔴 Dead | 直ちに移行が必要 |

---

## CI 連携

Risk または Dead のパッケージが見つかった場合、Drift は終了コード `2` を返します。CI パイプラインへの組み込みが容易です。

```bash
drift check --format json
```

| 終了コード | 意味 |
|------------|------|
| `0` | すべての依存関係が Safe または Watch |
| `1` | エラー（例：`package.json` が見つからない） |
| `2` | Risk または Dead のパッケージが 1 つ以上存在する |

---

## 環境変数

| 変数 | 用途 |
|------|------|
| `GITHUB_TOKEN` | GitHub API 認証（未認証: 60 リクエスト/時間、認証済み: 5,000 リクエスト/時間） |
| `DRIFT_LOG` | ログレベル：`debug`、`info`、`warn` |
| `NO_COLOR` | ターミナルのカラー出力を無効化 |

---

## 技術スタック

Rust で構築。使用クレート：`clap v4`、`tokio`、`reqwest`、`serde_json`、`comfy-table`、`colored`、`thiserror`、および [OSV API](https://osv.dev/)。

---

## ロードマップ

**v0.2**

- `drift suggest` — Dead/Risk な依存関係に対する代替パッケージの提案
- `drift watch` — 依存関係を継続的に監視し、変化をアラート通知
- API 呼び出しを削減しパフォーマンスを向上させるローカルキャッシュ
- マルチエコシステム対応（Cargo、PyPI、Go modules）

---

## コントリビューション

コントリビューションを歓迎します。大きな変更を加える場合は、プルリクエストを送る前に Issue を作成してください。

1. リポジトリをフォークする
2. フィーチャーブランチを作成する（`git checkout -b feature/your-feature`）
3. 変更をコミットする
4. プルリクエストを開く

---

## ライセンス

MIT — 詳細は [LICENSE](LICENSE) を参照してください。
