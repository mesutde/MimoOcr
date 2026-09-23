import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open, save } from "@tauri-apps/plugin-dialog";

// ---------------------------------------------------------------------------
// Tipler
// ---------------------------------------------------------------------------

interface OcrWord {
  text: string;
  confidence: number;
  left: number;
  top: number;
  width: number;
  height: number;
}

interface OcrDocument {
  plain_text: string;
  words: OcrWord[];
  language: string;
  engine: string;
  elapsed_ms: number;
  image_png_base64: string;
}

interface EngineInfo {
  id: string;
  name: string;
  available: boolean;
  detail?: string | null;
  active: boolean;
}

interface EngineStatus {
  ok: boolean;
  path?: string | null;
  tessdata?: string | null;
  error?: string | null;
}

interface MonitorDto {
  index: number;
  x: number;
  y: number;
  width: number;
  height: number;
  scale: number;
}

interface DocResult {
  path: string;
  name: string;
  kind: string;
  plainText: string;
  engine: string;
  elapsedMs: number;
  words: OcrWord[];
}

interface BatchItem {
  path: string;
  name: string;
  kind: string;
  ok: boolean;
  error?: string | null;
  chars: number;
}

interface BatchResult {
  items: BatchItem[];
  outDir: string;
  okCount: number;
  failCount: number;
  combinedPath?: string | null;
}

interface VideoItemResult {
  path: string;
  name: string;
  ok: boolean;
  error?: string | null;
  outputFiles: string[];
  message: string;
}

interface VideoBatchDto {
  items: VideoItemResult[];
  outDir: string;
  okCount: number;
  failCount: number;
}

interface VideoSupportInfo {
  python: string;
  pythonOk: boolean;
  script?: string | null;
  scriptOk: boolean;
  ffmpegOk: boolean;
  tesseractOk: boolean;
}

const $ = <T extends HTMLElement>(id: string) => document.getElementById(id) as T;

// ---------------------------------------------------------------------------
// i18n (TR + EN)
// ---------------------------------------------------------------------------

type Lang = "tr" | "en";
let uiLang: Lang = "tr";

