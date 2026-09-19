//! Mimo OCR Phase 0 CLI (`mimo`).

use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};
use mimo_ocr_capture::{capture_logical_region, capture_monitor, capture_region, list_monitors};
use mimo_ocr_core::{cer, wer, CancellationToken, OcrImage, OcrOptions, QualityMode};
use mimo_ocr_engines::{discover_tesseract, resolve_tessdata, EngineRegistry};

#[derive(Parser)]
#[command(name = "mimo", version, about = "Mimo OCR — Phase 0 validation CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List registered OCR engines
    Engines,
    /// OCR an image file
    Ocr {
        image: PathBuf,
        #[arg(long, default_value = "auto")]
        engine: String,
        #[arg(long, default_value = "tur,eng")]
        langs: String,
        #[arg(long, value_parser = ["fast", "balanced", "accurate"], default_value = "accurate")]
        quality: String,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        model_dir: Option<PathBuf>,
    },
    /// List monitors (DPI / scale probe)
    Monitors,
    /// Capture primary monitor or a region to PNG
    Capture {
        #[arg(short, long)]
        out: PathBuf,
        #[arg(long, default_value_t = 0)]
        monitor: usize,
        /// Region as x,y,w,h in physical pixels
        #[arg(long)]
        region: Option<String>,
        /// Region as x,y,w,h in logical DIPs
        #[arg(long)]
        logical: Option<String>,
    },
    /// Benchmark engines on a dataset directory (images + optional .txt expected)
    Bench {
        dataset: PathBuf,
        #[arg(long, default_value = "auto")]
        engine: String,
        #[arg(long, default_value = "tur,eng")]
        langs: String,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        model_dir: Option<PathBuf>,
    },
    /// Batch OCR a folder; writes <image>.ocr.txt beside each file
    Batch {
        dir: PathBuf,
        #[arg(long, default_value = "tur,eng")]
        langs: String,
        #[arg(long, value_parser = ["fast", "balanced", "accurate"], default_value = "balanced")]
        quality: String,
        #[arg(long)]
        out_dir: Option<PathBuf>,
    },
    /// Environment / packaging probe
    Doctor,
}

fn parse_quality(s: &str) -> QualityMode {
    match s {
        "fast" => QualityMode::Fast,
        "accurate" => QualityMode::Accurate,
        _ => QualityMode::Balanced,
    }
}

fn parse_langs(s: &str) -> Vec<String> {
    s.split(',')
        .map(|x| x.trim().to_string())
        .filter(|x| !x.is_empty())
        .collect()
}

fn parse_region(s: &str) -> anyhow::Result<(u32, u32, u32, u32)> {
    let parts: Vec<u32> = s
        .split(',')
        .map(|p| p.trim().parse::<u32>())
        .collect::<Result<_, _>>()?;
    if parts.len() != 4 {
        anyhow::bail!("region must be x,y,w,h");
    }
    Ok((parts[0], parts[1], parts[2], parts[3]))
}

fn build_registry(engine_filter: &str) -> EngineRegistry {
    let _ = engine_filter;
    EngineRegistry::phase0_default()
}

fn pick_engine_id(reg: &EngineRegistry, requested: &str) -> anyhow::Result<&'static str> {
    if requested != "auto" && requested != "all" {
        return Ok(reg.get(requested)?.id());
    }
    // Prefer Tesseract for quality when present; Windows OCR as fallback accelerator.
    if reg.ids().contains(&"tesseract-cli") {
        return Ok("tesseract-cli");
    }
    if reg.ids().contains(&"windows-ocr") {
        return Ok("windows-ocr");
    }
    reg.ids()
        .first()
        .copied()
        .ok_or_else(|| anyhow::anyhow!("no engines registered"))
}

