use std::path::Path;

const PREVIEW_CHARS: usize = 1200;

fn build_preview(text: &str, max_chars: usize) -> String {
    let normalized = text.replace('\r', "");
    let total = normalized.chars().count();
    if total <= max_chars {
        return normalized;
    }
    let head = normalized.chars().take(max_chars).collect::<String>();
    format!("{head}\n\n...[truncated, total_chars={total}]")
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let file_path = args.next().ok_or_else(|| {
        "usage: cargo run --example undoc_probe -- <absolute-path-to-docx/xlsx/pptx>".to_string()
    })?;

    if args.next().is_some() {
        return Err("only one file path argument is supported".into());
    }

    let path = Path::new(&file_path);
    if !path.is_absolute() {
        return Err("absolute path is required".into());
    }
    if !path.exists() {
        return Err(format!("file not found: {}", path.display()).into());
    }

    let detected = undoc::detect_format_from_path(path)?;
    println!("detected_format={detected:?}");

    let text = undoc::extract_text(path)?;
    println!("total_chars={}", text.chars().count());
    println!("----- preview -----");
    println!("{}", build_preview(&text, PREVIEW_CHARS));

    Ok(())
}