const I18N: Record<Lang, Record<string, string>> = {
  tr: {
    lblEngine: "Motor", tabCapture: "Yakala", tabDocument: "Belge", tabBatch: "Toplu",
    tabUdf: "UDF", tabVideo: "Video", btnEnginePath: "Yolu Seç…", btnEngineRescan: "Tekrar Tara",
    btnRegion: "Bölge seç", btnFull: "Ekran OCR",
    hintPreview: "Önizleme seç: overlay'de Ctrl+sürükle ile çoklu alan, Enter ile bitir.",
    lblMonitor: "Monitör", lblOcrLang: "OCR Dili", lblPsm: "Sayfa Modu (PSM)",
    lblScale: "Ön İşleme", btnOpen: "Görsel Aç…", btnPaste: "Panodan OCR",
    privacy: "Tamamen cihazınızda işlendi · Ağ kullanılmadı",
    paneSource: "Kaynak", paneText: "Metin",
    imgEmpty: "Henüz görsel yok. Bölge yakalayın veya görsel açın; önizlemeye sağ tıklayarak başka motorla yeniden OCR yapabilirsiniz.",
    btnCopy: "Panoya Kopyala", btnSave: "Farklı Kaydet…",
    docTip: "Destek: görseller (OCR) · PDF · DOCX · XLSX · PPTX · UDF (UYAP) · TXT/MD",
    btnBrowse: "Dosya seç…", btnDocRun: "Belgeyi OCR'la",
    udfTip: "UYAP Doküman Formatı (.udf): ZIP + content.xml. Metin çıkarılır; imza doğrulanmaz, gömülü resimler ayrı OCR'a girmez.",
    btnUdfRun: "UDF'leri Dönüştür", btnUdfAdd: "UDF Ekle…", udfNoFiles: "UDF dosyası yok",
    lblOutDir: "Kaydedilecek klasör:", chkPdfTitle: "PDF'e dosya adını başlık olarak yaz",
    btnBatchAdd: "Dosya ekle…", btnBatchOutDir: "Çıktı klasörü…",
    batchNoFiles: "Dosya yok", lblFormat: "Format", lblSaveMode: "Kayıt",
    optSeparate: "Ayrı ayrı", optCombined: "Birleştir",
    btnBatchRun: "Toplu başlat", btnClear: "Temizle",
    videoDesc: "Kaydırılan ekran kayıtlarından metin çıkarır: ffmpeg kare örnekleme → Tesseract → scroll tekrarı eleme → CSV/TXT.",
    btnVideoAdd: "Video Ekle…", videoNoFiles: "Henüz video seçilmedi.",
    lblScroll: "Scroll hızı", optAuto: "Otomatik", optFast: "Hızlı scroll", optSlow: "Yavaş scroll",
    lblQuality: "Kalite / Hız", optFastQ: "Hızlı (~120 kare)", optBalanced: "Dengeli (~180 kare)",
    optAccurate: "Yüksek (~280 kare)", btnVideoRun: "Videodan Çıkar",
    menuReOcr: "Motor ile yeniden OCR", working: "Çalışıyor…",
  },
  en: {
    lblEngine: "Engine", tabCapture: "Capture", tabDocument: "Document", tabBatch: "Batch",
    tabUdf: "UDF", tabVideo: "Video", btnEnginePath: "Pick path…", btnEngineRescan: "Rescan",
    btnRegion: "Region select", btnFull: "Screen OCR",
    hintPreview: "Preview select: Ctrl+drag multi-area in overlay, Enter to finish.",
    lblMonitor: "Monitor", lblOcrLang: "OCR Language", lblPsm: "Page Mode (PSM)",
    lblScale: "Preprocess", btnOpen: "Open image…", btnPaste: "Clipboard OCR",
    privacy: "Processed fully on-device · No network",
    paneSource: "Source", paneText: "Text",
    imgEmpty: "No image yet. Capture a region or open an image; right-click the preview to re-OCR with another engine.",
    btnCopy: "Copy", btnSave: "Save as…",
    docTip: "Supported: images (OCR) · PDF · DOCX · XLSX · PPTX · UDF (UYAP) · TXT/MD",
    btnBrowse: "Browse…", btnDocRun: "OCR document",
    udfTip: "UYAP Document Format (.udf): ZIP + content.xml. Text is extracted; signatures are not verified, embedded images are not OCR'd separately.",
    btnUdfRun: "Convert UDFs", btnUdfAdd: "Add UDFs…", udfNoFiles: "No UDF files",
    lblOutDir: "Save folder:", chkPdfTitle: "Write file name as PDF title",
    btnBatchAdd: "Add files…", btnBatchOutDir: "Output folder…",
    batchNoFiles: "No files", lblFormat: "Format", lblSaveMode: "Save",
    optSeparate: "Separate", optCombined: "Combined",
    btnBatchRun: "Start batch", btnClear: "Clear",
    videoDesc: "Extract text from scrolled screen recordings: ffmpeg sampling → Tesseract → scroll dedup → CSV/TXT.",
    btnVideoAdd: "Add videos…", videoNoFiles: "No videos selected.",
    lblScroll: "Scroll speed", optAuto: "Auto", optFast: "Fast scroll", optSlow: "Slow scroll",
    lblQuality: "Quality / Speed", optFastQ: "Fast (~120 frames)", optBalanced: "Balanced (~180 frames)",
    optAccurate: "High (~280 frames)", btnVideoRun: "Extract from video",
    menuReOcr: "Re-OCR with engine", working: "Working…",
  },
};

function t(key: string): string {
  return I18N[uiLang][key] ?? key;
}

function applyI18n() {
  document.querySelectorAll<HTMLElement>("[data-i18n]").forEach((el) => {
    const k = el.dataset.i18n!;
    if (k in I18N[uiLang]) el.textContent = I18N[uiLang][k];
  });
  $("btn-lang-tr").classList.toggle("active", uiLang === "tr");
  $("btn-lang-en").classList.toggle("active", uiLang === "en");
  document.documentElement.lang = uiLang;
}

$("btn-lang-tr").addEventListener("click", () => { uiLang = "tr"; applyI18n(); });
$("btn-lang-en").addEventListener("click", () => { uiLang = "en"; applyI18n(); });

// Tema
$("btn-theme").addEventListener("click", () => {
  const root = document.documentElement;
  const light = root.dataset.theme !== "light";
  root.dataset.theme = light ? "light" : "";
  $("btn-theme").textContent = light ? "☀️" : "🌙";
});

// Sekmeler
const views: Record<string, HTMLElement> = {
  yakala: $("view-yakala"),
  belge: $("view-belge"),
  toplu: $("view-toplu"),
  udf: $("view-udf"),
  video: $("view-video"),
};
document.querySelectorAll<HTMLButtonElement>(".tab").forEach((b) =>
  b.addEventListener("click", () => {
    document.querySelectorAll(".tab").forEach((x) => x.classList.toggle("active", x === b));
    const v = b.dataset.view!;
    for (const [k, el] of Object.entries(views)) el.hidden = k !== v;
  }),
);