fn cmd_engines() -> anyhow::Result<()> {
    let reg = EngineRegistry::phase0_default();
    println!("Registered engines:");
    for id in reg.ids() {
        let e = reg.get(id)?;
        let caps = e.capabilities();
        let langs: Vec<String> = e
            .supported_languages()
            .iter()
            .map(|l| l.code.clone())
            .collect();
        println!(
            "  - {} ({}) local={} word_boxes={} langs={}",
            e.id(),
            e.display_name(),
            caps.local,
            caps.provides_word_boxes,
            langs.join(",")
        );
    }
    if let Ok(t) = discover_tesseract() {
        println!("tesseract binary: {}", t.display());
    }
    if let Some(td) = resolve_tessdata(None) {
        println!("tessdata: {}", td.display());
    }
    Ok(())
}

fn cmd_ocr(
    image: PathBuf,
    engine: String,
    langs: String,
    quality: String,
    json: bool,
    model_dir: Option<PathBuf>,
) -> anyhow::Result<()> {
    let img = OcrImage::from_path(&image)?;
    let options = OcrOptions {
        languages: parse_langs(&langs),
        quality: parse_quality(&quality),
        model_dir: model_dir.clone().or_else(|| resolve_tessdata(None)),
        ..Default::default()
    };
    let reg = build_registry(&engine);
    let id = pick_engine_id(&reg, &engine)?;
    let cancel = CancellationToken::new();
    let doc = reg.recognize(id, &img, &options, &cancel)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&doc)?);
    } else {
        println!("engine: {}", doc.engine);
        println!("language: {}", doc.language.clone().unwrap_or_default());
        println!("elapsed_ms: {}", doc.elapsed_ms);
        if let Some(c) = doc.mean_confidence() {
            println!("mean_confidence: {c:.3}");
        }
        println!("--- text ---");
        println!("{}", doc.plain_text);
    }
    Ok(())
}

fn cmd_monitors() -> anyhow::Result<()> {
    let monitors = list_monitors()?;
    println!(
        "{:<4} {:<20} {:>6} {:>6} {:>10} {:>10} {:>8} {:>8}",
        "idx", "name", "x", "y", "width", "height", "scale", "primary"
    );
    for m in monitors {
        println!(
            "{:<4} {:<20} {:>6} {:>6} {:>10} {:>10} {:>8.2} {:>8}",
            m.index, m.name, m.x, m.y, m.width, m.height, m.scale_factor, m.is_primary
        );
    }
    Ok(())
}

fn cmd_capture(
    out: PathBuf,
    monitor: usize,
    region: Option<String>,
    logical: Option<String>,
) -> anyhow::Result<()> {
    let img = if let Some(r) = logical {
        let (x, y, w, h) = parse_region(&r)?;
        capture_logical_region(monitor, x, y, w, h)?
    } else if let Some(r) = region {
        let (x, y, w, h) = parse_region(&r)?;
        capture_region(monitor, x, y, w, h)?
    } else {
        capture_monitor(monitor)?
    };
    if let Some(parent) = out.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    img.save_png(&out)?;
    println!(
        "saved {} ({}x{}, source={})",
        out.display(),
        img.width,
        img.height,
        img.source_name
    );
    Ok(())
}

fn list_images(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let Ok(rd) = std::fs::read_dir(dir) else {
        return files;
    };
    for e in rd.flatten() {
        let p = e.path();
        if let Some(ext) = p.extension().and_then(|s| s.to_str()) {
            let ext = ext.to_ascii_lowercase();
            if matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "bmp" | "tif" | "tiff") {
                files.push(p);
            }
        }
    }
    files.sort();
    files
}

