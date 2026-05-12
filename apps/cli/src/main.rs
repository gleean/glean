//! pds：端侧文档解析 CLI

use clap::{Parser, Subcommand};
use parser_core::{
    MarkdownRenderOptions, ParseError, ParseOptions, PipelineConfig, parse_path,
};
use std::fs::File;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser)]
#[command(name = "pds", version, about = "Private Doc Stack — 本地多格式解析")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 解析文件或目录，输出 Markdown（默认带语义标签与 HTML 元数据）
    Parse {
        /// 输入路径（文件；目录需配合 --code）
        input: PathBuf,
        /// 将目录视为源码树，按符号递归切分（tree-sitter）
        #[arg(long)]
        code: bool,
        /// 关闭相邻重复块去重
        #[arg(long)]
        no_dedupe: bool,
        /// 输出文件（默认 stdout）
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// 纯正文：不输出 `{ #... }` 与 `<!-- ... -->`，仅块内 Markdown
        #[arg(long, conflicts_with_all = ["no_tags", "no_meta"])]
        plain: bool,
        /// 不输出语义标签行 `{ #paragraph ... }`
        #[arg(long)]
        no_tags: bool,
        /// 不输出 HTML 元数据注释 `<!-- source=... -->`
        #[arg(long)]
        no_meta: bool,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Commands::Parse {
            input,
            code,
            no_dedupe,
            output,
            plain,
            no_tags,
            no_meta,
        } => match run_parse(input, code, no_dedupe, output, plain, no_tags, no_meta) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("pds: {e}");
                ExitCode::from(1)
            }
        },
    }
}

fn run_parse(
    input: PathBuf,
    code: bool,
    no_dedupe: bool,
    output: Option<PathBuf>,
    plain: bool,
    no_tags: bool,
    no_meta: bool,
) -> Result<(), ParseError> {
    let options = ParseOptions {
        pipeline: PipelineConfig {
            dedupe_adjacent_identical: !no_dedupe,
            normalize_chinese: false,
        },
        code_analysis: code,
    };

    let render = if plain {
        MarkdownRenderOptions::plain_body_only()
    } else {
        MarkdownRenderOptions {
            include_semantic_tags: !no_tags,
            include_html_meta: !no_meta,
        }
    };

    let doc = parse_path(&input, &options)?;
    let md = doc.to_markdown(&render);

    match output {
        Some(path) => {
            let mut f = File::create(&path)?;
            f.write_all(md.as_bytes())?;
        }
        None => {
            let mut out = io::stdout().lock();
            out.write_all(md.as_bytes())?;
        }
    }
    Ok(())
}
