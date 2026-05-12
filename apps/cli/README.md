# @repo/pds（`pds` CLI）

`pds` 是 **private-doc-stack** 的本地命令行入口，对 PDF、DOCX、Markdown、纯文本与多种源码调用 [`parser-core`](../../packages/parser-core)，将结果输出为 **Enhanced Markdown**（带 `#table` 等语义标签与 `<!-- meta -->` 注释）。

Rust 实现位于本目录；pnpm 包名 **`@repo/pds`**，通过 `bin/pds.cjs` 在仓库根目录触发 `cargo run -p pds`。

---

## 前置条件

- **Rust**：`cargo`、`rustc`（workspace 在仓库根 [`Cargo.toml`](../../Cargo.toml)）
- **pnpm（可选）**：在 monorepo 根安装依赖后，可通过 `pnpm run pds` 调用；无需单独全局安装 `pds` 二进制

---

## 安装与运行方式

### 1. 开发调试（推荐，在仓库根目录）

```bash
cargo run -p pds -- parse <INPUT> [OPTIONS]
```

### 2. 发布用本地安装

```bash
cargo install --path apps/cli
pds parse <INPUT> [OPTIONS]
```

### 3. 通过 pnpm（根目录已依赖 `@repo/pds` 时）

```bash
pnpm install
pnpm run pds -- parse <INPUT> [OPTIONS]
```

说明：`pnpm run … --` 后面的参数会经 `bin/pds.cjs` 转发给 `cargo run`，脚本内会去掉 pnpm 多传的一层 `--`。

### 4. 仅构建二进制

```bash
cargo build --release -p pds
# 可执行文件一般在 target/release/pds
```

---

## 命令：`pds parse`

| 项 | 说明 |
|----|------|
| `<INPUT>` | 文件路径，或目录（目录见下） |
| `--code` | 将 **目录** 视为源码树，递归解析 `.rs` / `.ts` / `.py` 等并按符号切片（需与目录配合；纯文件路径按扩展名自动选解析器） |
| `--no-dedupe` | 关闭解析管线中的「相邻重复块去重」 |
| `-o`, `--output <PATH>` | 写入文件；默认打印到 **stdout** |
| `--plain` | **纯正文**：不输出 `{ #... }` 语义标签行，也不输出 `<!-- ... -->` HTML 元数据 |
| `--no-tags` | 仅关闭语义标签行（可与 `--no-meta` 组合；与 `--plain` 互斥） |
| `--no-meta` | 仅关闭 HTML 元数据注释（可与 `--no-tags` 组合；与 `--plain` 互斥） |

目录解析：若 `<INPUT>` 为目录且 **未** 加 `--code`，`parser-core` 会返回错误（与库行为一致）；源码仓库请使用 `pds parse ./src --code` 这类形式。

**库用法**：`ParsedDocument::to_markdown(&MarkdownRenderOptions { include_semantic_tags, include_html_meta })`，或 `to_enhanced_markdown()` 表示二者均开启（默认）。

---

## 示例

```bash
# Markdown / 文本
cargo run -p pds -- parse ./notes.md

# PDF / DOCX（默认带 `{ #... }` 与 `<!-- ... -->`）
cargo run -p pds -- parse ./report.pdf -o ./full.md
# 同上，只要正文（不要标签与 HTML 注释）
cargo run -p pds -- parse ./report.pdf -o ./plain.md --plain

# 代码目录（tree-sitter 分块）
cargo run -p pds -- parse ./packages/parser-core/src --code -o ./chunks.md
```

---

## 与 `parser-core` 的关系

- 解析结果经 **`ParsedDocument::to_markdown`**（或默认的 `to_enhanced_markdown`）序列化。
- 本 CLI 只负责参数解析、错误打印与读写 stdout/文件。
- 支持格式与验收约定见 [`packages/parser-core/README.md`](../../packages/parser-core/README.md)。

---

## 许可证

与仓库根目录 `LICENSE` 保持一致（若未单独声明）。
