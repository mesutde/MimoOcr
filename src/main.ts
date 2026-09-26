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
    tabUdf: "UDF", tabWeb: "Web", tabVideo: "Video", btnEnginePath: "Yolu Seç…", btnEngineRescan: "Tekrar Tara",
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
    webTip: "Herkese açık Google Sheets bağlantısını yapıştırın; CSV / XLSX / Markdown olarak kaydedilir.",
    webUrlPh: "https://docs.google.com/spreadsheets/d/…",
    btnWebRun: "İçe Aktar",
    lblScroll: "Scroll hızı", optAuto: "Otomatik", optFast: "Hızlı scroll", optSlow: "Yavaş scroll",
    lblQuality: "Kalite / Hız", optFastQ: "Hızlı (~120 kare)", optBalanced: "Dengeli (~180 kare)",
    optAccurate: "Yüksek (~280 kare)", btnVideoRun: "Videodan Çıkar",
    menuReOcr: "Motor ile yeniden OCR", menuCopyImg: "📋 Kopyala (resim)", menuEdit: "✏️ Düzenle",
    menuPaste: "📋 Yapıştır", menuAll: "Tümü", menuLayer: "Katman", working: "Çalışıyor…",
    optPsmAuto: "Otomatik", optPsmBlock: "Tek blok", optPsmLine: "Tek satır", optPsmSparse: "Seyrek metin",
    optScaleNone: "Yok", optScale2: "2× Büyüt", optScale3: "3× Büyüt",
    zoomTip: "Yakınlaştırma: %{p} (sıfırlamak için çift tık)",
    engTesseract: "Tesseract (gömülü)", engWindows: "Windows OCR (sistem)", engMissing: "(yok)",
    engLangNone: "Tesseract bulunamadı.",
    uAreas: "alan", uMs: "ms", uWords: "kelime", uChars: "karakter", uRows: "satır", uCols: "sütun", uSec: "sn",
    stConf: "· güven %{p}",
    msgCopiedImg: "Resim panoya kopyalandı.", msgCopied: "Panoya kopyalandı.", msgSaved: "Kaydedildi: ",
    msgPasted: "Panodan yapıştırıldı.",
    msgPrevBusy: "Önceki işlem sürüyor, bitmesini bekleyin.",
    msgViewCut: "…[görünüm kısaltıldı: {n} karakterin tamamı kopyala/kaydet ile alınabilir]",
    msgEnterLink: "Önce geçerli bir bağlantı girin (Google Tablosu/Belgesi veya dosya).",
    msgPickUdf: "Yalnız .udf dosyası seçin.",
    reqPython: "Python yok", reqScript: "betik yok", reqFfmpeg: "ffmpeg yok", reqTess: "Tesseract yok",
    reqReady: "✓ python · ffmpeg · tesseract hazır",
    filterImage: "Görsel", filterDocs: "Belgeler", filterUdf: "UYAP UDF", filterVideo: "Video",
    filterTess: "Tesseract", filterText: "Metin", filterMd: "Markdown", filterJson: "JSON",
    webDone: "✓ {name} → {path}", webTable: "✓ {name} · {rows} satır × {cols} sütun → {path}",
    msgDropSkip: "{view}: desteklenmeyen dosya ({n} atlandı)",
    edTitle: "Düzenle", edCrop: "Kırp", edFlip: "Yansıt", edPen: "Kalem", edBox: "Kutu",
    edBorder: "Çerçeve",
    edBright: "Parlaklık", edContrast: "Kontrast", edBg: "Zemin",
    edUndo: "Geri Al", edApply: "Uygula", edCancel: "Vazgeç",
  },
  en: {
    lblEngine: "Engine", tabCapture: "Capture", tabDocument: "Document", tabBatch: "Batch",
    tabUdf: "UDF", tabWeb: "Web", tabVideo: "Video", btnEnginePath: "Pick path…", btnEngineRescan: "Rescan",
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
    webTip: "Paste a public Google Sheets link; save as CSV / XLSX / Markdown.",
    webUrlPh: "https://docs.google.com/spreadsheets/d/…",
    btnWebRun: "Import",
    lblScroll: "Scroll speed", optAuto: "Auto", optFast: "Fast scroll", optSlow: "Slow scroll",
    lblQuality: "Quality / Speed", optFastQ: "Fast (~120 frames)", optBalanced: "Balanced (~180 frames)",
    optAccurate: "High (~280 frames)", btnVideoRun: "Extract from video",
    menuReOcr: "Re-OCR with engine", menuCopyImg: "📋 Copy (image)", menuEdit: "✏️ Edit",
    menuPaste: "📋 Paste", menuAll: "All", menuLayer: "Layer", working: "Working…",
    optPsmAuto: "Auto", optPsmBlock: "Single block", optPsmLine: "Single line", optPsmSparse: "Sparse text",
    optScaleNone: "None", optScale2: "2× Upscale", optScale3: "3× Upscale",
    zoomTip: "Zoom: {p}% (double-click to reset)",
    engTesseract: "Tesseract (embedded)", engWindows: "Windows OCR (system)", engMissing: "(missing)",
    engLangNone: "Tesseract not found.",
    uAreas: "areas", uMs: "ms", uWords: "words", uChars: "chars", uRows: "rows", uCols: "cols", uSec: "s",
    stConf: "· {p}% conf.",
    msgCopiedImg: "Image copied.", msgCopied: "Copied.", msgSaved: "Saved: ",
    msgPasted: "Pasted from clipboard.",
    msgPrevBusy: "Previous task still running.",
    msgViewCut: "…[view truncated: full {n} chars available via copy/save]",
    msgEnterLink: "Enter a valid link first (Google Sheet/Doc or file).",
    msgPickUdf: "Please pick a .udf file.",
    reqPython: "no Python", reqScript: "no script", reqFfmpeg: "no ffmpeg", reqTess: "no Tesseract",
    reqReady: "✓ python · ffmpeg · tesseract ready",
    filterImage: "Images", filterDocs: "Documents", filterUdf: "UYAP UDF", filterVideo: "Video",
    filterTess: "Tesseract", filterText: "Text", filterMd: "Markdown", filterJson: "JSON",
    webDone: "✓ {name} → {path}", webTable: "✓ {name} · {rows} rows × {cols} cols → {path}",
    msgDropSkip: "{view}: unsupported file ({n} skipped)",
    edTitle: "Edit", edCrop: "Crop", edFlip: "Flip", edPen: "Pen", edBox: "Box",
    edBorder: "Border",
    edBright: "Brightness", edContrast: "Contrast", edBg: "Background",
    edUndo: "Undo", edApply: "Apply", edCancel: "Cancel",
  },
};

function tFmt(key: string, params: Record<string, string | number>): string {
  let s: string = I18N[uiLang][key] ?? key;
  for (const [k, v] of Object.entries(params)) s = s.replace(`{${k}}`, String(v));
  return s;
}

function t(key: string): string {
  return I18N[uiLang][key] ?? key;
}