// ---------------------------------------------------------------------------
// Motorlar
// ---------------------------------------------------------------------------

const selEngine = $<HTMLSelectElement>("sel-engine");
const engineWarn = $("engine-warn");
const engineWarnText = $("engine-warn-text");
let engines: EngineInfo[] = [];
let lastImageBase64 = "";

function engineLabel(e: EngineInfo): string {
  return e.available ? e.name : `${e.name} (yok)`;
}

async function refreshEngines() {
  try {
    engines = await invoke<EngineInfo[]>("list_engines");
    selEngine.innerHTML = "";
    for (const e of engines) {
      const o = document.createElement("option");
      o.value = e.id;
      o.textContent = engineLabel(e) + (e.detail ? ` — ${e.detail}` : "");
      o.disabled = !e.available;
      if (e.active) o.selected = true;
      selEngine.appendChild(o);
    }
  } catch { /* seçim gizli kalır */ }
}

selEngine.addEventListener("change", async () => {
  try {
    engines = await invoke<EngineInfo[]>("set_engine", { id: selEngine.value });
    await pushOptions();
  } catch (e) {
    setStatus(String(e), "err");
    refreshEngines();
  }
});

function renderEngineStatus(st: EngineStatus) {
  (engineWarn as HTMLElement).hidden = st.ok;
  if (!st.ok) engineWarnText.textContent = st.error ?? "Tesseract bulunamadı.";
}

async function refreshEngineStatus() {
  try {
    renderEngineStatus(await invoke<EngineStatus>("engine_status"));
  } catch (e) {
    renderEngineStatus({ ok: false, error: String(e) });
  }
}

$<HTMLButtonElement>("btn-engine-rescan").addEventListener("click", async () => {
  renderEngineStatus(await invoke<EngineStatus>("rescan_engine"));
  refreshEngines();
  refreshVideoReq();
});

$<HTMLButtonElement>("btn-engine-path").addEventListener("click", async () => {
  const sel = await open({ multiple: false, filters: [{ name: "Tesseract", extensions: ["exe"] }] });
  if (typeof sel === "string") {
    try {
      renderEngineStatus(await invoke<EngineStatus>("set_engine_path", { path: sel }));
    } catch (e) {
      renderEngineStatus({ ok: false, error: String(e) });
    }
    refreshEngines();
    refreshVideoReq();
  }
});

// Sağ-tık motor menüsü (önizleme)
const engineMenu = $("engine-menu");
const imgWrap = $("img-wrap");

imgWrap.addEventListener("contextmenu", (e) => {
  e.preventDefault();
  if (!lastImageBase64) return;
  engineMenu.innerHTML = "";
  const title = document.createElement("div");
  title.className = "menu-title";
  title.textContent = t("menuReOcr");
  engineMenu.appendChild(title);
  for (const en of engines) {
    if (!en.available) continue;
    const b = document.createElement("button");
    b.textContent = en.name;
    b.addEventListener("click", async () => {
      engineMenu.hidden = true;
      setBusy(true);
      try {
        const doc = await invoke<OcrDocument>("ocr_bytes", {
          imageBase64: lastImageBase64,
          engine: en.id,
        });
        showResult(doc);
      } catch (err) {
        setStatus(String(err), "err");
      } finally {
        setBusy(false);
      }
    });
    engineMenu.appendChild(b);
  }
  const r = imgWrap.getBoundingClientRect();
  engineMenu.style.left = `${Math.min(e.clientX - r.left, r.width - 230)}px`;
  engineMenu.style.top = `${Math.min(e.clientY - r.top, r.height - 40)}px`;
  engineMenu.hidden = false;
});
document.addEventListener("click", () => { engineMenu.hidden = true; });

// ---------------------------------------------------------------------------
// Yakala sekmesi
// ---------------------------------------------------------------------------

const btnCapture = $<HTMLButtonElement>("btn-region");
const btnFull = $<HTMLButtonElement>("btn-full");
const btnOpen = $<HTMLButtonElement>("btn-open");
const btnPaste = $<HTMLButtonElement>("btn-paste");
const btnCopy = $<HTMLButtonElement>("btn-copy");
const btnSave = $<HTMLButtonElement>("btn-save");
const selLang = $<HTMLSelectElement>("sel-lang");
const selPsm = $<HTMLSelectElement>("sel-psm");
const selScale = $<HTMLSelectElement>("sel-scale");
const selMonitor = $<HTMLSelectElement>("sel-monitor");
const txtResult = $<HTMLTextAreaElement>("txt-result");
const status = $("status");

let busy = false;