fn cmd_bench(
    dataset: PathBuf,
    engine: String,
    langs: String,
    json: bool,
    model_dir: Option<PathBuf>,
) -> anyhow::Result<()> {
    let images = list_images(&dataset);
    if images.is_empty() {
        anyhow::bail!("no images in {}", dataset.display());
    }
    let options = OcrOptions {
        languages: parse_langs(&langs),
        model_dir: model_dir.clone().or_else(|| resolve_tessdata(None)),
        ..Default::default()
    };
    let reg = build_registry(&engine);
    let ids: Vec<&'static str> = if engine == "all" {
        reg.ids()
    } else {
        vec![pick_engine_id(&reg, &engine)?]
    };

    #[derive(serde::Serialize)]
    struct Row {
        image: String,
        engine: String,
        elapsed_ms: u64,
        mean_conf: Option<f32>,
        cer: Option<f32>,
        wer: Option<f32>,
        chars: usize,
        error: Option<String>,
    }

    let mut rows: Vec<Row> = Vec::new();
    for id in ids {
        for img_path in &images {
            let cancel = CancellationToken::new();
            let mut row = Row {
                image: img_path
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_default(),
                engine: id.to_string(),
                elapsed_ms: 0,
                mean_conf: None,
                cer: None,
                wer: None,
                chars: 0,
                error: None,
            };
            match OcrImage::from_path(img_path) {
                Ok(img) => match reg.recognize(id, &img, &options, &cancel) {
                    Ok(doc) => {
                        row.elapsed_ms = doc.elapsed_ms;
                        row.mean_conf = doc.mean_confidence();
                        row.chars = doc.plain_text.chars().count();
                        let exp = img_path.with_extension("txt");
                        if let Ok(reference) = std::fs::read_to_string(&exp) {
                            row.cer = Some(cer(&reference, &doc.plain_text));
                            row.wer = Some(wer(&reference, &doc.plain_text));
                        }
                    }
                    Err(e) => row.error = Some(e.to_string()),
                },
                Err(e) => row.error = Some(e.to_string()),
            }
            rows.push(row);
        }
    }

    if json {
        println!("{}", serde_json::to_string_pretty(&rows)?);
    } else {
        println!(
            "{:<28} {:<16} {:>8} {:>8} {:>8} {:>8} {:>8}",
            "image", "engine", "ms", "conf", "CER", "WER", "chars"
        );
        for r in &rows {
            let conf = r
                .mean_conf
                .map(|c| format!("{c:.3}"))
                .unwrap_or_else(|| "-".into());
            let c = r
                .cer
                .map(|c| format!("{c:.3}"))
                .unwrap_or_else(|| "-".into());
            let w = r
                .wer
                .map(|c| format!("{c:.3}"))
                .unwrap_or_else(|| "-".into());
            println!(
                "{:<28} {:<16} {:>8} {:>8} {:>8} {:>8} {:>8}",
                r.image, r.engine, r.elapsed_ms, conf, c, w, r.chars
            );
            if let Some(e) = &r.error {
                println!("    error: {e}");
            }
        }
        let mut engines: Vec<String> = rows.iter().map(|r| r.engine.clone()).collect();
        engines.sort();
        engines.dedup();
        for id in engines {
            let times: Vec<u64> = rows
                .iter()
                .filter(|r| r.engine == id)
                .map(|r| r.elapsed_ms)
                .collect();
            let ok = rows
                .iter()
                .filter(|r| r.engine == id && r.error.is_none())
                .count();
            let cers: Vec<f32> = rows
                .iter()
                .filter(|r| r.engine == id)
                .filter_map(|r| r.cer)
                .collect();
            let sum: u64 = times.iter().sum();
            let avg_cer = if cers.is_empty() {
                None
            } else {
                Some(cers.iter().sum::<f32>() / cers.len() as f32)
            };
            println!(
                "summary[{id}]: n={} ok={} avg_ms={:.1} min={} max={} avg_cer={}",
                times.len(),
                ok,
                if times.is_empty() {
                    0.0
                } else {
                    sum as f64 / times.len() as f64
                },
                times.iter().min().copied().unwrap_or(0),
                times.iter().max().copied().unwrap_or(0),
                avg_cer
                    .map(|c| format!("{c:.3}"))
                    .unwrap_or_else(|| "-".into())
            );
        }
    }
    Ok(())
}