function applyI18n() {
  document.querySelectorAll<HTMLElement>("[data-i18n]").forEach((el) => {
    const k = el.dataset.i18n!;
    if (k in I18N[uiLang]) el.textContent = I18N[uiLang][k];
  });
  document.querySelectorAll<HTMLElement>("[data-i18n-ph]").forEach((el) => {
    const k = el.dataset.i18nPh!;
    if (k in I18N[uiLang]) (el as HTMLInputElement).placeholder = I18N[uiLang][k];
  });
  $("btn-lang-tr").classList.toggle("active", uiLang === "tr");
  $("btn-lang-en").classList.toggle("active", uiLang === "en");
  document.documentElement.lang = uiLang;
}

$("btn-lang-tr").addEventListener("click", () => { setUiLang("tr"); });
$("btn-lang-en").addEventListener("click", () => { setUiLang("en"); });

async function setUiLang(l: Lang) {
  uiLang = l;
  persistUiLang();
  applyI18n();
  try {
    await invoke("set_ui_lang", { lang: l });
  } catch { /* varsayılan tr kalır */ }
  refreshOcrLangs();
  refreshEngines();
}

function persistUiLang() {
  try {
    localStorage.setItem("mimo-ui-lang", uiLang);
  } catch { /* yoksay */ }
}

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
  web: $("view-web"),
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

function engineName(e: EngineInfo): string {
  if (e.id === "tesseract") return t("engTesseract");
  if (e.id === "windows-ocr") return t("engWindows");
  return e.name;
}

function engineDetail(e: EngineInfo): string {
  const d = e.detail ?? "";
  if (!d || d === "bulunamadı") return "";
  if (d === "dil paketi yok") return uiLang === "tr" ? "dil paketi yok" : "no language pack";
  if (d.startsWith("dil: ")) return (uiLang === "tr" ? "dil: " : "langs: ") + d.slice(5);
  return d;
}