function setStatus(msg: string, cls: "" | "ok" | "err" = "") {
  status.textContent = msg;
  status.className = "status " + cls;
}

function setBusy(b: boolean) {
  busy = b;
  [btnCapture, btnFull, btnOpen, btnPaste].forEach((x) => (x.disabled = b));
  if (b) setStatus(t("working"));
}

function confSuffix(doc: OcrDocument): string {
  if (doc.words.length === 0) return "";
  const scored = doc.words.filter((w) => w.confidence > 0);
  const conf = scored.length > 0
    ? ` · güven %${(scored.reduce((a, w) => a + w.confidence, 0) / scored.length).toFixed(0)}`
    : "";
  return ` · ${doc.words.length} kelime${conf}`;
}

function showResult(doc: OcrDocument) {
  txtResult.value = doc.plain_text;
  lastImageBase64 = doc.image_png_base64;
  const img = document.createElement("img");
  img.src = "data:image/png;base64," + doc.image_png_base64;
  imgWrap.innerHTML = "";
  imgWrap.appendChild(img);
  imgWrap.classList.add("has-img");
  setStatus(`${doc.engine} · ${doc.language} · ${doc.elapsed_ms} ms${confSuffix(doc)}`, "ok");
}

function showResults(docs: OcrDocument[]) {
  if (docs.length === 0) return;
  if (docs.length === 1) { showResult(docs[0]); return; }
  txtResult.value = docs
    .map((d, i) => `===== ${i + 1} =====\n${d.plain_text}`)
    .join("\n\n");
  lastImageBase64 = docs[0].image_png_base64;
  const img = document.createElement("img");
  img.src = "data:image/png;base64," + docs[0].image_png_base64;
  imgWrap.innerHTML = "";
  imgWrap.appendChild(img);
  imgWrap.classList.add("has-img");
  const totalWords = docs.reduce((a, d) => a + d.words.length, 0);
  const totalMs = docs.reduce((a, d) => a + d.elapsed_ms, 0);
  setStatus(`${docs.length} alan · ${totalMs} ms · ${totalWords} kelime`, "ok");
}

async function runOcr(args: Record<string, unknown>) {
  if (busy) return;
  setBusy(true);
  try {
    const doc = await invoke<OcrDocument>("ocr_run", args);
    showResult(doc);
  } catch (e) {
    setStatus(String(e), "err");
  } finally {
    setBusy(false);
  }
}

btnCapture.addEventListener("click", () => invoke("begin_capture"));

btnFull.addEventListener("click", async () => {
  if (busy) return;
  setBusy(true);
  try {
    const doc = await invoke<OcrDocument>("ocr_fullscreen", {
      monitor: selMonitor.value === "" ? null : Number(selMonitor.value),
    });
    showResult(doc);
  } catch (e) {
    setStatus(String(e), "err");
  } finally {
    setBusy(false);
  }
});

const IMG_FILTER = {
  filters: [{ name: "Görsel", extensions: ["png", "jpg", "jpeg", "bmp", "tif", "tiff", "webp", "gif"] }],
};

btnOpen.addEventListener("click", async () => {
  const path = await open({ multiple: false, ...IMG_FILTER });
  if (typeof path === "string") await runOcr({ source: { kind: "file", path } });
});

btnPaste.addEventListener("click", () => runOcr({ source: { kind: "clipboard" } }));

btnCopy.addEventListener("click", async () => {
  if (!txtResult.value) return;
  await invoke("copy_text", { text: txtResult.value });
  setStatus(uiLang === "tr" ? "Panoya kopyalandı." : "Copied.", "ok");
});

btnSave.addEventListener("click", async () => {
  if (!txtResult.value) return;
  const path = await save({
    filters: [
      { name: "Metin", extensions: ["txt"] },
      { name: "Markdown", extensions: ["md"] },
      { name: "JSON", extensions: ["json"] },
    ],
  });
  if (path) {
    await invoke("save_text", { path, text: txtResult.value });
    setStatus((uiLang === "tr" ? "Kaydedildi: " : "Saved: ") + path, "ok");
  }
});

async function refreshMonitors() {
  try {
    const mons = await invoke<MonitorDto[]>("list_monitors");
    selMonitor.innerHTML = "";
    for (const m of mons) {
      const o = document.createElement("option");
      o.value = String(m.index);
      o.textContent = `#${m.index + 1} ${m.width}×${m.height} (×${m.scale})`;
      selMonitor.appendChild(o);
    }
    if (mons.length === 0) {
      const o = document.createElement("option");
      o.value = "";
      o.textContent = "—";
      selMonitor.appendChild(o);
    }
  } catch { /* tek monitör varsay */ }
}

