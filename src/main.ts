import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open, save } from "@tauri-apps/plugin-dialog";
import { getCurrentWindow } from "@tauri-apps/api/window";

// Pencere X: editor aciksa once onu kapat (uygulamayi tepsiye gizleme).
getCurrentWindow().onCloseRequested(async (e) => {
  if (!($("editor") as HTMLElement).hidden) {
    e.preventDefault();
    closeEditor();
  }
});

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
  combinedPaths?: string[];
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
    btnBatchAdd: "Dosya ekle…", btnBatchAddDir: "Klasör ekle…", btnBatchOutDir: "Çıktı klasörü…",
    batchNoFiles: "Dosya yok", batchDirEmpty: "Klasörde desteklenen dosya yok.", batchDirAdded: "klasörden eklendi",
    lblFormat: "Format", lblSaveMode: "Kayıt", lblRecursive: "Alt klasörleri de ara",
    lblSplit: "Bölme", lblSplitValue: "Parça / MB",
    optSplitNone: "Bölme yok", optSplitCount: "Parça sayısı", optSplitSize: "MB eşiği",
    optSeparate: "Ayrı ayrı", optCombined: "Birleştir",
    btnBatchRun: "Toplu başlat", btnClear: "Temizle",
    btnBatchManage: "Yönet…", lblGroupByExt: "Dosya formatına göre kaydet",
    manageTitle: "Dosyaları yönet", manageSearchPh: "Dosya ara…",
    manageSelectAll: "Tümünü seç", manageSelectNone: "Hiçbirini seçme",
    manageTypes: "Formatlar", manageSelected: "{n} / {total} seçili",
    manageAiOnly: "AI'ın okuyamadıkları",
    manageAiOnlyTip: "Yalnızca AI agentlarının doğrudan açamadığı formatları seçer (PDF, Office, resim, arşiv…).",
    manageHint: "İşaretli dosyalar kalır.", manageEmpty: "Eşleşen dosya yok.",
    manageNote: "Uyarı: Seçili {n} dosya zaten okunabilir bir metin formatında; yine de OCR çalıştırılacak ve gereksiz işlem olacak.",
    btnClose: "Kapat", btnCancel: "Vazgeç", btnOk: "Tamam",
    ctxOpenFolder: "Dosyanın bulunduğu klasöre git",
    tabVerify: "Doğrulama",
    hashTip: "Dosyaların özet (hash) kodlarını üretir: MD5, SHA-1, SHA-256, CRC32. Belge doğrulamada SHA-256 standarttır; MD5/SHA-1 eski ve zayıftır, CRC32 yalnızca bütünlük kontrolüdür.",
    btnHashAdd: "Dosya ekle…", btnHashAddDir: "Klasör ekle…", lblHashAlgos: "Hash",
    lblHashGroup: "Aynı adlı farklı uzantıları yan yana karşılaştır (mesut.pdf ↔ mesut.md)",
    btnHashRun: "Hash listesi oluştur", btnHashSave: "Raporu kaydet…",
    cmpTitle: "İki klasörü karşılaştır",
    cmpTip: "Aynı dosyanın iki klasördeki hash'i farklıysa dosya değişmiş demektir; rapora uyarı notu eklenir.",
    btnCmpA: "Klasör A seç…", btnCmpB: "Klasör B seç…", btnCmpRun: "Karşılaştır", btnCmpSave: "Raporu kaydet…",
    hashComputing: "Hash hesaplanıyor…", hashNeedAlgo: "En az bir hash algoritması seçin.",
    hashNeedRun: "Önce hash listesini oluşturun.", cmpNeedBoth: "İki klasör seçin.",
    hashReportTitle: "Hash Raporu", cmpReportTitle: "Klasör Karşılaştırma Raporu",
    lblGroup: "Grup", lblFile: "Dosya", lblSize: "Boyut", lblPath: "Yol", lblStatus: "Durum",
    colWarning: "Uyarı", warnHashDiffRow: "HASH FARKLI",
    warnDiff: "UYARI: {n} dosyanın hash'i farklı — dosya değişmiş olabilir.",
    stSame: "AYNI", stDiff: "FARKLI", stOnlyA: "SADECE A", stOnlyB: "SADECE B", stError: "HATA",
    videoDesc: "Kaydırılan ekran kayıtlarından metin çıkarır: ffmpeg kare örnekleme → Tesseract → scroll tekrarı eleme → CSV/TXT.",
    btnVideoAdd: "Video Ekle…", videoNoFiles: "Henüz video seçilmedi.",
    webTip: "Herkese açık Google Sheets bağlantısını yapıştırın; CSV / XLSX / Markdown olarak kaydedilir.",
    webUrlPh: "https://docs.google.com/spreadsheets/d/…",
    btnWebRun: "İçe Aktar",
    lblScroll: "Scroll hızı", optAuto: "Otomatik", optFast: "Hızlı scroll", optSlow: "Yavaş scroll",
    lblQuality: "Kalite / Hız", optFastQ: "Hızlı (~120 kare)", optBalanced: "Dengeli (~180 kare)",
    optAccurate: "Yüksek (~280 kare)", btnVideoRun: "Videodan Çıkar",
    tabLive: "Canlı OCR",
    liveDesc: "Ekrandan canlı kayıt alıp yazıları çıkarır: bölge / pencere / tam ekran → ffmpeg kaydı → Tesseract → CSV/TXT/MD/XLSX.",
    lblSource: "Kaynak", optRegion: "Bölge seç", optWindow: "Pencere seç", optScreen: "Tam ekran",
    lblWindow: "Pencere", optNoWindow: "—", btnLivePick: "Bölge Seç…", btnLiveWindows: "Pencereleri Yenile",
    lblRegion: "Bölge:", lblAudio: "Ses", optNoAudio: "Ses yok", lblKeepVideo: "Videoyu da sakla",
    btnLiveStart: "Kaydı Başlat", btnLiveStop: "Kaydı Durdur",
    liveNoWindows: "Uygun pencere bulunamadı.", liveRegionSet: "bölge hazır",
    liveRecording: "● Kaydediliyor…", liveExtracting: "Metin çıkarılıyor…", liveDone: "Tamamlandı",
    liveNeedOut: "Önce çıktı klasörü seçin.",
    menuReOcr: "Motor ile yeniden OCR", menuCopyImg: "📋 Kopyala (resim)", menuEdit: "✏️ Düzenle",
    menuRestore: "↩ Orijinali Geri Yükle",
    menuDelete: "🗑 Sil", menuDeleteAll: "🗑 Tümünü Sil",
    menuPaste: "📋 Yapıştır", menuAll: "Tümü", menuLayer: "Katman", working: "Çalışıyor…",
    optPsmAuto: "Otomatik", optPsmBlock: "Tek blok", optPsmLine: "Tek satır", optPsmSparse: "Seyrek metin",
    optScaleNone: "Yok", optScale2: "2× Büyüt", optScale3: "3× Büyüt",
    zoomTip: "Yakınlaştırma: %{p} (sıfırlamak için çift tık)",
    engTesseract: "Tesseract (gömülü)", engWindows: "Windows OCR (sistem)", engMissing: "(yok)",
    engLangNone: "Tesseract bulunamadı.", engNoLangPack: "dil paketi yok", engLangsPrefix: "dil: ",
    linkEngineDl: "Tesseract 5 indir ↗",
    titleLangTr: "Türkçe", titleLangEn: "English", titleTheme: "Tema",
    uAreas: "alan", uMs: "ms", uWords: "kelime", uChars: "karakter", uRows: "satır", uCols: "sütun", uSec: "sn",
    stConf: "· güven %{p}",
    msgCopiedImg: "Resim panoya kopyalandı.", msgCopied: "Panoya kopyalandı.", msgSaved: "Kaydedildi: ",
    msgPasted: "Panodan yapıştırıldı.", msgRestored: "Orijinal geri yüklendi.",
    msgPrevBusy: "Önceki işlem sürüyor, bitmesini bekleyin.",
    msgViewCut: "…[görünüm kısaltıldı: {n} karakterin tamamı kopyala/kaydet ile alınabilir]",
    msgEnterLink: "Önce geçerli bir bağlantı girin (Google Tablosu/Belgesi veya dosya).",
    msgPickUdf: "Yalnız .udf dosyası seçin.",
    msgError: "hata",
    reqPython: "Python yok", reqScript: "betik yok", reqFfmpeg: "ffmpeg yok", reqTess: "Tesseract yok",
    reqReady: "✓ python · ffmpeg · tesseract hazır",
    filterImage: "Görsel", filterDocs: "Belgeler", filterUdf: "UYAP UDF", filterVideo: "Video",
    filterTess: "Tesseract", filterText: "Metin", filterMd: "Markdown", filterJson: "JSON",
    webDone: "✓ {name} → {path}", webTable: "✓ {name} · {rows} satır × {cols} sütun → {path}",
    msgDropSkip: "{view}: desteklenmeyen dosya ({n} atlandı)",
    edTitle: "Düzenle", edCrop: "Kırp", edFlip: "Yansıt", edPen: "Kalem", edBox: "Kutu",
    edBorder: "Çerçeve", edGrpSelect: "Seçim", edGrpTransform: "Dönüştür", edGrpDraw: "Çizim",
    edGrpLight: "Işık", edColor: "Renk", edWidth: "Kalınlık",
    edBorderDone: "Çerçeve eklendi ({w}px).",
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
    btnBatchAdd: "Add files…", btnBatchAddDir: "Add folder…", btnBatchOutDir: "Output folder…",
    batchNoFiles: "No files", batchDirEmpty: "No supported files in folder.", batchDirAdded: "added from folder",
    lblFormat: "Format", lblSaveMode: "Save", lblRecursive: "Include subfolders",
    lblSplit: "Split", lblSplitValue: "Parts / MB",
    optSplitNone: "No split", optSplitCount: "Part count", optSplitSize: "MB threshold",
    optSeparate: "Separate", optCombined: "Combined",
    btnBatchRun: "Start batch", btnClear: "Clear",
    btnBatchManage: "Manage…", lblGroupByExt: "Save by file type",
    manageTitle: "Manage files", manageSearchPh: "Search files…",
    manageSelectAll: "Select all", manageSelectNone: "Select none",
    manageTypes: "Formats", manageSelected: "{n} / {total} selected",
    manageAiOnly: "Unreadable by AI",
    manageAiOnlyTip: "Selects only formats AI agents cannot open directly (PDF, Office, images, archives…).",
    manageHint: "Checked files are kept.", manageEmpty: "No matching files.",
    manageNote: "Warning: {n} selected file(s) are already in a readable text format; OCR will still run and do unnecessary work.",
    btnClose: "Close", btnCancel: "Cancel", btnOk: "OK",
    ctxOpenFolder: "Open containing folder",
    tabVerify: "Verify",
    hashTip: "Generates checksums (hashes) for files: MD5, SHA-1, SHA-256, CRC32. SHA-256 is the standard for document verification; MD5/SHA-1 are legacy/weak, CRC32 is integrity-only.",
    btnHashAdd: "Add files…", btnHashAddDir: "Add folder…", lblHashAlgos: "Hash",
    lblHashGroup: "Compare same-name different-extension files side by side (mesut.pdf ↔ mesut.md)",
    btnHashRun: "Build hash list", btnHashSave: "Save report…",
    cmpTitle: "Compare two folders",
    cmpTip: "If the same file has different hashes in the two folders, it has changed; a warning note is added to the report.",
    btnCmpA: "Choose folder A…", btnCmpB: "Choose folder B…", btnCmpRun: "Compare", btnCmpSave: "Save report…",
    hashComputing: "Computing hash…", hashNeedAlgo: "Select at least one hash algorithm.",
    hashNeedRun: "Build the hash list first.", cmpNeedBoth: "Choose both folders.",
    hashReportTitle: "Hash Report", cmpReportTitle: "Folder Comparison Report",
    lblGroup: "Group", lblFile: "File", lblSize: "Size", lblPath: "Path", lblStatus: "Status",
    colWarning: "Warning", warnHashDiffRow: "HASH DIFFERS",
    warnDiff: "WARNING: {n} file(s) have different hashes — the file may have changed.",
    stSame: "SAME", stDiff: "DIFFERENT", stOnlyA: "ONLY A", stOnlyB: "ONLY B", stError: "ERROR",
    videoDesc: "Extract text from scrolled screen recordings: ffmpeg sampling → Tesseract → scroll dedup → CSV/TXT.",
    btnVideoAdd: "Add videos…", videoNoFiles: "No videos selected.",
    webTip: "Paste a public Google Sheets link; save as CSV / XLSX / Markdown.",
    webUrlPh: "https://docs.google.com/spreadsheets/d/…",
    btnWebRun: "Import",
    lblScroll: "Scroll speed", optAuto: "Auto", optFast: "Fast scroll", optSlow: "Slow scroll",
    lblQuality: "Quality / Speed", optFastQ: "Fast (~120 frames)", optBalanced: "Balanced (~180 frames)",
    optAccurate: "High (~280 frames)", btnVideoRun: "Extract from video",
    tabLive: "Live OCR",
    liveDesc: "Record the screen live and extract text: region / window / fullscreen → ffmpeg capture → Tesseract → CSV/TXT/MD/XLSX.",
    lblSource: "Source", optRegion: "Pick region", optWindow: "Pick window", optScreen: "Fullscreen",
    lblWindow: "Window", optNoWindow: "—", btnLivePick: "Pick Region…", btnLiveWindows: "Refresh Windows",
    lblRegion: "Region:", lblAudio: "Audio", optNoAudio: "No audio", lblKeepVideo: "Keep video too",
    btnLiveStart: "Start Recording", btnLiveStop: "Stop Recording",
    liveNoWindows: "No suitable window found.", liveRegionSet: "region ready",
    liveRecording: "● Recording…", liveExtracting: "Extracting text…", liveDone: "Done",
    liveNeedOut: "Pick an output folder first.",
    menuReOcr: "Re-OCR with engine", menuCopyImg: "📋 Copy (image)", menuEdit: "✏️ Edit",
    menuRestore: "↩ Restore Original",
    menuDelete: "🗑 Delete", menuDeleteAll: "🗑 Delete all",
    menuPaste: "📋 Paste", menuAll: "All", menuLayer: "Layer", working: "Working…",
    optPsmAuto: "Auto", optPsmBlock: "Single block", optPsmLine: "Single line", optPsmSparse: "Sparse text",
    optScaleNone: "None", optScale2: "2× Upscale", optScale3: "3× Upscale",
    zoomTip: "Zoom: {p}% (double-click to reset)",
    engTesseract: "Tesseract (embedded)", engWindows: "Windows OCR (system)", engMissing: "(missing)",
    engLangNone: "Tesseract not found.", engNoLangPack: "no language pack", engLangsPrefix: "langs: ",
    linkEngineDl: "Download Tesseract 5 ↗",
    titleLangTr: "Turkish", titleLangEn: "English", titleTheme: "Theme",
    uAreas: "areas", uMs: "ms", uWords: "words", uChars: "chars", uRows: "rows", uCols: "cols", uSec: "s",
    stConf: "· {p}% conf.",
    msgCopiedImg: "Image copied.", msgCopied: "Copied.", msgSaved: "Saved: ",
    msgPasted: "Pasted from clipboard.", msgRestored: "Original restored.",
    msgPrevBusy: "Previous task still running.",
    msgViewCut: "…[view truncated: full {n} chars available via copy/save]",
    msgEnterLink: "Enter a valid link first (Google Sheet/Doc or file).",
    msgPickUdf: "Please pick a .udf file.",
    msgError: "error",
    reqPython: "no Python", reqScript: "no script", reqFfmpeg: "no ffmpeg", reqTess: "no Tesseract",
    reqReady: "✓ python · ffmpeg · tesseract ready",
    filterImage: "Images", filterDocs: "Documents", filterUdf: "UYAP UDF", filterVideo: "Video",
    filterTess: "Tesseract", filterText: "Text", filterMd: "Markdown", filterJson: "JSON",
    webDone: "✓ {name} → {path}", webTable: "✓ {name} · {rows} rows × {cols} cols → {path}",
    msgDropSkip: "{view}: unsupported file ({n} skipped)",
    edTitle: "Edit", edCrop: "Crop", edFlip: "Flip", edPen: "Pen", edBox: "Box",
    edBorder: "Border", edGrpSelect: "Select", edGrpTransform: "Transform", edGrpDraw: "Draw",
    edGrpLight: "Light", edColor: "Color", edWidth: "Width",
    edBorderDone: "Border added ({w}px).",
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
  document.querySelectorAll<HTMLElement>("[data-i18n-title]").forEach((el) => {
    const k = el.dataset.i18nTitle!;
    if (k in I18N[uiLang]) el.title = I18N[uiLang][k];
  });
  $("btn-lang-tr").classList.toggle("active", uiLang === "tr");
  $("btn-lang-en").classList.toggle("active", uiLang === "en");
  document.documentElement.lang = uiLang;
}