fn cmd_doctor() -> anyhow::Result<()> {
    println!("Mimo OCR Phase 0 doctor");
    println!("cwd: {}", std::env::current_dir()?.display());

    match discover_tesseract() {
        Ok(p) => println!("tesseract: OK {}", p.display()),
        Err(e) => println!("tesseract: MISSING ({e})"),
    }
    match resolve_tessdata(None) {
        Some(p) => {
            println!("tessdata: {}", p.display());
            for lang in ["tur", "eng", "osd"] {
                let f = p.join(format!("{lang}.traineddata"));
                println!(
                    "  {lang}: {}",
                    if f.exists() {
                        format!("OK ({} bytes)", std::fs::metadata(&f)?.len())
                    } else {
                        "MISSING".into()
                    }
                );
            }
        }
        None => println!("tessdata: MISSING"),
    }

    match list_monitors() {
        Ok(ms) => {
            println!("monitors: {}", ms.len());
            for m in ms {
                println!(
                    "  [{}] {} {}x{} scale={:.2} primary={}",
                    m.index, m.name, m.width, m.height, m.scale_factor, m.is_primary
                );
            }
        }
        Err(e) => println!("monitors: FAIL ({e})"),
    }

    let reg = build_registry("all");
    println!("engines: {:?}", reg.ids());

    for p in [
        PathBuf::from("assets/models/tessdata"),
        PathBuf::from("C:/Program Files/Tesseract-OCR"),
    ] {
        if p.exists() {
            let mut total = 0u64;
            if let Ok(rd) = std::fs::read_dir(&p) {
                for e in rd.flatten() {
                    if e.path().is_file() {
                        if let Ok(md) = e.metadata() {
                            total += md.len();
                        }
                    }
                }
            }
            println!("size[{}]: {} bytes (top-level files)", p.display(), total);
        }
    }
    Ok(())
}

fn cmd_batch(
    dir: PathBuf,
    langs: String,
    quality: String,
    out_dir: Option<PathBuf>,
) -> anyhow::Result<()> {
    let images = list_images(&dir);
    if images.is_empty() {
        anyhow::bail!("no images in {}", dir.display());
    }
    let out_root = out_dir.unwrap_or_else(|| dir.join("ocr-out"));
    std::fs::create_dir_all(&out_root)?;
    let options = OcrOptions {
        languages: parse_langs(&langs),
        quality: parse_quality(&quality),
        model_dir: resolve_tessdata(None),
        ..Default::default()
    };
    let reg = build_registry("auto");
    let engine_id = pick_engine_id(&reg, "auto")?;
    let cancel = CancellationToken::new();
    let mut ok = 0usize;
    for img_path in &images {
        let stem = img_path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "image".into());
        match OcrImage::from_path(img_path) {
            Ok(img) => match reg.recognize(engine_id, &img, &options, &cancel) {
                Ok(doc) => {
                    let out = out_root.join(format!("{stem}.ocr.txt"));
                    std::fs::write(&out, &doc.plain_text)?;
                    println!("OK {} -> {} ({} ms)", stem, out.display(), doc.elapsed_ms);
                    ok += 1;
                }
                Err(e) => println!("ERR {stem}: {e}"),
            },
            Err(e) => println!("ERR {stem}: {e}"),
        }
    }
    println!("batch done: {ok}/{} -> {}", images.len(), out_root.display());
    Ok(())
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Engines => cmd_engines(),
        Commands::Ocr {
            image,
            engine,
            langs,
            quality,
            json,
            model_dir,
        } => cmd_ocr(image, engine, langs, quality, json, model_dir),
        Commands::Batch {
            dir,
            langs,
            quality,
            out_dir,
        } => cmd_batch(dir, langs, quality, out_dir),
        Commands::Monitors => cmd_monitors(),
        Commands::Capture {
            out,
            monitor,
            region,
            logical,
        } => cmd_capture(out, monitor, region, logical),
        Commands::Bench {
            dataset,
            engine,
            langs,
            json,
            model_dir,
        } => cmd_bench(dataset, engine, langs, json, model_dir),
        Commands::Doctor => cmd_doctor(),
    }
}