async function pushOptions() {
  await invoke("set_options", {
    options: {
      languages: selLang.value,
      psm: selPsm.value,
      scale: Number(selScale.value),
    },
  });
}
[selLang, selPsm, selScale].forEach((s) => s.addEventListener("change", pushOptions));

listen<OcrDocument>("ocr-result", (ev) => showResult(ev.payload));
listen<OcrDocument[]>("ocr-results", (ev) => showResults(ev.payload));
listen<string>("ocr-error", (ev) => setStatus(ev.payload, "err"));
listen<Record<string, unknown>>("ocr-status", (ev) => {
  const s = ev.payload;
  setStatus(String(s["message"] ?? s["state"] ?? ""), s["state"] === "ready" ? "ok" : "");
});

// ---------------------------------------------------------------------------
// Belge sekmesi (görsel + PDF/DOCX/XLSX/PPTX/UDF/metin)
// ---------------------------------------------------------------------------

const docPath = $<HTMLInputElement>("doc-path");
const docResult = $<HTMLTextAreaElement>("doc-result");
const docStatus = $("doc-status");
let docBusy = false;

function setDocStatus(msg: string, cls: "" | "ok" | "err" = "") {
  docStatus.textContent = msg;
  docStatus.className = "status " + cls;
}

const DOC_FILTER = {
  filters: [{
    name: "Belgeler",
    extensions: ["png", "jpg", "jpeg", "bmp", "tif", "tiff", "webp",
      "pdf", "docx", "xlsx", "pptx", "udf", "txt", "md"],
  }],
};

$<HTMLButtonElement>("btn-doc-browse").addEventListener("click", async () => {
  const sel = await open({ multiple: false, ...DOC_FILTER });
  if (typeof sel === "string") {
    docPath.value = sel;
    setDocStatus("");
  }
});

$<HTMLButtonElement>("btn-doc-run").addEventListener("click", async () => {
  if (docBusy || !docPath.value) return;
  docBusy = true;
  setDocStatus(t("working"));
  try {
    const r = await invoke<DocResult>("ocr_path", { path: docPath.value });
    docResult.value = r.plainText;
    const extra = r.kind === "image" && r.words.length
      ? ` · ${r.words.length} kelime`
      : ` · ${r.plainText.length} karakter`;
    setDocStatus(`${r.engine} · ${r.elapsedMs} ms${extra}`, "ok");
  } catch (e) {
    setDocStatus(String(e), "err");
  } finally {
    docBusy = false;
  }
});

$<HTMLButtonElement>("btn-doc-copy").addEventListener("click", async () => {
  if (!docResult.value) return;
  await invoke("copy_text", { text: docResult.value });
});

$<HTMLButtonElement>("btn-doc-save").addEventListener("click", async () => {
  if (!docResult.value) return;
  const path = await save({
    filters: [
      { name: "Metin", extensions: ["txt"] },
      { name: "Markdown", extensions: ["md"] },
    ],
  });
  if (path) {
    await invoke("save_text", { path, text: docResult.value });
    setDocStatus(String(path), "ok");
  }
});

// ---------------------------------------------------------------------------
// UDF sekmesi (UYAP .udf toplu dönüştürme — Toplu akışın UDF'ye daraltılmış hali)
// ---------------------------------------------------------------------------

const udfFilesEl = $("udf-files");
const udfOutputs = $("udf-outputs");
const udfStatus = $("udf-status");
const udfBar = $("udf-progress-bar");
const selUdfFormat = $<HTMLSelectElement>("sel-udf-format");
const selUdfMode = $<HTMLSelectElement>("sel-udf-mode");
let udfPaths: string[] = [];
let udfOutDir = "";
let udfBusy = false;

function setUdfStatus(msg: string, cls: "" | "ok" | "err" = "") {
  udfStatus.textContent = msg;
  udfStatus.className = "status " + cls;
}

function renderUdfFiles() {
  udfFilesEl.innerHTML = udfPaths.length ? "" : `<span class="muted">${t("udfNoFiles")}</span>`;
  udfPaths.forEach((p, i) => {
    const div = document.createElement("div");
    div.className = "batch-item";
    const name = document.createElement("span");
    name.className = "name";
    name.textContent = `${i + 1}. 📄 ` + shortName(p);
    name.title = p;
    const rm = document.createElement("button");
    rm.type = "button";
    rm.className = "rm";
    rm.dataset.rm = String(i);
    rm.textContent = "×";
    rm.title = "×";
    div.append(name, rm);
    udfFilesEl.appendChild(div);
  });
  ($("udf-outdir") as HTMLElement).textContent = udfOutDir || "…";
}