$("btn-lang-tr").addEventListener("click", () => { setUiLang("tr"); });
$("btn-lang-en").addEventListener("click", () => { setUiLang("en"); });

// Harici bağlantılar (yazar LinkedIn/GitHub, Tesseract indir): sistem tarayıcısında aç.
document.addEventListener("click", (ev) => {
  const el = (ev.target as HTMLElement | null)?.closest?.("[data-open]") as HTMLElement | null;
  if (!el) return;
  ev.preventDefault();
  const url = el.getAttribute("data-open") || "";
  if (url) invoke("open_url", { url }).catch(() => {});
});

async function setUiLang(l: Lang) {
  uiLang = l;
  persistUiLang();
  applyI18n();
  try {
    await invoke("set_ui_lang", { lang: l });
  } catch { /* varsayılan tr kalır */ }
  refreshOcrLangs();
  refreshEngines();
  refreshDynamicTexts();
}

/// Dil degisiminde dinamik cizilmis metinleri guncelle
/// (dosya listeleri, bos-alan yazilari, placeholder'lar).
function refreshDynamicTexts() {
  renderBatchFiles();
  renderUdfFiles();
  renderVideoFiles();
  renderLiveRegion(lastLiveRect);
  if (!lastImageBase64) {
    imgWrap.innerHTML = `<span id="img-empty">${t("imgEmpty")}</span>`;
  }
  txtResult.placeholder = t("imgEmpty");
  if (selLiveWindow.options.length) selLiveWindow.options[0].textContent = t("optNoWindow");
  if (selLiveAudio.options.length) selLiveAudio.options[0].textContent = t("optNoAudio");
  refreshVideoReq();
  refreshEngineStatus();
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
  canli: $("view-canli"),
  hash: $("view-hash"),
};
document.querySelectorAll<HTMLButtonElement>(".tab").forEach((b) =>
  b.addEventListener("click", () => {
    document.querySelectorAll(".tab").forEach((x) => x.classList.toggle("active", x === b));
    const v = b.dataset.view!;
    for (const [k, el] of Object.entries(views)) el.hidden = k !== v;
    // Canli sekmesi acilinca pencere/ses listelerini tazele (yeni acilan uygulamalar).
    if (v === "canli" && !liveRecording) {
      refreshLiveWindows();
      refreshLiveAudio();
    }
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
  if (d === "dil paketi yok") return t("engNoLangPack");
  if (d.startsWith("dil: ")) return t("engLangsPrefix") + d.slice(5);
  return d;
}

function engineLabel(e: EngineInfo): string {
  const base = engineName(e) + (e.available ? "" : " " + t("engMissing"));
  const det = engineDetail(e);
  // Yol yazma (secim kutusunu sidirir); yol ipucunda durur.
  const isPath = /[\\/]/.test(det);
  return det && !isPath ? `${base} - ${det}` : base;
}

async function refreshEngines() {
  try {
    engines = await invoke<EngineInfo[]>("list_engines");
    selEngine.innerHTML = "";
    for (const e of engines) {
      const o = document.createElement("option");
      o.value = e.id;
      o.textContent = engineLabel(e);
      o.title = e.detail ?? "";
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
  orig: string;
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
  srcLayers.push({ id: layerSeq++, b64, orig: b64, x: off, y: off, w, h, nw, nh });
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
    // Duzenlenmisse orijinali geri yukleme secenegi.
    if (srcLayers[idx] && srcLayers[idx].b64 !== srcLayers[idx].orig) {
      const rs = document.createElement("button");
      rs.textContent = t("menuRestore");
      rs.addEventListener("click", () => {
        engineMenu.hidden = true;
        const L = srcLayers[idx];
        if (!L) return;
        L.b64 = L.orig;
        const img = new Image();
        img.onload = () => {
          L.nw = img.naturalWidth;
          L.nh = img.naturalHeight;
          renderLayers();
          setStatus(t("msgRestored"), "ok");
        };
        img.onerror = () => renderLayers();
        img.src = "data:image/png;base64," + L.orig;
      });
      engineMenu.appendChild(rs);
    }
    const del = document.createElement("button");
    del.textContent = t("menuDelete");
    del.addEventListener("click", () => {
      engineMenu.hidden = true;
      deleteLayer(idx);
    });
    engineMenu.appendChild(del);
  } else {
    const sep = document.createElement("div");
    sep.className = "menu-sep";
    engineMenu.appendChild(sep);
    const delAll = document.createElement("button");
    delAll.textContent = t("menuDeleteAll");
    delAll.addEventListener("click", () => {
      engineMenu.hidden = true;
      deleteAllLayers();
    });
    engineMenu.appendChild(delAll);
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

// Secili katmani siler (Delete tusu + sag-tik Sil).
function deleteLayer(i: number) {
  if (i < 0 || i >= srcLayers.length) return;
  srcLayers.splice(i, 1);
  if (selectedLayer >= srcLayers.length) selectedLayer = srcLayers.length - 1;
  renderLayers();
}

function deleteAllLayers() {
  srcLayers = [];
  selectedLayer = -1;
  renderLayers();
  txtResult.value = "";
  setStatus("");
}

// Delete: kaynak gorunumunde secili katmani sil.
document.addEventListener("keydown", (e) => {
  const tag = (e.target as HTMLElement)?.tagName ?? "";
  if (tag === "TEXTAREA" || tag === "INPUT" || tag === "SELECT") return;
  if (currentView() !== "yakala") return;
  if (e.key === "Delete" || e.key === "Backspace") {
    if (selectedLayer < 0 || !srcLayers[selectedLayer]) return;
    e.preventDefault();
    deleteLayer(selectedLayer);
    return;
  }
  // Ctrl+V: panodaki resmi katman olarak ekle.
  if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "v") {
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
      orig: docs[i].image_png_base64,
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
listen<string>("editor-blocked", (ev) => setStatus(ev.payload, "err"));
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
const docHashEl = $<HTMLElement>("doc-hash");
const docResult = $<HTMLTextAreaElement>("doc-result");
const docStatus = $("doc-status");
let docBusy = false;
let docHashSeq = 0;
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
  refreshDocHash();
}

async function refreshDocHash() {
  const path = docPath.value.trim();
  const seq = ++docHashSeq;
  if (!path) {
    docHashEl.hidden = true;
    docHashEl.textContent = "";
    return;
  }
  docHashEl.hidden = false;
  docHashEl.textContent = t("hashComputing");
  try {
    const res = await invoke<FileHash[]>("hash_files", { files: [path], algos: ["sha256", "md5"] });
    if (seq !== docHashSeq) return;
    const h = res[0];
    if (!h || h.error) {
      docHashEl.textContent = h?.error ?? t("msgError");
      return;
    }
    docHashEl.textContent = `SHA-256: ${h.hashes["sha256"] ?? "—"}  ·  MD5: ${h.hashes["md5"] ?? "—"}`;
  } catch (e) {
    if (seq === docHashSeq) docHashEl.textContent = String(e);
  }
}

function setDocStatus(msg: string, cls: "" | "ok" | "err" = "") {
  docStatus.textContent = msg;
  docStatus.className = "status " + cls;
}

function docFilter() {
  return {
    filters: [{
      name: t("filterDocs"),
      extensions: ["png", "jpg", "jpeg", "bmp", "tif", "tiff", "webp", "gif",
        "pdf", "docx", "xlsx", "pptx", "udf", "odt", "ods", "odp", "epub",
        "rtf", "txt", "md", "markdown", "csv", "tsv", "json", "jsonl", "ndjson",
        "log", "xml", "html", "htm",
        "sql", "srt", "vtt", "ini", "cfg", "yaml", "yml", "toml",
        "ps1", "bat", "cmd", "sh"],
    }],
  };
}

$<HTMLButtonElement>("btn-doc-browse").addEventListener("click", async () => {
  const sel = await open({ multiple: false, ...docFilter() });
  if (typeof sel === "string") {
    docPath.value = sel;
    setDocStatus("");
    refreshDocHash();
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
    if (dto.combinedPaths?.length || dto.combinedPath) {
      renderCombined(dto.combinedPaths, dto.combinedPath, udfOutputs);
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
    attachCtx(div, p);
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

// ---------------------------------------------------------------------------
// Dosya sağ tık menüsü (bulunduğu klasörü aç)
// ---------------------------------------------------------------------------
const fileCtx = $<HTMLElement>("file-ctx");
const ctxOpenFolder = $<HTMLButtonElement>("ctx-open-folder");
let ctxPath = "";

function openFileCtx(x: number, y: number, path: string) {
  ctxPath = path;
  fileCtx.hidden = false;
  const r = fileCtx.getBoundingClientRect();
  const px = Math.max(6, Math.min(x, window.innerWidth - r.width - 6));
  const py = Math.max(6, Math.min(y, window.innerHeight - r.height - 6));
  fileCtx.style.left = `${px}px`;
  fileCtx.style.top = `${py}px`;
}
function closeFileCtx() {
  fileCtx.hidden = true;
  ctxPath = "";
}
function attachCtx(el: HTMLElement, path: string) {
  el.addEventListener("contextmenu", (e) => {
    e.preventDefault();
    openFileCtx(e.clientX, e.clientY, path);
  });
}
ctxOpenFolder.addEventListener("click", () => {
  const p = ctxPath;
  closeFileCtx();
  if (p) invoke("show_in_folder", { path: p }).catch(() => {});
});
document.addEventListener("click", closeFileCtx);
document.addEventListener("scroll", closeFileCtx, true);
window.addEventListener("blur", closeFileCtx);

// ---------------------------------------------------------------------------
// Toplu: dosya yönetimi (modal) — arama, tek tek / format bazında seçim
// ---------------------------------------------------------------------------
let manageKeep = new Set<string>();
let manageQuery = "";

function fileExtOf(p: string): string {
  const base = shortName(p);
  const i = base.lastIndexOf(".");
  return i > 0 ? base.slice(i + 1).toLowerCase() : "";
}

// AI agentlari (opencode, Cursor vb.) bu ikili formatlari dogrudan okuyamaz;
// metne/Markdown'a cevrilmeleri gerekir. Duz metinler (md, txt, csv, tsv, ...) okunabilir.
const AI_BINARY_EXTS = new Set([
  "pdf", "doc", "docx", "xls", "xlsx", "ppt", "pptx",
  "odt", "ods", "odp", "rtf", "epub", "udf",
  "png", "jpg", "jpeg", "bmp", "tif", "tiff", "webp", "gif", "heic", "heif", "avif",
  "zip", "rar", "7z", "tar", "gz", "bz2", "xz", "iso", "dmg", "exe", "dll", "msi", "bin",
]);

function isAiUnreadable(p: string): boolean {
  return AI_BINARY_EXTS.has(fileExtOf(p));
}

function manageOpen() {
  manageKeep = new Set(batchPaths.filter(isAiUnreadable));
  if (manageKeep.size === 0) manageKeep = new Set(batchPaths);
  manageQuery = "";
  manageResetShift();
  $<HTMLInputElement>("inp-manage-search").value = "";
  renderManageModal();
  $<HTMLElement>("batch-manage-modal").hidden = false;
  $<HTMLInputElement>("inp-manage-search").focus();
}

function manageClose() {
  $<HTMLElement>("batch-manage-modal").hidden = true;
}

// Pencereyi basligindan tutup surukleme.
let manageDrag: { id: number; sx: number; sy: number; ox: number; oy: number } | null = null;
let manageShiftX = 0;
let manageShiftY = 0;
const manageDialog = $<HTMLElement>("batch-manage-modal").querySelector(".modal") as HTMLElement;
const manageHead = manageDialog.querySelector(".modal-head") as HTMLElement;

function manageResetShift() {
  manageShiftX = 0;
  manageShiftY = 0;
  manageDialog.style.transform = "";
}

manageHead.addEventListener("pointerdown", (e) => {
  if (e.button !== 0 || (e.target as HTMLElement).closest("button")) return;
  manageDrag = { id: e.pointerId, sx: e.clientX, sy: e.clientY, ox: manageShiftX, oy: manageShiftY };
  manageHead.setPointerCapture(e.pointerId);
  manageDialog.classList.add("dragging");
  e.preventDefault();
});
manageHead.addEventListener("pointermove", (e) => {
  if (!manageDrag || e.pointerId !== manageDrag.id) return;
  manageShiftX = manageDrag.ox + (e.clientX - manageDrag.sx);
  manageShiftY = manageDrag.oy + (e.clientY - manageDrag.sy);
  manageDialog.style.transform = `translate(${manageShiftX}px, ${manageShiftY}px)`;
});
function manageEndDrag(e: PointerEvent) {
  if (!manageDrag || e.pointerId !== manageDrag.id) return;
  manageDrag = null;
  manageDialog.classList.remove("dragging");
}
manageHead.addEventListener("pointerup", manageEndDrag);
manageHead.addEventListener("pointercancel", manageEndDrag);

function renderManageModal() {
  const filesEl = $<HTMLElement>("manage-files");
  const typesEl = $<HTMLElement>("manage-types-list");

  const groups = new Map<string, { total: number; keep: number }>();
  for (const p of batchPaths) {
    const e = fileExtOf(p) || "—";
    const g = groups.get(e) ?? { total: 0, keep: 0 };
    g.total++;
    if (manageKeep.has(p)) g.keep++;
    groups.set(e, g);
  }

  typesEl.innerHTML = "";
  [...groups.entries()]
    .sort((a, b) => b[1].total - a[1].total || a[0].localeCompare(b[0]))
    .forEach(([ext, g]) => {
      const row = document.createElement("label");
      row.className = "type-row";
      const cb = document.createElement("input");
      cb.type = "checkbox";
      cb.checked = g.keep === g.total;
      cb.indeterminate = g.keep > 0 && g.keep < g.total;
      cb.addEventListener("change", () => {
        for (const p of batchPaths) {
          if ((fileExtOf(p) || "—") !== ext) continue;
          if (cb.checked) manageKeep.add(p);
          else manageKeep.delete(p);
        }
        renderManageModal();
      });
      const nameEl = document.createElement("span");
      nameEl.className = "type-name";
      nameEl.textContent = ext === "—" ? "—" : "." + ext;
      const cnt = document.createElement("span");
      cnt.className = "type-count";
      cnt.textContent = String(g.total);
      row.append(cb, nameEl, cnt);
      typesEl.appendChild(row);
    });
  if (!groups.size) {
    typesEl.innerHTML = `<span class="muted">${t("manageEmpty")}</span>`;
  }

  filesEl.innerHTML = "";
  const q = manageQuery.trim().toLowerCase();
  let shown = 0;
  for (const p of batchPaths) {
    const nm = shortName(p);
    if (q && !nm.toLowerCase().includes(q) && !p.toLowerCase().includes(q)) continue;
    shown++;
    const row = document.createElement("label");
    row.className = "manage-file";
    const cb = document.createElement("input");
    cb.type = "checkbox";
    cb.checked = manageKeep.has(p);
    cb.addEventListener("change", () => {
      if (cb.checked) manageKeep.add(p);
      else manageKeep.delete(p);
      renderManageModal();
    });
    const nameEl = document.createElement("span");
    nameEl.className = "name";
    nameEl.textContent = nm;
    nameEl.title = p;
    const extEl = document.createElement("span");
    extEl.className = "ext";
    extEl.textContent = fileExtOf(p) || "—";
    row.append(cb, nameEl, extEl);
    attachCtx(row, p);
    filesEl.appendChild(row);
  }
  if (!shown) {
    filesEl.innerHTML = `<span class="muted">${t("manageEmpty")}</span>`;
  }
  $<HTMLElement>("manage-count").textContent = tFmt("manageSelected", {
    n: manageKeep.size,
    total: batchPaths.length,
  });
  updateManageNote();
}

function updateManageNote() {
  const el = $<HTMLElement>("manage-note");
  let readable = 0;
  for (const p of manageKeep) {
    if (!isAiUnreadable(p)) readable++;
  }
  el.hidden = readable === 0;
  if (readable) el.textContent = tFmt("manageNote", { n: readable });
}

$<HTMLButtonElement>("btn-batch-manage").addEventListener("click", manageOpen);
$<HTMLButtonElement>("btn-manage-close").addEventListener("click", manageClose);
$<HTMLButtonElement>("btn-manage-cancel").addEventListener("click", manageClose);
$<HTMLButtonElement>("btn-manage-ok").addEventListener("click", () => {
  batchPaths = batchPaths.filter((p) => manageKeep.has(p));
  manageClose();
  renderBatchFiles();
});
$<HTMLButtonElement>("btn-manage-all").addEventListener("click", () => {
  manageKeep = new Set(batchPaths);
  renderManageModal();
});
$<HTMLButtonElement>("btn-manage-none").addEventListener("click", () => {
  manageKeep = new Set();
  renderManageModal();
});
$<HTMLButtonElement>("btn-types-all").addEventListener("click", () => {
  manageKeep = new Set(batchPaths);
  renderManageModal();
});
$<HTMLButtonElement>("btn-types-none").addEventListener("click", () => {
  manageKeep = new Set();
  renderManageModal();
});
$<HTMLButtonElement>("btn-manage-ai").addEventListener("click", () => {
  manageKeep = new Set(batchPaths.filter(isAiUnreadable));
  renderManageModal();
});
$<HTMLInputElement>("inp-manage-search").addEventListener("input", (e) => {
  manageQuery = (e.target as HTMLInputElement).value;
  renderManageModal();
});
$<HTMLElement>("batch-manage-modal").addEventListener("click", (e) => {
  if (e.target === $<HTMLElement>("batch-manage-modal")) manageClose();
});
document.addEventListener("keydown", (e) => {
  if (e.key === "Escape" && !$<HTMLElement>("batch-manage-modal").hidden) manageClose();
});

function renderCombined(paths: string[] | undefined, old: string | null | undefined, out: HTMLElement) {
  const list = (paths && paths.length ? paths : (old ? [old] : []));
  for (const p of list) {
    const div = document.createElement("div");
    div.className = "fout";
    div.textContent = "📦 " + p;
    out.appendChild(div);
  }
}

$<HTMLButtonElement>("btn-batch-add").addEventListener("click", async () => {
  const sel = await open({ multiple: true, ...docFilter() });
  if (Array.isArray(sel)) batchPaths.push(...sel.filter((s) => !batchPaths.includes(s)));
  else if (typeof sel === "string" && !batchPaths.includes(sel)) batchPaths.push(sel);
  renderBatchFiles();
});

$<HTMLButtonElement>("btn-batch-add-dir").addEventListener("click", async () => {
  const sel = await open({ directory: true, multiple: true });
  if (!sel) return;
  const dirs = (Array.isArray(sel) ? sel : [sel]).filter((s) => typeof s === "string") as string[];
  if (!dirs.length) return;
  const recursive = ($("chk-batch-recursive") as HTMLInputElement).checked;
  try {
    const found = await invoke<string[]>("expand_batch_dirs", { dirs, recursive });
    const fresh = found.filter((s) => !batchPaths.includes(s));
    batchPaths.push(...fresh);
    setBatchStatus(fresh.length ? `${fresh.length} ${t("batchDirAdded")}` : t("batchDirEmpty"), fresh.length ? "ok" : "err");
  } catch (e) {
    setBatchStatus(String(e), "err");
  }
  renderBatchFiles();
});

function syncSplitInput() {
  const mode = ($("sel-batch-split") as HTMLSelectElement).value;
  const inp = $("inp-batch-split") as HTMLInputElement;
  const combined = ($("sel-batch-mode") as HTMLSelectElement).value === "combined";
  inp.disabled = !combined || mode === "none";
  if (mode === "size") {
    inp.min = "1"; inp.max = "4096"; inp.step = "1";
    if (Number(inp.value) < 1 || Number(inp.value) > 4096) inp.value = "15";
  } else {
    inp.min = "2"; inp.max = "50"; inp.step = "1";
    if (Number(inp.value) < 2 || Number(inp.value) > 50) inp.value = "4";
  }
  ($("sel-batch-split") as HTMLSelectElement).disabled = !combined;
}
$("sel-batch-split")?.addEventListener("change", syncSplitInput);
$("sel-batch-mode")?.addEventListener("change", syncSplitInput);
syncSplitInput();

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
    const splitMode = ($("sel-batch-split") as HTMLSelectElement).value;
    const splitVal = Number(($("inp-batch-split") as HTMLInputElement).value);
    const dto = await invoke<BatchResult>("batch_process_files", {
      files: batchPaths,
      outDir: batchOutDir,
      format: selBatchFormat.value,
      saveMode: selBatchMode.value,
      fileTitle: true,
      splitMode,
      splitCount: splitMode === "count" ? Math.round(splitVal) : null,
      splitMb: splitMode === "size" ? splitVal : null,
      groupByExt: ($("chk-batch-group") as HTMLInputElement).checked,
    });
    batchBar.style.width = "100%";
    setBatchStatus(`${dto.okCount}/${dto.items.length} → ${dto.outDir}`, dto.failCount ? "err" : "ok");
    for (const it of dto.items) {
      const div = document.createElement("div");
      div.textContent = it.ok ? `✓ ${it.name} (${it.chars})` : `✗ ${it.name}: ${it.error ?? ""}`;
      if (it.ok) div.className = "fout";
      batchOutputs.appendChild(div);
    }
    if (dto.combinedPaths?.length || dto.combinedPath) {
      renderCombined(dto.combinedPaths, dto.combinedPath, batchOutputs);
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
// Doğrulama sekmesi: dosya hash listesi + iki klasör karşılaştırma
// ---------------------------------------------------------------------------

interface FileHash {
  path: string;
  name: string;
  size: number;
  hashes: Record<string, string>;
  error: string | null;
}
interface CompareEntry {
  rel_path: string;
  status: "same" | "different" | "only_a" | "only_b" | "error";
  a: FileHash | null;
  b: FileHash | null;
}
interface CompareResult {
  dir_a: string;
  dir_b: string;
  same: number;
  different: number;
  only_a: number;
  only_b: number;
  error: number;
  entries: CompareEntry[];
}

const HASH_ORDER = ["sha256", "sha1", "md5", "crc32", "sha512"];
const hashFilesEl = $<HTMLElement>("hash-files");
const hashResultsEl = $<HTMLElement>("hash-results");
const hashStatus = $<HTMLElement>("hash-status");
const hashBar = $<HTMLElement>("hash-progress-bar");
const cmpResultsEl = $<HTMLElement>("cmp-results");
const cmpAEl = $<HTMLElement>("cmp-a");
const cmpBEl = $<HTMLElement>("cmp-b");

let hashPaths: string[] = [];
let hashBusy = false;
let hashAlgosUsed: string[] = ["sha256"];
let hashRows: FileHash[] = [];
let cmpA = "";
let cmpB = "";
let cmpBusy = false;
let cmpData: CompareResult | null = null;
let cmpAlgosUsed: string[] = ["sha256"];

function setHashStatus(msg: string, cls: "" | "ok" | "err" = "") {
  hashStatus.textContent = msg;
  hashStatus.className = "status " + cls;
}

function hashAlgoLabel(a: string): string {
  return a === "sha256" ? "SHA-256"
    : a === "sha1" ? "SHA-1"
    : a === "md5" ? "MD5"
    : a === "crc32" ? "CRC32"
    : "SHA-512";
}

function selectedHashAlgos(): string[] {
  const out: string[] = [];
  if ($<HTMLInputElement>("chk-hash-sha256").checked) out.push("sha256");
  if ($<HTMLInputElement>("chk-hash-sha1").checked) out.push("sha1");
  if ($<HTMLInputElement>("chk-hash-md5").checked) out.push("md5");
  if ($<HTMLInputElement>("chk-hash-crc32").checked) out.push("crc32");
  return out;
}

function orderedKeys(algos: string[]): string[] {
  return HASH_ORDER.filter((a) => algos.includes(a));
}

function humanSize(n: number): string {
  if (n < 1024) return `${n} B`;
  const units = ["KB", "MB", "GB", "TB"];
  let v = n / 1024;
  let i = 0;
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024;
    i++;
  }
  return `${v.toFixed(v >= 10 ? 0 : 1)} ${units[i]}`;
}

function stemOf(p: string): string {
  const b = shortName(p);
  const i = b.lastIndexOf(".");
  return i > 0 ? b.slice(0, i) : b;
}

function groupKeyOf(p: string): string {
  const dir = p.replace(/[\\/][^\\/]*$/, "").toLowerCase();
  return `${dir}|${stemOf(p).toLowerCase()}`;
}

function renderHashFiles() {
  hashFilesEl.innerHTML = hashPaths.length ? "" : `<span class="muted">${t("batchNoFiles")}</span>`;
  hashPaths.forEach((p, i) => {
    const div = document.createElement("div");
    div.className = "batch-item";
    const name = document.createElement("span");
    name.className = "name";
    name.textContent = `${i + 1}. ${shortName(p)}`;
    name.title = p;
    const rm = document.createElement("button");
    rm.type = "button";
    rm.className = "rm";
    rm.dataset.hrm = String(i);
    rm.textContent = "×";
    div.append(name, rm);
    attachCtx(div, p);
    hashFilesEl.appendChild(div);
  });
}

hashFilesEl.addEventListener("click", (e) => {
  const b = (e.target as HTMLElement).closest("[data-hrm]") as HTMLElement | null;
  if (!b) return;
  hashPaths.splice(Number(b.dataset.hrm), 1);
  renderHashFiles();
});

$<HTMLButtonElement>("btn-hash-add").addEventListener("click", async () => {
  const sel = await open({ multiple: true });
  const list = Array.isArray(sel) ? sel : typeof sel === "string" ? [sel] : [];
  for (const p of list) {
    if (!hashPaths.includes(p)) hashPaths.push(p);
  }
  renderHashFiles();
});

$<HTMLButtonElement>("btn-hash-add-dir").addEventListener("click", async () => {
  const sel = await open({ directory: true, multiple: true });
  if (!sel) return;
  const dirs = (Array.isArray(sel) ? sel : [sel]).filter((s) => typeof s === "string") as string[];
  if (!dirs.length) return;
  const recursive = $<HTMLInputElement>("chk-hash-recursive").checked;
  try {
    const found = await invoke<string[]>("expand_all_dirs", { dirs, recursive });
    let added = 0;
    for (const p of found) {
      if (!hashPaths.includes(p)) {
        hashPaths.push(p);
        added++;
      }
    }
    setHashStatus(added ? `${added} ${t("batchDirAdded")}` : t("batchDirEmpty"), added ? "ok" : "err");
  } catch (e) {
    setHashStatus(String(e), "err");
  }
  renderHashFiles();
});

$<HTMLButtonElement>("btn-hash-clear").addEventListener("click", () => {
  clearHashView();
});

function fmtHashes(h: FileHash, algos: string[]): string {
  return orderedKeys(algos)
    .map((a) => `${hashAlgoLabel(a)}: ${h.hashes[a] ?? "—"}`)
    .join("  ·  ");
}

function renderHashResults(rows: FileHash[]) {
  hashResultsEl.innerHTML = "";
  const grouping = $<HTMLInputElement>("chk-hash-group").checked;
  const head = document.createElement("div");
  head.className = "muted";
  head.textContent = `${rows.length} ${t("lblFile").toLowerCase()} · ${orderedKeys(hashAlgosUsed).map(hashAlgoLabel).join(", ")}`;
  hashResultsEl.appendChild(head);

  if (grouping) {
    const groups = new Map<string, FileHash[]>();
    for (const r of rows) {
      const k = groupKeyOf(r.path);
      const arr = groups.get(k) ?? [];
      arr.push(r);
      groups.set(k, arr);
    }
    for (const arr of groups.values()) {
      if (arr.length > 1) {
        const g = document.createElement("div");
        g.className = "hgroup";
        g.textContent = `↔ ${arr.map((r) => r.name).join("  ↔  ")}`;
        hashResultsEl.appendChild(g);
      }
      for (const r of arr) {
        const d = document.createElement("div");
        d.className = "hrow" + (r.error ? " diff" : "");
        d.textContent = `  ${r.name} (${humanSize(r.size)}) — ${r.error ? r.error : fmtHashes(r, hashAlgosUsed)}`;
        d.title = r.path;
        hashResultsEl.appendChild(d);
      }
    }
  } else {
    for (const r of rows) {
      const d = document.createElement("div");
      d.className = "hrow" + (r.error ? " diff" : "");
      d.textContent = `${r.name} (${humanSize(r.size)}) — ${r.error ? r.error : fmtHashes(r, hashAlgosUsed)}`;
      d.title = r.path;
      hashResultsEl.appendChild(d);
    }
  }
}

function nowStamp(): string {
  const d = new Date();
  const p = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())} ${p(d.getHours())}:${p(d.getMinutes())}:${p(d.getSeconds())}`;
}

function csvCell(v: string | number): string {
  const s = String(v);
  return /[",\r\n]/.test(s) ? `"${s.replace(/"/g, '""')}"` : s;
}

function hashReportRows(): { group: string; r: FileHash }[] {
  const grouping = $<HTMLInputElement>("chk-hash-group").checked;
  const counts = new Map<string, number>();
  if (grouping) {
    for (const r of hashRows) {
      const k = groupKeyOf(r.path);
      counts.set(k, (counts.get(k) ?? 0) + 1);
    }
  }
  return hashRows.map((r) => ({
    group: grouping && (counts.get(groupKeyOf(r.path)) ?? 0) > 1 ? stemOf(r.path) : "",
    r,
  }));
}

function buildHashReport(format: string): string {
  const cols = orderedKeys(hashAlgosUsed);
  const grouping = $<HTMLInputElement>("chk-hash-group").checked;
  const rows = hashReportRows();

  if (format === "csv") {
    const head = [
      ...(grouping ? [t("lblGroup")] : []),
      t("lblFile"), t("lblPath"), t("lblSize"), ...cols.map(hashAlgoLabel),
    ];
    const lines = [head.map(csvCell).join(",")];
    for (const { group, r } of rows) {
      lines.push([
        ...(grouping ? [group] : []),
        r.name, r.path, r.size, ...cols.map((a) => r.hashes[a] ?? ""),
      ].map(csvCell).join(","));
    }
    return lines.join("\r\n") + "\r\n";
  }

  if (format === "txt") {
    const head = [
      ...(grouping ? [t("lblGroup")] : []),
      t("lblFile"), t("lblPath"), t("lblSize"), ...cols.map(hashAlgoLabel),
    ].join("\t");
    const lines = [`${t("hashReportTitle")}\t${nowStamp()}`, head];
    for (const { group, r } of rows) {
      lines.push([
        ...(grouping ? [group] : []),
        r.name, r.path, r.size, ...cols.map((a) => r.hashes[a] ?? ""),
      ].join("\t"));
    }
    return lines.join("\r\n") + "\r\n";
  }

  const head = [
    ...(grouping ? [t("lblGroup")] : []),
    t("lblFile"), t("lblPath"), t("lblSize"), ...cols.map(hashAlgoLabel),
  ];
  const lines = [
    `# ${t("hashReportTitle")}`,
    "",
    `${nowStamp()} · ${hashRows.length} ${t("lblFile").toLowerCase()} · ${cols.map(hashAlgoLabel).join(", ")}`,
    "",
    `| ${head.join(" | ")} |`,
    `|${head.map(() => "---").join("|")}|`,
  ];
  for (const { group, r } of rows) {
    const vals = [
      ...(grouping ? [group] : []),
      r.name, r.path, r.size, ...cols.map((a) => r.hashes[a] ?? ""),
    ].map((v) => String(v).replace(/\|/g, "\\|"));
    lines.push(`| ${vals.join(" | ")} |`);
  }
  const failed = hashRows.filter((r) => r.error).length;
  if (failed) lines.push("", `> ${t("stError")}: ${failed}`);
  return lines.join("\n") + "\n";
}

async function runHashList() {
  if (hashBusy || hashPaths.length === 0) return;
  const algos = selectedHashAlgos();
  if (!algos.length) {
    setHashStatus(t("hashNeedAlgo"), "err");
    return;
  }
  hashBusy = true;
  $<HTMLButtonElement>("btn-hash-run").disabled = true;
  hashResultsEl.innerHTML = "";
  hashBar.style.width = "0%";
  setHashStatus(t("working"));
  try {
    const res = await invoke<FileHash[]>("hash_files", { files: hashPaths, algos });
    hashRows = res;
    hashAlgosUsed = algos;
    renderHashResults(res);
    const failed = res.filter((r) => r.error).length;
    setHashStatus(`${res.length - failed}/${res.length}`, failed ? "err" : "ok");
  } catch (e) {
    setHashStatus(String(e), "err");
  } finally {
    hashBusy = false;
    $<HTMLButtonElement>("btn-hash-run").disabled = false;
    hashBar.style.width = "100%";
  }
}

$<HTMLButtonElement>("btn-hash-run").addEventListener("click", runHashList);

$<HTMLButtonElement>("btn-hash-save").addEventListener("click", async () => {
  if (!hashRows.length) {
    setHashStatus(t("hashNeedRun"), "err");
    return;
  }
  const fmt = $<HTMLSelectElement>("sel-hash-format").value;
  const path = await save({
    defaultPath: `hash-raporu.${fmt}`,
    filters: [{ name: fmt.toUpperCase(), extensions: [fmt] }],
  });
  if (typeof path !== "string") return;
  try {
    await invoke("save_text", { path, text: buildHashReport(fmt) });
    setHashStatus(t("msgSaved") + path, "ok");
  } catch (e) {
    setHashStatus(String(e), "err");
  }
});

$<HTMLInputElement>("chk-hash-group").addEventListener("change", () => {
  if (hashRows.length) renderHashResults(hashRows);
});

listen<Record<string, unknown>>("hash-progress", (ev) => {
  const p = ev.payload;
  const i = Number(p["index"] ?? 0);
  const n = Number(p["total"] ?? 0);
  hashBar.style.width = n > 0 ? `${(i / n) * 100}%` : "0%";
});

$<HTMLButtonElement>("btn-cmp-a").addEventListener("click", async () => {
  const sel = await open({ directory: true, multiple: false });
  if (typeof sel === "string") {
    cmpA = sel;
    cmpAEl.textContent = sel;
    cmpAEl.title = sel;
  }
});

$<HTMLButtonElement>("btn-cmp-b").addEventListener("click", async () => {
  const sel = await open({ directory: true, multiple: false });
  if (typeof sel === "string") {
    cmpB = sel;
    cmpBEl.textContent = sel;
    cmpBEl.title = sel;
  }
});

function statusLabel(s: string): string {
  if (s === "same") return t("stSame");
  if (s === "different") return t("stDiff");
  if (s === "only_a") return t("stOnlyA");
  if (s === "only_b") return t("stOnlyB");
  return t("stError");
}

function sideHashes(h: FileHash | null, algos: string[]): string {
  if (!h) return "—";
  if (h.error) return `(${h.error})`;
  return orderedKeys(algos)
    .map((a) => `${hashAlgoLabel(a)}=${h.hashes[a] ?? ""}`)
    .join("; ");
}

function renderCompareResults(res: CompareResult) {
  cmpResultsEl.innerHTML = "";
  const head = document.createElement("div");
  head.className = "muted";
  head.textContent = `${res.same} ${t("stSame")} · ${res.different} ${t("stDiff")} · ${res.only_a} ${t("stOnlyA")} · ${res.only_b} ${t("stOnlyB")}${res.error ? ` · ${res.error} ${t("stError")}` : ""}`;
  cmpResultsEl.appendChild(head);
  for (const e of res.entries) {
    const d = document.createElement("div");
    d.className = "hrow" + (e.status === "same" ? " ok" : e.status === "different" || e.status === "error" ? " diff" : " only");
    let text = `${statusLabel(e.status)} · ${e.rel_path}`;
    if (e.status !== "same") {
      text += ` · A[${sideHashes(e.a, cmpAlgosUsed)}] B[${sideHashes(e.b, cmpAlgosUsed)}]`;
    }
    d.textContent = text;
    d.title = `A: ${e.a?.path ?? "—"}\nB: ${e.b?.path ?? "—"}`;
    if (e.a) attachCtx(d, e.a.path);
    else if (e.b) attachCtx(d, e.b.path);
    cmpResultsEl.appendChild(d);
  }
}

async function runCompare() {
  if (cmpBusy) return;
  if (!cmpA || !cmpB) {
    setHashStatus(t("cmpNeedBoth"), "err");
    return;
  }
  const algos = selectedHashAlgos();
  if (!algos.length) {
    setHashStatus(t("hashNeedAlgo"), "err");
    return;
  }
  const recursive = $<HTMLInputElement>("chk-cmp-recursive").checked;
  cmpBusy = true;
  $<HTMLButtonElement>("btn-cmp-run").disabled = true;
  cmpResultsEl.innerHTML = "";
  hashBar.style.width = "0%";
  setHashStatus(t("working"));
  try {
    const res = await invoke<CompareResult>("compare_dirs", {
      dirA: cmpA,
      dirB: cmpB,
      algos,
      recursive,
    });
    cmpData = res;
    cmpAlgosUsed = algos;
    renderCompareResults(res);
    setHashStatus(
      `${res.same} ${t("stSame")} · ${res.different} ${t("stDiff")}`,
      res.different || res.error ? "err" : "ok"
    );
  } catch (e) {
    setHashStatus(String(e), "err");
  } finally {
    cmpBusy = false;
    $<HTMLButtonElement>("btn-cmp-run").disabled = false;
    hashBar.style.width = "100%";
  }
}

$<HTMLButtonElement>("btn-cmp-run").addEventListener("click", runCompare);

function buildCompareReport(format: string): string {
  const res = cmpData;
  if (!res) return "";
  const cols = orderedKeys(cmpAlgosUsed);
  const warn = res.different ? tFmt("warnDiff", { n: res.different }) : "";

  if (format === "csv") {
    const lines = [[t("lblPath"), t("lblStatus"), t("colWarning"), "A", "B"].map(csvCell).join(",")];
    for (const e of res.entries) {
      const note = e.status === "different" ? t("warnHashDiffRow")
        : e.status === "only_a" ? t("stOnlyA")
        : e.status === "only_b" ? t("stOnlyB")
        : e.status === "error" ? t("stError")
        : "";
      lines.push([e.rel_path, statusLabel(e.status), note, sideHashes(e.a, cols), sideHashes(e.b, cols)].map(csvCell).join(","));
    }
    return lines.join("\r\n") + "\r\n";
  }

  if (format === "txt") {
    const lines = [
      t("cmpReportTitle"),
      nowStamp(),
      `${t("lblPath")} A: ${res.dir_a}`,
      `${t("lblPath")} B: ${res.dir_b}`,
      "",
    ];
    if (warn) lines.push(warn, "");
    lines.push([t("lblPath"), t("lblStatus"), "A", "B"].join("\t"));
    for (const e of res.entries) {
      lines.push([e.rel_path, statusLabel(e.status), sideHashes(e.a, cols), sideHashes(e.b, cols)].join("\t"));
    }
    return lines.join("\r\n") + "\r\n";
  }

  const lines = [
    `# ${t("cmpReportTitle")}`,
    "",
    nowStamp(),
    "",
    `- A: \`${res.dir_a}\``,
    `- B: \`${res.dir_b}\``,
    "",
  ];
  if (warn) lines.push(`> ${warn}`, "");
  lines.push(`| ${t("lblPath")} | ${t("lblStatus")} | A | B |`, "|---|---|---|---|");
  for (const e of res.entries) {
    const cells = [e.rel_path, statusLabel(e.status), sideHashes(e.a, cols), sideHashes(e.b, cols)]
      .map((v) => String(v).replace(/\|/g, "\\|"));
    lines.push(`| ${cells.join(" | ")} |`);
  }
  return lines.join("\n") + "\n";
}

$<HTMLButtonElement>("btn-cmp-save").addEventListener("click", async () => {
  if (!cmpData) {
    setHashStatus(t("hashNeedRun"), "err");
    return;
  }
  const fmt = $<HTMLSelectElement>("sel-hash-format").value;
  const path = await save({
    defaultPath: `karsilastirma-raporu.${fmt}`,
    filters: [{ name: fmt.toUpperCase(), extensions: [fmt] }],
  });
  if (typeof path !== "string") return;
  try {
    await invoke("save_text", { path, text: buildCompareReport(fmt) });
    setHashStatus(t("msgSaved") + path, "ok");
  } catch (e) {
    setHashStatus(String(e), "err");
  }
});

function clearHashView() {
  hashPaths = [];
  hashRows = [];
  cmpData = null;
  cmpA = "";
  cmpB = "";
  cmpAEl.textContent = "…";
  cmpBEl.textContent = "…";
  hashResultsEl.innerHTML = "";
  cmpResultsEl.innerHTML = "";
  hashBar.style.width = "0%";
  setHashStatus("");
  renderHashFiles();
}

renderHashFiles();

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
      if (dto.combinedPaths?.length || dto.combinedPath) {
        renderCombined(dto.combinedPaths, dto.combinedPath, webOutputs);
      }
    } else if (webKind === "sheet") {
      const r = await invoke<SheetResult>("import_sheet_url", {
        url, outDir: webOutDir, format: fmt,
      });
      setWebStatus(tFmt("webTable", { name: `${r.name}.${r.format}`, rows: r.rows, cols: r.cols, path: r.path }), "ok");
      showWebOutput(r.path);
    } else {
      setWebStatus(t("msgEnterLink"), "err");
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

function edPushUndo(keepBorder = false) {
  try {
    const c = edCanvas();
    edUndo.push(edCtx().getImageData(0, 0, c.width, c.height));
    if (edUndo.length > 20) edUndo.shift();
    // Baska islem cerceveyi icine gomerse takip biter (kutu isareti kalkar).
    if (!keepBorder) edBorderReset();
  } catch { /* yoksay */ }
}

function edSetTool(tool: "crop" | "pen" | "box" | null) {
  edTool = tool;
  document.querySelectorAll<HTMLButtonElement>("#editor-bar [data-ed]").forEach((b) => {
    b.classList.toggle("on", b.dataset.ed === tool);
  });
  // Kalem seckiliyken imlec kalem seklini alir.
  edCanvas().style.cursor = tool === "pen"
    ? "url(\"data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' width='24' height='24'><path d='M4 20l3-9L17 1l3 3L10 14l-6 6z' fill='%23ff0000' stroke='white' stroke-width='1'/></svg>\") 4 20, crosshair"
    : tool ? "crosshair" : "default";
}

function edRefreshSwatch() {
  const col = ($<HTMLInputElement>("ed-color")).value || "#ff0000";
  const w = Number(($<HTMLInputElement>("ed-width")).value) || 3;
  const sw = $("ed-swatch") as HTMLElement;
  sw.style.background = col;
  sw.style.height = Math.max(3, Math.min(14, w)) + "px";
  ($("ed-wlabel") as HTMLElement).textContent = `${w}px`;
}

function openEditor(idx: number) {
  const L = srcLayers[idx];
  if (!L) return;
  edIdx = idx;
  edUndo = [];
  edFilterBase = null;
  invoke("set_editor_open", { open: true }).catch(() => {});
  // Varsayilan arac yok (kullanici secer).
  edSetTool(null);
  edRefreshSwatch();
  ($<HTMLInputElement>("ed-bright")).value = "100";
  ($<HTMLInputElement>("ed-contrast")).value = "100";
  loadImg(L.b64).then((img) => {
    const c = edCanvas();
    c.width = img.naturalWidth;
    c.height = img.naturalHeight;
    edCtx().drawImage(img, 0, 0);
    ($("editor-stage") as HTMLElement).style.background =
      ($<HTMLInputElement>("ed-bg")).value || "#ffffff";
    ($("editor") as HTMLElement).hidden = false;
  }).catch((e) => setStatus(String(e), "err"));
}

function closeEditor() {
  ($("editor") as HTMLElement).hidden = true;
  edIdx = -1;
  edTool = null;
  edUndo = [];
  edFilterBase = null;
  invoke("set_editor_open", { open: false }).catch(() => {});
  // Ana formu one getir.
  try {
    const w = getCurrentWindow();
    w.show().then(() => w.unminimize().then(() => w.setFocus()).catch(() => {})).catch(() => {});
  } catch { /* yoksay */ }
}

listen<string>("editor-blocked", (ev) => setStatus(ev.payload, "err"));

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

// Cerceve checkbox mantigi: seciliyse aktif, birakilirsa son islem geri alinir.
// Renk/kalinlik degisirse mevcut cerceve degistirilir (yigin sismez).
let edBorderOn = false;

function edDrawBorder() {
  const c = edCanvas();
  const ctx = edCtx();
  const col = ($<HTMLInputElement>("ed-color")).value || "#ff0000";
  const w = Number(($<HTMLInputElement>("ed-width")).value) || 4;
  ctx.save();
  ctx.strokeStyle = col;
  ctx.lineWidth = Math.max(1, w);
  const inset = Math.max(1, w) / 2;
  ctx.strokeRect(inset, inset, c.width - inset * 2, c.height - inset * 2);
  ctx.restore();
  edFilterBase = null;
}

function edBorderReplace() {
  // Yasli cerceveyi kaldir (bir adim geri), tazesini ciz.
  const prev = edUndo.pop();
  if (prev) {
    const c = edCanvas();
    if (prev.width !== c.width || prev.height !== c.height) {
      c.width = prev.width;
      c.height = prev.height;
    }
    edCtx().putImageData(prev, 0, 0);
  }
  edPushUndo(true);
  edDrawBorder();
}

// Diger islemler cerceveyi icine gomerse takip biter (kutu isareti kalkar).
function edBorderReset() {
  edBorderOn = false;
  const box = $("ed-border-check") as HTMLInputElement | null;
  if (box) box.checked = false;
}

function edBorder(check: boolean) {
  const box = $<HTMLInputElement>("ed-border-check");
  if (check) {
    edPushUndo(true);
    edDrawBorder();
    edBorderOn = true;
    box.checked = true;
    setStatus(tFmt("edBorderDone", { w: Number(($<HTMLInputElement>("ed-width")).value) || 4 }), "ok");
  } else if (edBorderOn) {
    const prev = edUndo.pop();
    if (prev) {
      const c = edCanvas();
      if (prev.width !== c.width || prev.height !== c.height) {
        c.width = prev.width;
        c.height = prev.height;
      }
      edCtx().putImageData(prev, 0, 0);
    }
    edBorderOn = false;
    box.checked = false;
    edFilterBase = null;
  }
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
$<HTMLButtonElement>("ed-close").addEventListener("click", closeEditor);
($<HTMLInputElement>("ed-color")).addEventListener("input", edRefreshSwatch);
($<HTMLInputElement>("ed-width")).addEventListener("input", edRefreshSwatch);

{
  const rl = document.querySelector<HTMLButtonElement>('#editor-bar [data-ed="rotl"]');
  const rr = document.querySelector<HTMLButtonElement>('#editor-bar [data-ed="rotr"]');
  const fh = document.querySelector<HTMLButtonElement>('#editor-bar [data-ed="fliph"]');
  const bdCheck = $<HTMLInputElement>("ed-border-check");
  rl?.addEventListener("click", () => { edRotate(-1); edBorderReset(); });
  rr?.addEventListener("click", () => { edRotate(1); edBorderReset(); });
  fh?.addEventListener("click", () => { edFlipH(); edBorderReset(); });
  bdCheck?.addEventListener("change", () => edBorder(bdCheck.checked));
  const edCol = $<HTMLInputElement>("ed-color");
  const edW = $<HTMLInputElement>("ed-width");
  const edLiveBorder = () => {
    if (!edBorderOn) return;
    edBorderReplace();
    edRefreshSwatch();
  };
  edCol?.addEventListener("input", () => { edRefreshSwatch(); edLiveBorder(); });
  edW?.addEventListener("input", () => { edRefreshSwatch(); edLiveBorder(); });
  ($<HTMLInputElement>("ed-bright")).addEventListener("input", edApplyFilter);
  ($<HTMLInputElement>("ed-contrast")).addEventListener("input", edApplyFilter);
  // Zemin rengi sahnede aninda gorunur (saydam alanlar bu renkle gorunur;
  // Uygula'da ayni renkle duzlestirilir).
  ($<HTMLInputElement>("ed-bg")).addEventListener("input", () => {
    ($("editor-stage") as HTMLElement).style.background =
      ($<HTMLInputElement>("ed-bg")).value || "#ffffff";
  });
  document.addEventListener("keydown", (e) => {
    if (!($("editor") as HTMLElement).hidden && e.key === "Escape") closeEditor();
  });
  // Ctrl+Z: duzenleyicide geri al.
  document.addEventListener("keydown", (e) => {
    if (($("editor") as HTMLElement).hidden) return;
    const tag = (e.target as HTMLElement)?.tagName ?? "";
    if (tag === "TEXTAREA" || tag === "INPUT" || tag === "SELECT") return;
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "z" && !e.shiftKey) {
      e.preventDefault();
      ($<HTMLButtonElement>("ed-undo")).click();
    }
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
      name: t("filterVideo"),
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
        div.textContent = "✗ " + it.name + ": " + (it.error ?? t("msgError"));
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

listen<string>("video-log", (ev) => {
  if (recExtracting) rlog(ev.payload);
  else vlog(ev.payload);
});
listen<Record<string, unknown>>("video-progress", (ev) => {
  const p = Number(ev.payload["percent"] ?? 0);
  const w = `${Math.max(0, Math.min(100, p))}%`;
  if (recExtracting) liveBar.style.width = w;
  else videoBar.style.width = w;
});

// ---------------------------------------------------------------------------
// Canli OCR sekmesi (ekran kaydi → video hatti)
// ---------------------------------------------------------------------------

interface RecWindowDto { id: string; title: string; full: string; w: number; h: number; }
interface RecStopDto { path: string; elapsedSec: number; keptVideo: boolean; mask?: string | null; }

const liveStatus = $("live-status");
const liveTimerEl = $("live-timer");
const liveRegionEl = $("live-region");
const liveLog = $("live-log");
const liveBar = $("live-progress-bar");
const liveOutputs = $("live-outputs");
const selLiveSource = $<HTMLSelectElement>("sel-live-source");
const selLiveWindow = $<HTMLSelectElement>("sel-live-window");
const selLiveMode = $<HTMLSelectElement>("sel-live-mode");
const selLiveFormat = $<HTMLSelectElement>("sel-live-format");
const selLiveQuality = $<HTMLSelectElement>("sel-live-quality");
const selLiveAudio = $<HTMLSelectElement>("sel-live-audio");
let liveOutDir = "";
let liveRecording = false;
let recExtracting = false;
let liveWindowList: RecWindowDto[] = [];
let lastLiveRect: [number, number, number, number] | null = null;
let liveTimer: number | undefined;
let liveStartTs = 0;

function fmtElapsed(s: number): string {
  const m = Math.floor(s / 60);
  return `${String(m).padStart(2, "0")}:${String(Math.floor(s % 60)).padStart(2, "0")}`;
}

function setLiveStatus(msg: string, cls: "" | "ok" | "err" = "") {
  liveStatus.textContent = msg;
  liveStatus.className = "status " + cls;
}

function rlog(msg: string) {
  liveLog.textContent += msg + "\n";
  liveLog.scrollTop = liveLog.scrollHeight;
}

function liveTick() {
  liveTimerEl.textContent = "⏱ " + fmtElapsed((Date.now() - liveStartTs) / 1000);
}

function renderLiveRegion(r: [number, number, number, number] | null) {
  lastLiveRect = r;
  liveRegionEl.textContent = r
    ? `${Math.round(r[0])},${Math.round(r[1])} ${Math.round(r[2])}×${Math.round(r[3])} · ${t("liveRegionSet")}`
    : "…";
}

async function refreshLiveWindows() {
  try {
    const wins = await invoke<RecWindowDto[]>("list_open_windows");
    liveWindowList = wins;
    const prev = selLiveWindow.value;
    selLiveWindow.innerHTML = `<option value="">${t("optNoWindow")}</option>`;
    for (const w of wins) {
      const o = document.createElement("option");
      o.value = w.id;
      o.textContent = `${w.title} (${w.w}×${w.h})`;
      selLiveWindow.appendChild(o);
    }
    if (wins.some((w) => w.id === prev)) selLiveWindow.value = prev;
    if (!wins.length) setLiveStatus(t("liveNoWindows"), "err");
  } catch (e) {
    setLiveStatus(String(e), "err");
  }
}

async function refreshLiveAudio() {
  try {
    const devs = await invoke<string[]>("list_audio_inputs");
    const prev = selLiveAudio.value;
    selLiveAudio.innerHTML = `<option value="">${t("optNoAudio")}</option>`;
    for (const d of devs) {
      const o = document.createElement("option");
      o.value = d;
      o.textContent = d;
      selLiveAudio.appendChild(o);
    }
    if (devs.includes(prev)) selLiveAudio.value = prev;
  } catch (e) {
    setLiveStatus(String(e), "err");
  }
}

function syncLiveSource() {
  const k = selLiveSource.value;
  ($("btn-live-pick") as HTMLButtonElement).disabled = k !== "region" || liveRecording;
  selLiveWindow.disabled = k !== "window" || liveRecording;
  if (k === "window" && !selLiveWindow.options.length) refreshLiveWindows();
}
selLiveSource.addEventListener("change", syncLiveSource);

$<HTMLButtonElement>("btn-live-pick").addEventListener("click", async () => {
  try {
    await invoke("rec_arm", { armed: true });
    await invoke("begin_capture");
  } catch (e) {
    await invoke("rec_arm", { armed: false }).catch(() => {});
    setLiveStatus(String(e), "err");
  }
});

listen<[number, number, number, number]>("rec-region", (ev) => renderLiveRegion(ev.payload));

$<HTMLButtonElement>("btn-live-windows").addEventListener("click", refreshLiveWindows);

$<HTMLButtonElement>("btn-live-out").addEventListener("click", async () => {
  const sel = await open({ directory: true, multiple: false });
  if (typeof sel === "string") {
    liveOutDir = sel;
    ($("live-outdir") as HTMLElement).textContent = sel;
  }
});

$<HTMLButtonElement>("btn-live-start").addEventListener("click", async () => {
  if (liveRecording) return;
  const kind = selLiveSource.value;
  if (kind === "region") {
    const r = await invoke<[number, number, number, number] | null>("rec_get_region").catch(() => null);
    renderLiveRegion(r);
  }
  try {
    const audio = selLiveAudio.value || null;
    const ident = kind === "window" ? selLiveWindow.value || null : null;
    const identTitle = kind === "window"
      ? (liveWindowList.find((w) => w.id === selLiveWindow.value)?.full ?? null)
      : null;
    await invoke("rec_start", { kind, ident, identTitle, audio });
    liveRecording = true;
    liveStartTs = Date.now();
    liveTimerEl.textContent = "⏱ 00:00";
    liveTimer = window.setInterval(liveTick, 1000);
    setLiveStatus(t("liveRecording"));
    liveOutputs.innerHTML = "";
    liveBar.style.width = "0%";
    ($("btn-live-start") as HTMLButtonElement).disabled = true;
    ($("btn-live-stop") as HTMLButtonElement).disabled = false;
    syncLiveSource();
  } catch (e) {
    setLiveStatus(String(e), "err");
  }
});

$<HTMLButtonElement>("btn-live-stop").addEventListener("click", () => void liveStop());
listen("rec-stop-request", () => { if (liveRecording) void liveStop(); });

async function liveStop() {
  if (!liveRecording) return;
  if (!liveOutDir) {
    const sel = await open({ directory: true, multiple: false });
    if (typeof sel !== "string") return;
    liveOutDir = sel;
    ($("live-outdir") as HTMLElement).textContent = sel;
  }
  try {
    const keep = ($("chk-live-keep") as HTMLInputElement).checked;
    const dto = await invoke<RecStopDto>("rec_stop", { outDir: liveOutDir, keepVideo: keep });
    liveRecording = false;
    if (liveTimer !== undefined) { clearInterval(liveTimer); liveTimer = undefined; }
    liveTimerEl.textContent = "⏱ " + fmtElapsed(dto.elapsedSec);
    ($("btn-live-start") as HTMLButtonElement).disabled = false;
    ($("btn-live-stop") as HTMLButtonElement).disabled = true;
    syncLiveSource();
    // Ayni video hatti: kare OCR + dedup + CSV/TXT/MD/XLSX.
    setLiveStatus(t("liveExtracting"));
    recExtracting = true;
    try {
      const vdto = await invoke<VideoBatchDto>("video_extract_batch", {
        files: [dto.path],
        outDir: liveOutDir,
        mode: selLiveMode.value,
        langs: selLang.value,
        maxFrames: Number(selLiveQuality.value),
        formats: selLiveFormat.value,
        mask: dto.mask ?? null,
      });
      liveBar.style.width = "100%";
      setLiveStatus(`${vdto.okCount}/${vdto.items.length} → ${vdto.outDir} · ${t("liveDone")}`, vdto.failCount ? "err" : "ok");
      for (const it of vdto.items) {
        const div = document.createElement("div");
        if (it.ok) {
          div.className = "fout";
          div.textContent = "✓ " + it.name + " → " + it.outputFiles.join(" · ");
        } else {
          div.textContent = "✗ " + it.name + ": " + (it.error ?? t("msgError"));
        }
        liveOutputs.appendChild(div);
      }
      if (dto.keptVideo) {
        const div = document.createElement("div");
        div.className = "fout";
        div.textContent = "🎬 " + dto.path;
        liveOutputs.appendChild(div);
      } else {
        await invoke("rec_cleanup", { path: dto.path }).catch(() => {});
      }
    } finally {
      recExtracting = false;
    }
  } catch (e) {
    setLiveStatus(String(e), "err");
  }
}

// Sekme acilisinda listeleri doldur + yarim kalan kayit varsa goster.
refreshLiveWindows();
refreshLiveAudio();
syncLiveSource();
// Canli sekme acikken pencere listesini 10 sn'de bir sessizce tazele.
window.setInterval(() => {
  if (!liveRecording && currentView() === "canli") refreshLiveWindows();
}, 10000);
($("btn-live-stop") as HTMLButtonElement).disabled = true;
invoke<{ recording: boolean; elapsedSec: number }>("rec_status").then((s) => {
  if (s.recording) {
    liveRecording = true;
    liveStartTs = Date.now() - s.elapsedSec * 1000;
    liveTimer = window.setInterval(liveTick, 1000);
    setLiveStatus(t("liveRecording"));
    ($("btn-live-start") as HTMLButtonElement).disabled = true;
    ($("btn-live-stop") as HTMLButtonElement).disabled = false;
  }
}).catch(() => {});

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
  clearHashView();
});

const IMG_EXTS = ["png", "jpg", "jpeg", "bmp", "tif", "tiff", "webp", "gif"];
const DOC_EXTS = [...IMG_EXTS, "pdf", "docx", "xlsx", "pptx", "udf", "odt", "ods", "odp", "epub",
  "rtf", "txt", "md", "markdown", "csv", "tsv", "json", "jsonl", "ndjson", "log",
  "xml", "html", "htm", "sql", "srt", "vtt", "ini", "cfg", "yaml", "yml", "toml",
  "ps1", "bat", "cmd", "sh"];
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
    refreshDocHash();
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
  } else if (view === "hash") {
    const recursive = $<HTMLInputElement>("chk-hash-recursive").checked;
    try {
      const found = await invoke<string[]>("expand_all_dirs", { dirs: paths, recursive });
      const add = new Set<string>(found);
      for (const p of paths) {
        if (extOf(p)) add.add(p);
      }
      let n = 0;
      for (const p of add) {
        if (!hashPaths.includes(p)) {
          hashPaths.push(p);
          n++;
        }
      }
      renderHashFiles();
      if (!n) dropNotSupported(view, paths.length);
    } catch {
      dropNotSupported(view, paths.length);
    }
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