function engineLabel(e: EngineInfo): string {
  const base = engineName(e) + (e.available ? "" : " " + t("engMissing"));
  const det = engineDetail(e);
  return det ? `${base} — ${det}` : base;
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

interface EngineLang {
  code: string;
}

// Motorun destekledigi OCR dilleri: secili motora gore listelenir.
const OCR_LANG_NAMES: Record<Lang, Record<string, string>> = {
  tr: {
    "tur+eng": "Türkçe + İngilizce",
    tur: "Türkçe", eng: "İngilizce", ara: "Arapça",
    chi_sim: "Çince (Basit)", chi_tra: "Çince (Geleneksel)",
    jpn: "Japonca", kor: "Korece",
  },
  en: {
    "tur+eng": "Turkish + English",
    tur: "Turkish", eng: "English", ara: "Arabic",
    chi_sim: "Chinese (Simplified)", chi_tra: "Chinese (Traditional)",
    jpn: "Japanese", kor: "Korean",
  },
};

function ocrLangLabel(code: string): string {
  return OCR_LANG_NAMES[uiLang][code] ?? code;
}

async function refreshOcrLangs() {
  try {
    const langs = await invoke<EngineLang[]>("engine_languages");
    if (langs.length === 0) return; // motor yoksa eski liste kalir
    const prev = selLang.value;
    selLang.innerHTML = "";
    for (const l of langs) {
      const o = document.createElement("option");
      o.value = l.code;
      o.textContent = ocrLangLabel(l.code);
      selLang.appendChild(o);
    }
    selLang.value = langs.some((l) => l.code === prev) ? prev : langs[0].code;
    await pushOptions();
  } catch { /* eski liste kalir */ }
}

selEngine.addEventListener("change", async () => {
  try {
    engines = await invoke<EngineInfo[]>("set_engine", { id: selEngine.value });
    await refreshOcrLangs();
    await pushOptions();
  } catch (e) {
    setStatus(String(e), "err");
    refreshEngines();
  }
});

function renderEngineStatus(st: EngineStatus) {
  (engineWarn as HTMLElement).hidden = st.ok;
  if (!st.ok) engineWarnText.textContent = st.error ?? t("engLangNone");
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
  const sel = await open({ multiple: false, filters: [{ name: t("filterTess"), extensions: ["exe"] }] });
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

// ---------------------------------------------------------------------------
// Kaynak katmanları: tikla-sec, surukle-tasi, koseden boyutlandir.
// ---------------------------------------------------------------------------

interface SrcLayer {
  id: number;
  b64: string;
  x: number;
  y: number;
  w: number;
  h: number;
  nw: number;
  nh: number;
  el?: HTMLDivElement;
}

let srcLayers: SrcLayer[] = [];
let selectedLayer = -1;
let layerSeq = 0;
const LAYER_GAP = 8;

function stageEl(): HTMLElement {
  let st = $("img-stage") as HTMLElement | null;
  if (!st) {
    imgWrap.innerHTML = "";
    st = document.createElement("div");
    st.id = "img-stage";
    imgWrap.appendChild(st);
  }
  return st;
}

function clearStage() {
  srcLayers = [];
  selectedLayer = -1;
  imgWrap.classList.remove("has-img");
  imgWrap.innerHTML = `<span id="img-empty">${t("imgEmpty")}</span>`;
  lastImageBase64 = "";
}

// İlk dizilim: 2 sutunlu izgara (kolaj gorunumu), en-boy korunur.
function layoutLayers() {
  const cols = 2;
  const cellW = 560;
  srcLayers.forEach((L) => {
    const s = Math.min(1, cellW / L.nw);
    L.w = Math.max(32, Math.round(L.nw * s));
    L.h = Math.max(32, Math.round(L.nh * s));
  });
  const rows = Math.ceil(srcLayers.length / cols);
  const colW = [0, 0];
  const rowH: number[] = [];
  for (let r = 0; r < rows; r++) {
    let h = 0;
    for (let c = 0; c < cols; c++) {
      const L = srcLayers[r * cols + c];
      if (!L) continue;
      h = Math.max(h, L.h);
      colW[c] = Math.max(colW[c], L.w);
    }
    rowH.push(h);
  }
  let y = 0;
  for (let r = 0; r < rows; r++) {
    let x = 0;
    for (let c = 0; c < cols; c++) {
      const L = srcLayers[r * cols + c];
      if (L) {
        L.x = x + Math.round((colW[c] - L.w) / 2);
        L.y = y + Math.round((rowH[r] - L.h) / 2);
      }
      x += colW[c] + LAYER_GAP;
    }
    y += rowH[r] + LAYER_GAP;
  }
}

function renderLayers() {
  const st = stageEl();
  st.innerHTML = "";
  const z = previewZoom;
  let maxX = 0;
  let maxY = 0;
  srcLayers.forEach((L, i) => {
    const d = document.createElement("div");
    d.className = "layer" + (i === selectedLayer ? " sel" : "");
    d.style.left = Math.round(L.x * z) + "px";
    d.style.top = Math.round(L.y * z) + "px";
    d.style.width = Math.round(L.w * z) + "px";
    d.style.height = Math.round(L.h * z) + "px";
    const img = document.createElement("img");
    img.src = "data:image/png;base64," + L.b64;
    img.draggable = false;
    d.appendChild(img);
    const num = document.createElement("span");
    num.className = "layer-num";
    num.textContent = String(i + 1);
    d.appendChild(num);
    if (i === selectedLayer) {
      (["nw", "ne", "sw", "se"] as const).forEach((pos) => {
        const h = document.createElement("span");
        h.className = "handle " + pos;
        h.dataset.handle = pos;
        d.appendChild(h);
      });
    }
    d.addEventListener("mousedown", (e) => onLayerDown(e, i));
    d.addEventListener("contextmenu", (e) => {
      e.preventDefault();
      e.stopPropagation();
      selectLayer(i);
      openEngineMenu(e.clientX, e.clientY, i);
    });
    st.appendChild(d);
    maxX = Math.max(maxX, (L.x + L.w) * z);
    maxY = Math.max(maxY, (L.y + L.h) * z);
  });
  st.style.width = Math.max(1, Math.round(maxX)) + "px";
  st.style.height = Math.max(1, Math.round(maxY)) + "px";
  st.title = zoomLabel();
  imgWrap.classList.toggle("has-img", srcLayers.length > 0);
  lastImageBase64 = srcLayers.length ? srcLayers[Math.max(0, selectedLayer)].b64 : "";
}

function selectLayer(i: number) {
  selectedLayer = i;
  renderLayers();
}

// Sürükle-tasi + koseden boyutlandir.
let dragMode: null | { kind: "move" | "size"; idx: number; el: HTMLDivElement; sx: number; sy: number; ox: number; oy: number; ow: number; oh: number; corner: string } = null;

function onLayerDown(e: MouseEvent, i: number) {
  if (e.button !== 0) return;
  selectLayer(i);
  const L = srcLayers[i];
  if (!L) return;
  const h = (e.target as HTMLElement).dataset.handle;
  // Secim sahneyi yeniden kurdu; suruklenecek guncel kutuyu sahneden al.
  const el = stageEl().children[i] as HTMLDivElement | undefined;
  if (!el) return;
  // Isaretci yakalama: imlec pencere disina ciksa bile mouseup/move gelsin.
  try {
    el.setPointerCapture((e as PointerEvent).pointerId);
  } catch { /* desteksiz tarayici */ }
  dragMode = {
    kind: h ? "size" : "move",
    idx: i,
    el,
    sx: e.clientX, sy: e.clientY,
    ox: L.x, oy: L.y, ow: L.w, oh: L.h,
    corner: h ?? "",
  };
  e.preventDefault();
}

function paintDragged() {
  // Tumu compositor'da: kutu koken konum/boyutta durur, fark transform ile
  // uygulanir (layout yok, titreme yok). Model gunceldir; birakinca tek render.
  const d = dragMode;
  if (!d) return;
  const L = srcLayers[d.idx];
  if (!L || !d.el.isConnected) return;
  const z = previewZoom;
  d.el.style.left = Math.round(d.ox * z) + "px";
  d.el.style.top = Math.round(d.oy * z) + "px";
  d.el.style.width = Math.max(1, Math.round(d.ow * z)) + "px";
  d.el.style.height = Math.max(1, Math.round(d.oh * z)) + "px";
  const dx = (L.x - d.ox) * z;
  const dy = (L.y - d.oy) * z;
  const sx = d.ow > 0 ? L.w / d.ow : 1;
  const sy = d.oh > 0 ? L.h / d.oh : 1;
  // Sabit kose capalanir (tasi: sol ust).
  const origin = d.kind === "move"
    ? "0 0"
    : d.corner.includes("n")
      ? d.corner.includes("w") ? "100% 100%" : "0 100%"
      : d.corner.includes("w") ? "100% 0" : "0 0";
  d.el.style.transformOrigin = origin;
  d.el.style.transform = `translate(${dx}px, ${dy}px) scale(${sx}, ${sy})`;
}

document.addEventListener("mousemove", (e) => {
  if (!dragMode) return;
  const d = dragMode;
  const L = srcLayers[d.idx];
  if (!L) { dragMode = null; return; }
  const dx = (e.clientX - d.sx) / previewZoom;
  const dy = (e.clientY - d.sy) / previewZoom;
  if (Math.abs(e.clientX - d.sx) + Math.abs(e.clientY - d.sy) > 3) suppressStageClick = true;
  if (d.kind === "move") {
    L.x = Math.round(d.ox + dx);
    L.y = Math.round(d.oy + dy);
    // Tamamen sahne disina kaybolmasin: en az 48px tutamak payi kalir.
    // (Sinirda durmaz, otesine tasinir ama tutamak erisilir kalir.)
    L.x = Math.max(48 - L.w, L.x);
    L.y = Math.max(48 - L.h, L.y);
  } else {
    const fx = (d.ow + (d.corner.includes("w") ? -dx : dx)) / d.ow;
    const fy = (d.oh + (d.corner.includes("n") ? -dy : dy)) / d.oh;
    const f = Math.abs(dx) >= Math.abs(dy) ? fx : fy;
    const nw = Math.max(32, Math.round(d.ow * f));
    const nh = Math.max(32, Math.round(d.oh * f));
    if (d.corner.includes("n")) L.y = Math.round(d.oy + (d.oh - nh));
    if (d.corner.includes("w")) L.x = Math.round(d.ox + (d.ow - nw));
    L.w = nw;
    L.h = nh;
    L.x = Math.max(48 - L.w, L.x);
    L.y = Math.max(48 - L.h, L.y);
  }
  paintDragged();
});

function endDrag() {
  if (dragMode) {
    renderLayers();
  }
  dragMode = null;
}

document.addEventListener("mouseup", endDrag);
// Isaretci calinirsa/iptal olursa da ayni kapanis (yari yolda takilma yok).
document.addEventListener("pointercancel", endDrag);

// Kaynak onizlemede fare tekerlegiyle yakinlastirma (cift tik sifirlar).
// Yakilastirma renderLayers icinde uygulanir (katman geometrisi korunur).
let previewZoom = 1;

function zoomLabel(): string {
  return tFmt("zoomTip", { p: Math.round(previewZoom * 100) });
}

function resetZoom() {
  previewZoom = 1;
  if (srcLayers.length) renderLayers();
}

function previewImg(): HTMLImageElement | null {
  return ($("img-wrap") as HTMLElement).querySelector("#img-stage img");
}

function bindPreviewZoom() {
  const box = $("img-wrap") as HTMLElement;
  box.addEventListener("wheel", (e) => {
    if (!srcLayers.length) return;
    e.preventDefault();
    const step = e.deltaY < 0 ? 1.15 : 1 / 1.15;
    previewZoom = Math.max(0.2, Math.min(8, previewZoom * step));
    renderLayers();
  }, { passive: false });

  box.addEventListener("dblclick", () => {
    previewZoom = 1;
    renderLayers();
  });
}

// Sağ-tık motor menüsü (önizleme): katman üstünde tek katman + Tümü,
// boş alanda (katman varsa) Tümü.
const engineMenu = $("engine-menu");
const imgWrap = $("img-wrap");
bindPreviewZoom();

async function reOcrLayers(idxs: number[], engineId: string) {
  engineMenu.hidden = true;
  const b64s = idxs.map((i) => srcLayers[i]?.b64).filter((s): s is string => !!s);
  if (!b64s.length) return;
  if (busy) { setStatus(t("msgPrevBusy"), "err"); return; }
  setBusy(true);
  try {
    const docs = await invoke<OcrDocument[]>("ocr_bytes_batch", {
      imagesBase64: b64s,
      engine: engineId,
    });
    if (!docs.length) return;
    txtResult.value = combineText(docs);
    const totalWords = docs.reduce((a, d) => a + d.words.length, 0);
    const totalMs = docs.reduce((a, d) => a + d.elapsed_ms, 0);
    setStatus(`${docs[0].engine} · ${totalMs} ${t("uMs")} · ${totalWords} ${t("uWords")}`, "ok");
  } catch (err) {
    setStatus(String(err), "err");
  } finally {
    setBusy(false);
  }
}

function addEngineItems(targetIdx: number | "all") {
  const idxs = targetIdx === "all"
    ? srcLayers.map((_, i) => i)
    : [targetIdx];
  const title = document.createElement("div");
  title.className = "menu-title";
  title.textContent = t("menuReOcr")
    + (targetIdx === "all" ? ` • ${t("menuAll")}` : ` • ${t("menuLayer")} ${targetIdx + 1}`);
  engineMenu.appendChild(title);
  for (const en of engines) {
    if (!en.available) continue;
    const b = document.createElement("button");
    b.textContent = engineName(en);
    b.addEventListener("click", () => reOcrLayers(idxs, en.id));
    engineMenu.appendChild(b);
  }
}

interface ClipImage {
  imagePngBase64: string;
  width: number;
  height: number;
}

// Panodaki resmi kaynak katmani olarak ekler (dogal boyut; buyukse kuculur).
async function pasteFromClipboard() {
  try {
    const r = await invoke<ClipImage>("clipboard_image");
    addSourceLayer(r.imagePngBase64, r.width, r.height);
    setStatus(t("msgPasted"), "ok");
  } catch (e) {
    setStatus(String(e), "err");
  }
}

function addSourceLayer(b64: string, nw: number, nh: number) {
  const s = Math.min(1, 1100 / Math.max(1, Math.max(nw, nh)));
  const w = Math.max(32, Math.round(nw * s));
  const h = Math.max(32, Math.round(nh * s));
  const off = (srcLayers.length % 6) * 24;
  srcLayers.push({ id: layerSeq++, b64, x: off, y: off, w, h, nw, nh });
  selectedLayer = srcLayers.length - 1;
  renderLayers();
}

function openEngineMenu(x: number, y: number, idx: number | "all") {
  engineMenu.innerHTML = "";
  if (srcLayers.length) addEngineItems(idx);
  if (idx !== "all") {
    const cp = document.createElement("button");
    cp.textContent = t("menuCopyImg");
    cp.addEventListener("click", async () => {
      engineMenu.hidden = true;
      try {
        await invoke("copy_image", { imageBase64: srcLayers[idx].b64 });
        setStatus(t("msgCopiedImg"), "ok");
      } catch (err) {
        setStatus(String(err), "err");
      }
    });
    engineMenu.appendChild(cp);
    const ed = document.createElement("button");
    ed.textContent = t("menuEdit");
    ed.addEventListener("click", () => {
      engineMenu.hidden = true;
      openEditor(idx);
    });
    engineMenu.appendChild(ed);
  } else {
    const sep = document.createElement("div");
    sep.className = "menu-sep";
    engineMenu.appendChild(sep);
  }
  const ps = document.createElement("button");
  ps.textContent = t("menuPaste");
  ps.addEventListener("click", async () => {
    engineMenu.hidden = true;
    await pasteFromClipboard();
  });
  engineMenu.appendChild(ps);
  // İmleçte aç (ekran taşması korumalı)
  const w = 250;
  const h = Math.min(420, 60 + engineMenu.childElementCount * 36);
  engineMenu.style.left = `${Math.max(4, Math.min(x, window.innerWidth - w))}px`;
  engineMenu.style.top = `${Math.max(4, Math.min(y, window.innerHeight - h))}px`;
  engineMenu.hidden = false;
}

imgWrap.addEventListener("contextmenu", (e) => {
  e.preventDefault();
  openEngineMenu(e.clientX, e.clientY, "all");
});

// Ctrl+V: kaynak gorunumunde panodaki resmi katman olarak ekle.
document.addEventListener("keydown", (e) => {
  const tag = (e.target as HTMLElement)?.tagName ?? "";
  if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "v") {
    if (tag === "TEXTAREA" || tag === "INPUT" || tag === "SELECT") return;
    if (currentView() !== "yakala") return;
    e.preventDefault();
    pasteFromClipboard();
  }
});

// Bos alana tiklama: secimi birak (numaralar da gizlenir).
// Surukleme sonrasi gelen tiklama yoksayilir.
let suppressStageClick = false;
imgWrap.addEventListener("click", (e) => {
  if (suppressStageClick) {
    suppressStageClick = false;
    return;
  }
  const el = e.target as HTMLElement;
  if (el === imgWrap || el.id === "img-stage" || el.id === "img-empty") {
    if (selectedLayer !== -1) {
      selectedLayer = -1;
      renderLayers();
    }
  }
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
    ? " " + tFmt("stConf", { p: (scored.reduce((a, w) => a + w.confidence, 0) / scored.length).toFixed(0) })
    : "";
  return ` · ${doc.words.length} ${t("uWords")}${conf}`;
}

function showResult(doc: OcrDocument) {
  txtResult.value = doc.plain_text;
  setLayersFromDocs([doc]);
  setStatus(`${doc.engine} · ${doc.language} · ${doc.elapsed_ms} ${t("uMs")}${confSuffix(doc)}`, "ok");
}

function combineText(docs: OcrDocument[]): string {
  return docs.map((d, i) => `===== ${i + 1} =====\n${d.plain_text}`).join("\n\n");
}

function showResults(docs: OcrDocument[]) {
  if (docs.length === 0) return;
  if (docs.length === 1) { showResult(docs[0]); return; }
  txtResult.value = combineText(docs);
  setLayersFromDocs(docs);
  const totalWords = docs.reduce((a, d) => a + d.words.length, 0);
  const totalMs = docs.reduce((a, d) => a + d.elapsed_ms, 0);
  setStatus(`${docs.length} ${t("uAreas")} · ${totalMs} ${t("uMs")} · ${totalWords} ${t("uWords")}`, "ok");
}

// Belgelerden katman listesi kurar (2 sutun izgara, en-boy korunur).
function setLayersFromDocs(docs: OcrDocument[]) {
  srcLayers = [];
  selectedLayer = docs.length > 1 ? -1 : 0;
  previewZoom = 1;
  const loaders = docs.map((d) => d.image_png_base64).filter(Boolean);
  if (!loaders.length) {
    renderLayers();
    return;
  }
  Promise.all(loaders.map(loadImg)).then((imgs) => {
    srcLayers = imgs.map((im, i) => ({
      id: layerSeq++,
      b64: docs[i].image_png_base64,
      x: 0, y: 0, w: im.naturalWidth, h: im.naturalHeight,
      nw: im.naturalWidth, nh: im.naturalHeight,
    }));
    // Asiri buyuk tek resimleri sahneye sigdir (orijinal b64 korunur).
    const maxStage = 1100;
    srcLayers.forEach((L) => {
      const s = Math.min(1, maxStage / Math.max(L.nw, L.nh));
      L.w = Math.max(32, Math.round(L.nw * s));
      L.h = Math.max(32, Math.round(L.nh * s));
    });
    layoutLayers();
    if (srcLayers.length === 1) selectedLayer = 0;
    renderLayers();
  }).catch(() => {
    // Yedek: ilk belge klasik gosterim
    if (docs.length) showResult(docs[0]);
  });
}

function loadImg(b64: string): Promise<HTMLImageElement> {
  return new Promise((resolve, reject) => {
    const img = new Image();
    img.onload = () => resolve(img);
    img.onerror = reject;
    img.src = "data:image/png;base64," + b64;
  });
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

btnCapture.addEventListener("click", () => {
  if (busy) { setStatus(t("msgPrevBusy"), "err"); return; }
  invoke("begin_capture");
});

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

function imgFilter() {
  return {
    filters: [{ name: t("filterImage"), extensions: ["png", "jpg", "jpeg", "bmp", "tif", "tiff", "webp", "gif"] }],
  };
}

btnOpen.addEventListener("click", async () => {
  const path = await open({ multiple: false, ...imgFilter() });
  if (typeof path === "string") await runOcr({ source: { kind: "file", path } });
});

btnPaste.addEventListener("click", () => runOcr({ source: { kind: "clipboard" } }));

btnCopy.addEventListener("click", async () => {
  if (!txtResult.value) return;
  await invoke("copy_text", { text: txtResult.value });
  setStatus(t("msgCopied"), "ok");
});

btnSave.addEventListener("click", async () => {
  if (!txtResult.value) return;
  const path = await save({
    filters: [
      { name: t("filterText"), extensions: ["txt"] },
      { name: t("filterMd"), extensions: ["md"] },
      { name: t("filterJson"), extensions: ["json"] },
    ],
  });
  if (path) {
    await invoke("save_text", { path, text: txtResult.value });
    setStatus(t("msgSaved") + path, "ok");
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
  if (typeof s["ok"] === "number" && typeof s["total"] === "number") {
    setStatus(`${s["ok"]}/${s["total"]}`, s["state"] === "ready" ? "ok" : "");
  } else {
    setStatus(String(s["message"] ?? s["state"] ?? ""), s["state"] === "ready" ? "ok" : "");
  }
});

// ---------------------------------------------------------------------------
// Belge sekmesi (görsel + PDF/DOCX/XLSX/PPTX/UDF/metin)
// ---------------------------------------------------------------------------

const docPath = $<HTMLInputElement>("doc-path");
const docResult = $<HTMLTextAreaElement>("doc-result");
const docStatus = $("doc-status");
let docBusy = false;
// Tam metin bellekte durur; görünüm çok büyük belgelerde kısaltılır (donmayı önler).
let docFullText = "";
const DOC_VIEW_LIMIT = 300_000;

function setDocResult(full: string) {
  docFullText = full;
  if (full.length > DOC_VIEW_LIMIT) {
    docResult.value = full.slice(0, DOC_VIEW_LIMIT) + "\n\n" + tFmt("msgViewCut", { n: full.length });
  } else {
    docResult.value = full;
  }
}

function clearDoc() {
  docPath.value = "";
  docFullText = "";
  docResult.value = "";
  setDocStatus("");
}

function setDocStatus(msg: string, cls: "" | "ok" | "err" = "") {
  docStatus.textContent = msg;
  docStatus.className = "status " + cls;
}

function docFilter() {
  return {
    filters: [{
      name: t("filterDocs"),
      extensions: ["png", "jpg", "jpeg", "bmp", "tif", "tiff", "webp",
        "pdf", "docx", "xlsx", "pptx", "udf", "txt", "md"],
    }],
  };
}

$<HTMLButtonElement>("btn-doc-browse").addEventListener("click", async () => {
  const sel = await open({ multiple: false, ...docFilter() });
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
    setDocResult(r.plainText);
    const extra = r.kind === "image" && r.words.length
      ? ` · ${r.words.length} ${t("uWords")}`
      : ` · ${r.plainText.length} ${t("uChars")}`;
    setDocStatus(`${r.engine} · ${r.elapsedMs} ${t("uMs")}${extra}`, "ok");
  } catch (e) {
    setDocStatus(String(e), "err");
  } finally {
    docBusy = false;
  }
});

$<HTMLButtonElement>("btn-doc-clear").addEventListener("click", clearDoc);

$<HTMLButtonElement>("btn-doc-copy").addEventListener("click", async () => {
  if (!docFullText) return;
  await invoke("copy_text", { text: docFullText });
});

$<HTMLButtonElement>("btn-doc-save").addEventListener("click", async () => {
  if (!docFullText) return;
  const path = await save({
    filters: [
      { name: t("filterText"), extensions: ["txt"] },
      { name: t("filterMd"), extensions: ["md"] },
    ],
  });
  if (path) {
    await invoke("save_text", { path, text: docFullText });
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
    filters: [{ name: t("filterUdf"), extensions: ["udf"] }],
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
  const sel = await open({ multiple: true, ...docFilter() });
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
// Web sekmesi (Google Sheets içe aktarma)
// ---------------------------------------------------------------------------

interface SheetResult {
  path: string;
  name: string;
  rows: number;
  cols: number;
  format: string;
}

interface WebDetect {
  kind: string;
  label: string;
  formats: string[];
  detail?: string | null;
}

const FORMAT_LABELS: Record<string, string> = {
  csv: "CSV", xlsx: "Excel (XLSX)", md: "Markdown",
  docx: "Word (DOCX)", odt: "ODT", txt: "TXT", pdf: "PDF",
};

let webKind = "";

async function detectWebKind() {
  const badge = $("web-kind");
  const url = webUrl.value.trim();
  if (!url) {
    badge.textContent = "";
    webKind = "";
    return;
  }
  try {
    const d = await invoke<WebDetect>("detect_web_url", { url });
    webKind = d.kind;
    badge.textContent = "📄 " + d.label;
    badge.className = "status " + (d.formats.length ? "ok" : "err");
    if (d.detail && !d.formats.length) setWebStatus(d.detail, "err");
    if (d.formats.length) {
      const prev = selWebFormat.value;
      selWebFormat.innerHTML = "";
      for (const f of d.formats) {
        const o = document.createElement("option");
        o.value = f;
        o.textContent = FORMAT_LABELS[f] ?? f.toUpperCase();
        selWebFormat.appendChild(o);
      }
      if (d.formats.includes(prev)) selWebFormat.value = prev;
    }
  } catch (e) {
    webKind = "";
    badge.textContent = "";
  }
}

let webDetectTimer = 0;
function bindWebDetect() {
  webUrl.addEventListener("input", () => {
    if (webDetectTimer) window.clearTimeout(webDetectTimer);
    webDetectTimer = window.setTimeout(detectWebKind, 500);
  });
}

const webUrl = $<HTMLInputElement>("web-url");
const webStatus = $("web-status");
const webOutputs = $("web-outputs");
const selWebFormat = $<HTMLSelectElement>("sel-web-format");
let webOutDir = "";
let webBusy = false;
bindWebDetect();

function setWebStatus(msg: string, cls: "" | "ok" | "err" = "") {
  webStatus.textContent = msg;
  webStatus.className = "status " + cls;
}

function clearWeb() {
  webOutputs.innerHTML = "";
  setWebStatus("");
}

$<HTMLButtonElement>("btn-web-out").addEventListener("click", async () => {
  const sel = await open({ directory: true, multiple: false });
  if (typeof sel === "string") {
    webOutDir = sel;
    ($("web-outdir") as HTMLElement).textContent = sel;
  }
});

$<HTMLButtonElement>("btn-web-clear").addEventListener("click", () => {
  webUrl.value = "";
  ($("web-outdir") as HTMLElement).textContent = "…";
  webOutDir = "";
  clearWeb();
});

$<HTMLButtonElement>("btn-web-run").addEventListener("click", async () => {
  if (webBusy || !webUrl.value.trim()) return;
  if (!webOutDir) {
    const sel = await open({ directory: true, multiple: false });
    if (typeof sel !== "string") return;
    webOutDir = sel;
    ($("web-outdir") as HTMLElement).textContent = sel;
  }
  webBusy = true;
  ($<HTMLButtonElement>("btn-web-run")).disabled = true;
  setWebStatus(t("working"));
  try {
    // Tur onceden saptanmamissa simdi saptanir.
    if (!webKind) await detectWebKind();
    const url = webUrl.value.trim();
    const fmt = selWebFormat.value;
    webOutputs.innerHTML = "";
    if (webKind === "doc") {
      const r = await invoke<SheetResult>("import_doc_url", {
        url, outDir: webOutDir, format: fmt,
      });
      setWebStatus(tFmt("webDone", { name: `${r.name}.${r.format}`, path: r.path }), "ok");
      showWebOutput(r.path);
    } else if (webKind === "file") {
      const dto = await invoke<BatchResult>("import_direct_url", {
        url, outDir: webOutDir, format: fmt,
      });
      setWebStatus(`${dto.okCount}/${dto.items.length} → ${dto.outDir}`, dto.failCount ? "err" : "ok");
      webOutputs.innerHTML = "";
      for (const it of dto.items) {
        const div = document.createElement("div");
        div.textContent = it.ok ? `✓ ${it.name}` : `✗ ${it.name}: ${it.error ?? ""}`;
        if (it.ok) div.className = "fout";
        webOutputs.appendChild(div);
      }
      if (dto.combinedPath) {
        const div = document.createElement("div");
        div.className = "fout";
        div.textContent = "📦 " + dto.combinedPath;
        webOutputs.appendChild(div);
      }
    } else if (webKind === "sheet") {
      const r = await invoke<SheetResult>("import_sheet_url", {
        url, outDir: webOutDir, format: fmt,
      });
      setWebStatus(tFmt("webTable", { name: `${r.name}.${r.format}`, rows: r.rows, cols: r.cols, path: r.path }), "ok");
      showWebOutput(r.path);
    } else {
      setWebStatus(
        uiLang === "tr"
          ? "Önce geçerli bir bağlantı girin (Google Tablosu/Belgesi veya dosya)."
          : "Enter a valid link first (Google Sheet/Doc or file).",
        "err",
      );
    }
  } catch (e) {
    setWebStatus(String(e), "err");
  } finally {
    webBusy = false;
    ($<HTMLButtonElement>("btn-web-run")).disabled = false;
  }
});

function showWebOutput(path: string) {
  const div = document.createElement("div");
  div.className = "fout";
  div.textContent = "📦 " + path;
  webOutputs.appendChild(div);
}

// ---------------------------------------------------------------------------
// Tam ekran resim duzenleyici (kirp / dondur / parlaklik / yansit / cizim / zemin)
// ---------------------------------------------------------------------------

let edIdx = -1;
let edTool: "crop" | "pen" | "box" | null = null;
let edUndo: ImageData[] = [];
let edFilterBase: ImageData | null = null;
let edDown: { x: number; y: number } | null = null;
let edLast: { x: number; y: number } | null = null;

function edCanvas(): HTMLCanvasElement {
  return $<HTMLCanvasElement>("editor-canvas");
}

function edCtx(): CanvasRenderingContext2D {
  return edCanvas().getContext("2d")!;
}

function edPushUndo() {
  try {
    const c = edCanvas();
    edUndo.push(edCtx().getImageData(0, 0, c.width, c.height));
    if (edUndo.length > 20) edUndo.shift();
  } catch { /* yoksay */ }
}

function edSetTool(tool: "crop" | "pen" | "box" | null) {
  edTool = tool;
  document.querySelectorAll<HTMLButtonElement>("#editor-bar [data-ed]").forEach((b) => {
    const on = b.dataset.ed === tool
      || (tool === null && false);
    b.classList.toggle("on", b.dataset.ed === tool);
    void on;
  });
  edCanvas().style.cursor = tool ? "crosshair" : "default";
}

function openEditor(idx: number) {
  const L = srcLayers[idx];
  if (!L) return;
  edIdx = idx;
  edUndo = [];
  edFilterBase = null;
  edSetTool("crop");
  ($<HTMLInputElement>("ed-bright")).value = "100";
  ($<HTMLInputElement>("ed-contrast")).value = "100";
  loadImg(L.b64).then((img) => {
    const c = edCanvas();
    c.width = img.naturalWidth;
    c.height = img.naturalHeight;
    edCtx().drawImage(img, 0, 0);
    ($("editor") as HTMLElement).hidden = false;
  }).catch((e) => setStatus(String(e), "err"));
}

function closeEditor() {
  ($("editor") as HTMLElement).hidden = true;
  edIdx = -1;
  edTool = null;
  edUndo = [];
  edFilterBase = null;
}

function edPos(e: MouseEvent): { x: number; y: number } {
  const c = edCanvas();
  const r = c.getBoundingClientRect();
  const sx = c.width / Math.max(1, r.width);
  const sy = c.height / Math.max(1, r.height);
  return {
    x: Math.max(0, Math.min(c.width - 1, Math.round((e.clientX - r.left) * sx))),
    y: Math.max(0, Math.min(c.height - 1, Math.round((e.clientY - r.top) * sy))),
  };
}

function edRedrawClean() {
  // Son kayitli durumu geri koy (gecici cerceveler temizlenir).
  const last = edUndo.length ? edUndo[edUndo.length - 1] : null;
  if (last) {
    const c = edCanvas();
    if (last.width !== c.width || last.height !== c.height) {
      c.width = last.width;
      c.height = last.height;
    }
    edCtx().putImageData(last, 0, 0);
  }
}

document.querySelectorAll<HTMLButtonElement>("#editor-bar [data-ed]").forEach((b) => {
  const kind = b.dataset.ed;
  if (kind !== "crop" && kind !== "pen" && kind !== "box") return;
  b.addEventListener("click", () => {
    const tool = kind as "crop" | "pen" | "box";
    edSetTool(edTool === tool ? null : tool);
  });
});

edCanvas().addEventListener("mousedown", (e) => {
  if (e.button !== 0 || !edTool) return;
  edDown = edPos(e);
  edLast = edDown;
  edPushUndo(); // tum araclar: once temiz durum kaydi
  edFilterBase = null;
});

edCanvas().addEventListener("mousemove", (e) => {
  if (!edDown || !edTool) return;
  const p = edPos(e);
  const c = edCanvas();
  const ctx = edCtx();
  if (edTool === "pen") {
    ctx.strokeStyle = ($<HTMLInputElement>("ed-color")).value;
    ctx.lineWidth = Number(($<HTMLInputElement>("ed-width")).value) || 3;
    ctx.lineCap = "round";
    ctx.beginPath();
    ctx.moveTo(edLast!.x, edLast!.y);
    ctx.lineTo(p.x, p.y);
    ctx.stroke();
    edLast = p;
    return;
  }
  // Kirp + kutu: gecici cerceve (tabani her karede geri yukle).
  if (!edDown) return;
  const snap = edUndo.length ? edUndo[edUndo.length - 1] : null;
  if (edTool === "crop") {
    // Gecici gosterge icin mevcut canvas korunur; asil kirpma mouseup'ta.
    drawEdOverlay(p);
    void snap;
    return;
  }
  if (edTool === "box" && snap) {
    ctx.putImageData(snap, 0, 0);
    const x = Math.min(edDown.x, p.x);
    const y = Math.min(edDown.y, p.y);
    const w = Math.abs(p.x - edDown.x);
    const h = Math.abs(p.y - edDown.y);
    ctx.strokeStyle = ($<HTMLInputElement>("ed-color")).value;
    ctx.lineWidth = Number(($<HTMLInputElement>("ed-width")).value) || 3;
    ctx.strokeRect(x, y, Math.max(1, w), Math.max(1, h));
    edLast = p;
  }
});

function drawEdOverlay(p: { x: number; y: number }) {
  edRedrawClean();
  if (!edDown) return;
  const ctx = edCtx();
  const x = Math.min(edDown.x, p.x);
  const y = Math.min(edDown.y, p.y);
  const w = Math.abs(p.x - edDown.x);
  const h = Math.abs(p.y - edDown.y);
  ctx.save();
  ctx.strokeStyle = "#5B8CFF";
  ctx.lineWidth = 2;
  ctx.setLineDash([6, 4]);
  ctx.strokeRect(x, y, Math.max(1, w), Math.max(1, h));
  ctx.restore();
}

edCanvas().addEventListener("mouseup", (e) => {
  if (!edDown || !edTool) { edDown = null; return; }
  const p = edPos(e);
  const tool = edTool;
  const d = { ...edDown };
  edDown = null;
  edLast = null;
  if (tool === "crop") {
    const x = Math.min(d.x, p.x);
    const y = Math.min(d.y, p.y);
    const w = Math.abs(p.x - d.x);
    const h = Math.abs(p.y - d.y);
    if (w < 4 || h < 4) {
      edRedrawClean();
      return;
    }
    edRedrawClean(); // kesik cizgiyi sil, temiz kareden kes (undo mousedown'da var)
    const c = edCanvas();
    const cut = edCtx().getImageData(x, y, w, h);
    c.width = w;
    c.height = h;
    edCtx().putImageData(cut, 0, 0);
    edFilterBase = null;
  }
  // pen/box: cizim zaten islendi (undo mouseup'ta alinmisti).
});

function edBorder() {
  const c = edCanvas();
  edPushUndo();
  const ctx = edCtx();
  const w = Number(($<HTMLInputElement>("ed-width")).value) || 3;
  ctx.save();
  ctx.strokeStyle = ($<HTMLInputElement>("ed-color")).value || "#ff0000";
  ctx.lineWidth = Math.max(1, w);
  const inset = Math.max(1, w) / 2;
  ctx.strokeRect(inset, inset, c.width - inset * 2, c.height - inset * 2);
  ctx.restore();
  edFilterBase = null;
}

function edRotate(dir: 1 | -1) {
  const c = edCanvas();
  edPushUndo();
  const tmp = document.createElement("canvas");
  tmp.width = c.width;
  tmp.height = c.height;
  tmp.getContext("2d")!.drawImage(c, 0, 0);
  c.width = tmp.height;
  c.height = tmp.width;
  const ctx = edCtx();
  ctx.translate(c.width / 2, c.height / 2);
  ctx.rotate(dir * Math.PI / 2);
  ctx.drawImage(tmp, -tmp.width / 2, -tmp.height / 2);
  edFilterBase = null;
}

function edFlipH() {
  const c = edCanvas();
  edPushUndo();
  const tmp = document.createElement("canvas");
  tmp.width = c.width;
  tmp.height = c.height;
  tmp.getContext("2d")!.drawImage(c, 0, 0);
  const ctx = edCtx();
  ctx.save();
  ctx.translate(c.width, 0);
  ctx.scale(-1, 1);
  ctx.drawImage(tmp, 0, 0);
  ctx.restore();
  edFilterBase = null;
}

function edApplyFilter() {
  const b = Number(($<HTMLInputElement>("ed-bright")).value) || 100;
  const k = Number(($<HTMLInputElement>("ed-contrast")).value) || 100;
  if (!edFilterBase) {
    try {
      const c = edCanvas();
      edFilterBase = edCtx().getImageData(0, 0, c.width, c.height);
      edPushUndo();
    } catch { return; }
  } else {
    edCtx().putImageData(edFilterBase, 0, 0);
  }
  const c = edCanvas();
  const tmp = document.createElement("canvas");
  tmp.width = c.width;
  tmp.height = c.height;
  const tctx = tmp.getContext("2d")!;
  tctx.filter = `brightness(${b}%) contrast(${k}%)`;
  tctx.drawImage(c, 0, 0);
  edCtx().clearRect(0, 0, c.width, c.height);
  edCtx().drawImage(tmp, 0, 0);
}

$<HTMLButtonElement>("ed-undo").addEventListener("click", () => {
  const prev = edUndo.pop();
  if (prev) {
    const c = edCanvas();
    if (prev.width !== c.width || prev.height !== c.height) {
      c.width = prev.width;
      c.height = prev.height;
    }
    edCtx().putImageData(prev, 0, 0);
    edFilterBase = null;
  }
});

$<HTMLButtonElement>("ed-apply").addEventListener("click", () => {
  if (edIdx < 0 || !srcLayers[edIdx]) { closeEditor(); return; }
  const src = edCanvas();
  // Secili zemin rengine duzlestir (saydam PNG'ler icin).
  const flat = document.createElement("canvas");
  flat.width = src.width;
  flat.height = src.height;
  const fctx = flat.getContext("2d")!;
  fctx.fillStyle = ($<HTMLInputElement>("ed-bg")).value || "#ffffff";
  fctx.fillRect(0, 0, flat.width, flat.height);
  fctx.drawImage(src, 0, 0);
  const url = flat.toDataURL("image/png");
  const L = srcLayers[edIdx];
  L.b64 = url.slice(url.indexOf(",") + 1);
  const img = new Image();
  img.onload = () => {
    L.nw = img.naturalWidth;
    L.nh = img.naturalHeight;
    const s = Math.min(1, 560 / Math.max(L.nw, L.nh));
    L.w = Math.max(32, Math.round(L.nw * s));
    L.h = Math.max(32, Math.round(L.nh * s));
    renderLayers();
    closeEditor();
  };
  img.onerror = () => closeEditor();
  img.src = url;
});

$<HTMLButtonElement>("ed-cancel").addEventListener("click", closeEditor);

{
  const rl = document.querySelector<HTMLButtonElement>('#editor-bar [data-ed="rotl"]');
  const rr = document.querySelector<HTMLButtonElement>('#editor-bar [data-ed="rotr"]');
  const fh = document.querySelector<HTMLButtonElement>('#editor-bar [data-ed="fliph"]');
  const bd = document.querySelector<HTMLButtonElement>('#editor-bar [data-ed="border"]');
  rl?.addEventListener("click", () => edRotate(-1));
  rr?.addEventListener("click", () => edRotate(1));
  fh?.addEventListener("click", edFlipH);
  bd?.addEventListener("click", edBorder);
  ($<HTMLInputElement>("ed-bright")).addEventListener("input", edApplyFilter);
  ($<HTMLInputElement>("ed-contrast")).addEventListener("input", edApplyFilter);
  document.addEventListener("keydown", (e) => {
    if (!($("editor") as HTMLElement).hidden && e.key === "Escape") closeEditor();
  });
}

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
    if (!info.pythonOk) missing.push(t("reqPython"));
    if (!info.scriptOk) missing.push(t("reqScript"));
    if (!info.ffmpegOk) missing.push(t("reqFfmpeg"));
    if (!info.tesseractOk) missing.push(t("reqTess"));
    videoReq.textContent = missing.length ? "⚠ " + missing.join(" · ") : t("reqReady");
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
// Tümünü temizle (tepsi menüsü) + sürükle-bırak
// ---------------------------------------------------------------------------

function currentView(): string {
  const active = document.querySelector(".tab.active") as HTMLElement | null;
  return active?.dataset.view ?? "yakala";
}

function clearCapture() {
  txtResult.value = "";
  lastImageBase64 = "";
  imgWrap.classList.remove("has-img");
  imgWrap.innerHTML = `<span id="img-empty">${t("imgEmpty")}</span>`;
  setStatus("");
}

function clearBatch() {
  batchPaths = [];
  batchOutputs.innerHTML = "";
  batchBar.style.width = "0%";
  setBatchStatus("");
  renderBatchFiles();
}

function clearUdf() {
  udfPaths = [];
  udfOutputs.innerHTML = "";
  udfBar.style.width = "0%";
  setUdfStatus("");
  renderUdfFiles();
}

function clearVideo() {
  videoPaths = [];
  videoOutputs.innerHTML = "";
  videoLog.textContent = "";
  videoBar.style.width = "0%";
  videoStatus.textContent = "";
  renderVideoFiles();
}

listen("clear-all", () => {
  clearCapture();
  clearDoc();
  clearBatch();
  clearUdf();
  clearWeb();
  clearVideo();
});

const IMG_EXTS = ["png", "jpg", "jpeg", "bmp", "tif", "tiff", "webp", "gif"];
const DOC_EXTS = [...IMG_EXTS, "pdf", "docx", "xlsx", "pptx", "udf", "txt", "md"];
const VIDEO_EXTS = ["mp4", "mov", "avi", "mkv", "webm", "m4v", "wmv", "flv"];

function extOf(p: string): string {
  const m = p.toLowerCase().match(/\.([a-z0-9]+)$/);
  return m ? m[1] : "";
}

function dropNotSupported(view: string, n: number) {
  const msg = tFmt("msgDropSkip", { view, n });
  if (view === "yakala") setStatus(msg, "err");
  else if (view === "belge") setDocStatus(msg, "err");
  else if (view === "toplu") setBatchStatus(msg, "err");
  else if (view === "udf") setUdfStatus(msg, "err");
  else { videoStatus.textContent = msg; videoStatus.className = "status err"; }
}

listen<{ paths: string[] }>("tauri://drag-drop", async (ev) => {
  const paths = (ev.payload?.paths ?? []).filter((p) => typeof p === "string");
  if (paths.length === 0) return;
  const view = currentView();
  if (view === "yakala") {
    const img = paths.find((p) => IMG_EXTS.includes(extOf(p)));
    if (!img) { dropNotSupported(view, paths.length); return; }
    await runOcr({ source: { kind: "file", path: img } });
  } else if (view === "belge") {
    const doc = paths.find((p) => DOC_EXTS.includes(extOf(p)));
    if (!doc) { dropNotSupported(view, paths.length); return; }
    docPath.value = doc;
    ($<HTMLButtonElement>("btn-doc-run")).click();
  } else if (view === "toplu") {
    const ok = paths.filter((p) => DOC_EXTS.includes(extOf(p)));
    ok.forEach((p) => { if (!batchPaths.includes(p)) batchPaths.push(p); });
    renderBatchFiles();
    if (ok.length < paths.length) dropNotSupported(view, paths.length - ok.length);
  } else if (view === "udf") {
    const ok = paths.filter((p) => extOf(p) === "udf");
    ok.forEach((p) => { if (!udfPaths.includes(p)) udfPaths.push(p); });
    renderUdfFiles();
    if (ok.length < paths.length) dropNotSupported(view, paths.length - ok.length);
  } else if (view === "video") {
    const ok = paths.filter((p) => VIDEO_EXTS.includes(extOf(p)));
    ok.forEach((p) => { if (!videoPaths.includes(p)) videoPaths.push(p); });
    renderVideoFiles();
    if (ok.length < paths.length) dropNotSupported(view, paths.length - ok.length);
  }
});

// ---------------------------------------------------------------------------
// Başlatma
// ---------------------------------------------------------------------------

applyI18n();
invoke("set_ui_lang", { lang: uiLang }).catch(() => {});
refreshEngines();
refreshEngineStatus();
refreshMonitors();
refreshVideoReq();
renderBatchFiles();
renderVideoFiles();
renderUdfFiles();
refreshOcrLangs();
pushOptions();
txtResult.placeholder = t("imgEmpty");