udfFilesEl.addEventListener("click", (e) => {
  const b = (e.target as HTMLElement).closest("[data-rm]");
  if (!b) return;
  udfPaths.splice(Number((b as HTMLElement).dataset.rm), 1);
  renderUdfFiles();
});

$<HTMLButtonElement>("btn-udf-add").addEventListener("click", async () => {
  const sel = await open({
    multiple: true,
    filters: [{ name: "UYAP UDF", extensions: ["udf"] }],
  });
  const add = (s: string) => {
    if (!s.toLowerCase().endsWith(".udf") || udfPaths.includes(s)) return;
    udfPaths.push(s);
  };
  if (Array.isArray(sel)) sel.forEach(add);
  else if (typeof sel === "string") add(sel);
  renderUdfFiles();
});

$<HTMLButtonElement>("btn-udf-out").addEventListener("click", async () => {
  const sel = await open({ directory: true, multiple: false });
  if (typeof sel === "string") {
    udfOutDir = sel;
    renderUdfFiles();
  }
});

$<HTMLButtonElement>("btn-udf-clear").addEventListener("click", () => {
  udfPaths = [];
  udfOutputs.innerHTML = "";
  udfBar.style.width = "0%";
  setUdfStatus("");
  renderUdfFiles();
});

$<HTMLButtonElement>("btn-udf-run").addEventListener("click", async () => {
  if (udfBusy || udfPaths.length === 0) return;
  if (!udfOutDir) {
    const sel = await open({ directory: true, multiple: false });
    if (typeof sel !== "string") return;
    udfOutDir = sel;
    renderUdfFiles();
  }
  udfBusy = true;
  ($<HTMLButtonElement>("btn-udf-run")).disabled = true;
  udfOutputs.innerHTML = "";
  udfBar.style.width = "0%";
  try {
    const dto = await invoke<BatchResult>("batch_process_files", {
      files: udfPaths,
      outDir: udfOutDir,
      format: selUdfFormat.value,
      saveMode: selUdfMode.value,
      fileTitle: ($<HTMLInputElement>("chk-udf-title")).checked,
    });
    udfBar.style.width = "100%";
    setUdfStatus(`${dto.okCount}/${dto.items.length} → ${dto.outDir}`, dto.failCount ? "err" : "ok");
    for (const it of dto.items) {
      const div = document.createElement("div");
      div.textContent = it.ok ? `✓ ${it.name} (${it.chars})` : `✗ ${it.name}: ${it.error ?? ""}`;
      if (it.ok) div.className = "fout";
      udfOutputs.appendChild(div);
    }
    if (dto.combinedPath) {
      const div = document.createElement("div");
      div.className = "fout";
      div.textContent = "📦 " + dto.combinedPath;
      udfOutputs.appendChild(div);
    }
  } catch (e) {
    setUdfStatus(String(e), "err");
  } finally {
    udfBusy = false;
    ($<HTMLButtonElement>("btn-udf-run")).disabled = false;
  }
});

// ---------------------------------------------------------------------------
// Toplu sekmesi
// ---------------------------------------------------------------------------

const batchFilesEl = $("batch-files");
const batchOutputs = $("batch-outputs");
const batchStatus = $("batch-status");
const batchBar = $("batch-progress-bar");
const selBatchFormat = $<HTMLSelectElement>("sel-batch-format");
const selBatchMode = $<HTMLSelectElement>("sel-batch-mode");
let batchPaths: string[] = [];
let batchOutDir = "";
let batchBusy = false;

function setBatchStatus(msg: string, cls: "" | "ok" | "err" = "") {
  batchStatus.textContent = msg;
  batchStatus.className = "status " + cls;
}

function shortName(p: string): string {
  return p.split(/[\\/]/).pop() || p;
}

function renderBatchFiles() {
  batchFilesEl.innerHTML = batchPaths.length ? "" : `<span class="muted">${t("batchNoFiles")}</span>`;
  batchPaths.forEach((p, i) => {
    const div = document.createElement("div");
    div.className = "batch-item";
    const name = document.createElement("span");
    name.className = "name";
    name.textContent = `${i + 1}. 📄 ` + shortName(p);
    name.title = p;
    const rm = document.createElement("button");
    rm.type = "button";
    rm.className = "rm";
    rm.dataset.rm = String(i);
    rm.textContent = "×";
    rm.title = "×";
    div.append(name, rm);
    batchFilesEl.appendChild(div);
  });
  ($("batch-outdir") as HTMLElement).textContent = batchOutDir || "…";
}

batchFilesEl.addEventListener("click", (e) => {
  const b = (e.target as HTMLElement).closest("[data-rm]");
  if (!b) return;
  batchPaths.splice(Number((b as HTMLElement).dataset.rm), 1);
  renderBatchFiles();
});

$<HTMLButtonElement>("btn-batch-add").addEventListener("click", async () => {
  const sel = await open({ multiple: true, ...DOC_FILTER });
  if (Array.isArray(sel)) batchPaths.push(...sel.filter((s) => !batchPaths.includes(s)));
  else if (typeof sel === "string" && !batchPaths.includes(sel)) batchPaths.push(sel);
  renderBatchFiles();
});

$<HTMLButtonElement>("btn-batch-out").addEventListener("click", async () => {
  const sel = await open({ directory: true, multiple: false });
  if (typeof sel === "string") {
    batchOutDir = sel;
    renderBatchFiles();
  }
});

$<HTMLButtonElement>("btn-batch-clear").addEventListener("click", () => {
  batchPaths = [];
  batchOutputs.innerHTML = "";
  batchBar.style.width = "0%";
  setBatchStatus("");
  renderBatchFiles();
});

$<HTMLButtonElement>("btn-batch-run").addEventListener("click", async () => {
  if (batchBusy || batchPaths.length === 0) return;
  if (!batchOutDir) {
    const sel = await open({ directory: true, multiple: false });
    if (typeof sel !== "string") return;
    batchOutDir = sel;
    renderBatchFiles();
  }
  batchBusy = true;
  ($<HTMLButtonElement>("btn-batch-run")).disabled = true;
  batchOutputs.innerHTML = "";
  batchBar.style.width = "0%";
  try {
    const dto = await invoke<BatchResult>("batch_process_files", {
      files: batchPaths,
      outDir: batchOutDir,
      format: selBatchFormat.value,
      saveMode: selBatchMode.value,
      fileTitle: true,
    });
    batchBar.style.width = "100%";
    setBatchStatus(`${dto.okCount}/${dto.items.length} → ${dto.outDir}`, dto.failCount ? "err" : "ok");
    for (const it of dto.items) {
      const div = document.createElement("div");
      div.textContent = it.ok ? `✓ ${it.name} (${it.chars})` : `✗ ${it.name}: ${it.error ?? ""}`;
      if (it.ok) div.className = "fout";
      batchOutputs.appendChild(div);
    }
    if (dto.combinedPath) {
      const div = document.createElement("div");
      div.className = "fout";
      div.textContent = "📦 " + dto.combinedPath;
      batchOutputs.appendChild(div);
    }
  } catch (e) {
    setBatchStatus(String(e), "err");
  } finally {
    batchBusy = false;
    ($<HTMLButtonElement>("btn-batch-run")).disabled = false;
  }
});

listen<Record<string, unknown>>("batch-progress", (ev) => {
  const p = ev.payload;
  const i = Number(p["index"] ?? 0);
  const n = Number(p["total"] ?? 0);
  const pct = n > 0 ? `${(i / n) * 100}%` : "0%";
  if (udfBusy) udfBar.style.width = pct;
  else batchBar.style.width = pct;
});

// ---------------------------------------------------------------------------
// Video sekmesi
// ---------------------------------------------------------------------------

const btnVideoAdd = $<HTMLButtonElement>("btn-video-add");
const btnVideoOut = $<HTMLButtonElement>("btn-video-out");
const btnVideoRun = $<HTMLButtonElement>("btn-video-run");
const btnVideoClear = $<HTMLButtonElement>("btn-video-clear");
const selVmode = $<HTMLSelectElement>("sel-vmode");
const selVquality = $<HTMLSelectElement>("sel-vquality");
const selVformat = $<HTMLSelectElement>("sel-vformat");
const videoFiles = $("video-files");
const videoReq = $("video-req");
const videoStatus = $("video-status");
const videoLog = $("video-log") as unknown as HTMLPreElement;
const videoBar = $("video-progress-bar");
const videoOutputs = $("video-outputs");

let videoPaths: string[] = [];
let videoOutDir = "";
let videoBusy = false;

function renderVideoFiles() {
  videoFiles.innerHTML = videoPaths.length
    ? ""
    : `<span class="muted">${t("videoNoFiles")}</span>`;
  videoPaths.forEach((p, i) => {
    const div = document.createElement("div");
    div.className = "batch-item";
    const name = document.createElement("span");
    name.className = "name";
    name.textContent = `${i + 1}. 🎬 ` + shortName(p);
    name.title = p;
    const rm = document.createElement("button");
    rm.type = "button";
    rm.className = "rm";
    rm.dataset.rm = String(i);
    rm.textContent = "×";
    rm.title = "×";
    div.append(name, rm);
    videoFiles.appendChild(div);
  });
  ($("video-outdir") as HTMLElement).textContent = videoOutDir || "…";
}

videoFiles.addEventListener("click", (e) => {
  const b = (e.target as HTMLElement).closest("[data-rm]");
  if (!b) return;
  videoPaths.splice(Number((b as HTMLElement).dataset.rm), 1);
  renderVideoFiles();
});

function vlog(msg: string) {
  videoLog.textContent += msg + "\n";
  videoLog.scrollTop = videoLog.scrollHeight;
}

async function refreshVideoReq() {
  try {
    const info = await invoke<VideoSupportInfo>("video_support_info");
    const missing: string[] = [];
    if (!info.pythonOk) missing.push("Python yok");
    if (!info.scriptOk) missing.push("betik yok");
    if (!info.ffmpegOk) missing.push("ffmpeg yok");
    if (!info.tesseractOk) missing.push("Tesseract yok");
    videoReq.textContent = missing.length ? "⚠ " + missing.join(" · ") : "✓ python · ffmpeg · tesseract hazır";
    videoReq.className = "status " + (missing.length ? "err" : "ok");
  } catch (e) {
    videoReq.textContent = String(e);
    videoReq.className = "status err";
  }
}

btnVideoAdd.addEventListener("click", async () => {
  const sel = await open({
    multiple: true,
    filters: [{
      name: "Video",
      extensions: ["mp4", "mov", "avi", "mkv", "webm", "m4v", "wmv", "flv"],
    }],
  });
  if (Array.isArray(sel)) videoPaths.push(...sel.filter((s) => !videoPaths.includes(s)));
  else if (typeof sel === "string" && !videoPaths.includes(sel)) videoPaths.push(sel);
  renderVideoFiles();
});

btnVideoOut.addEventListener("click", async () => {
  const sel = await open({ directory: true, multiple: false });
  if (typeof sel === "string") {
    videoOutDir = sel;
    renderVideoFiles();
  }
});

btnVideoClear.addEventListener("click", () => {
  videoPaths = [];
  videoOutputs.innerHTML = "";
  videoLog.textContent = "";
  videoBar.style.width = "0%";
  videoStatus.textContent = "";
  renderVideoFiles();
});

btnVideoRun.addEventListener("click", async () => {
  if (videoBusy || videoPaths.length === 0) return;
  if (!videoOutDir) {
    const sel = await open({ directory: true, multiple: false });
    if (typeof sel !== "string") return;
    videoOutDir = sel;
    renderVideoFiles();
  }
  videoBusy = true;
  btnVideoRun.disabled = true;
  videoOutputs.innerHTML = "";
  videoLog.textContent = "";
  videoBar.style.width = "0%";
  videoStatus.textContent = t("working");
  videoStatus.className = "status";
  try {
    const dto = await invoke<VideoBatchDto>("video_extract_batch", {
      files: videoPaths,
      outDir: videoOutDir,
      mode: selVmode.value,
      langs: selLang.value,
      maxFrames: Number(selVquality.value),
      formats: selVformat.value,
    });
    videoBar.style.width = "100%";
    videoStatus.textContent = `${dto.okCount}/${dto.items.length} → ${dto.outDir}`;
    videoStatus.className = "status " + (dto.failCount ? "err" : "ok");
    for (const it of dto.items) {
      const div = document.createElement("div");
      if (it.ok) {
        div.className = "fout";
        div.textContent = "✓ " + it.name + " → " + it.outputFiles.join(" · ");
      } else {
        div.textContent = "✗ " + it.name + ": " + (it.error ?? "hata");
      }
      videoOutputs.appendChild(div);
    }
  } catch (e) {
    videoStatus.textContent = String(e);
    videoStatus.className = "status err";
  } finally {
    videoBusy = false;
    btnVideoRun.disabled = false;
  }
});

listen<string>("video-log", (ev) => vlog(ev.payload));
listen<Record<string, unknown>>("video-progress", (ev) => {
  const p = Number(ev.payload["percent"] ?? 0);
  videoBar.style.width = `${Math.max(0, Math.min(100, p))}%`;
});

// ---------------------------------------------------------------------------
// Başlatma
// ---------------------------------------------------------------------------

applyI18n();
refreshEngines();
refreshEngineStatus();
refreshMonitors();
refreshVideoReq();
renderBatchFiles();
renderVideoFiles();
renderUdfFiles();
pushOptions();
txtResult.placeholder = t("imgEmpty");
