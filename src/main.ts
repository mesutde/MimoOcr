import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open, save } from "@tauri-apps/plugin-dialog";

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

const $ = <T extends HTMLElement>(id: string) => document.getElementById(id) as T;

const btnCapture = $<HTMLButtonElement>("btn-capture");
const btnOpen = $<HTMLButtonElement>("btn-open");
const btnPaste = $<HTMLButtonElement>("btn-paste");
const btnCopy = $<HTMLButtonElement>("btn-copy");
const btnSave = $<HTMLButtonElement>("btn-save");
const selLang = $<HTMLSelectElement>("sel-lang");
const selPsm = $<HTMLSelectElement>("sel-psm");
const selScale = $<HTMLSelectElement>("sel-scale");
const txtResult = $<HTMLTextAreaElement>("txt-result");
const imgWrap = $("img-wrap");
const status = $("status");

let busy = false;

function setStatus(msg: string, cls: "" | "ok" | "err" = "") {
  status.textContent = msg;
  status.className = "status " + cls;
}

function setBusy(b: boolean) {
  busy = b;
  [btnCapture, btnOpen, btnPaste].forEach((x) => (x.disabled = b));
  if (b) setStatus("OCR çalışıyor…");
}

function showResult(doc: OcrDocument) {
  txtResult.value = doc.plain_text;
  const img = document.createElement("img");
  img.src = "data:image/png;base64," + doc.image_png_base64;
  imgWrap.innerHTML = "";
  imgWrap.appendChild(img);
  const avg =
    doc.words.length > 0
      ? doc.words.reduce((a, w) => a + w.confidence, 0) / doc.words.length
      : 0;
  const conf = doc.words.length > 0 ? ` · güven %${avg.toFixed(0)}` : "";
  setStatus(
    `${doc.engine} · ${doc.language} · ${doc.elapsed_ms} ms · ${doc.words.length} kelime${conf}`,
    "ok",
  );
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

btnOpen.addEventListener("click", async () => {
  const path = await open({
    filters: [{ name: "Görsel", extensions: ["png", "jpg", "jpeg", "bmp", "webp", "gif"] }],
  });
  if (typeof path === "string") await runOcr({ source: { kind: "file", path } });
});

btnPaste.addEventListener("click", () => runOcr({ source: { kind: "clipboard" } }));

btnCopy.addEventListener("click", async () => {
  if (!txtResult.value) return;
  await invoke("copy_text", { text: txtResult.value });
  setStatus("Panoya kopyalandı.", "ok");
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
    setStatus("Kaydedildi: " + path, "ok");
  }
});

// ---------------------------------------------------------------------------
// Video → CSV/TXT sekmesi
// ---------------------------------------------------------------------------

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

const tabOcr = $<HTMLButtonElement>("tab-ocr");
const tabVideo = $<HTMLButtonElement>("tab-video");
const viewOcr = $("view-ocr");
const viewVideo = $("view-video");
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

function switchTab(video: boolean) {
  tabOcr.classList.toggle("active", !video);
  tabVideo.classList.toggle("active", video);
  viewOcr.style.display = video ? "none" : "";
  (viewVideo as HTMLElement).hidden = !video;
}

tabOcr.addEventListener("click", () => switchTab(false));
tabVideo.addEventListener("click", () => switchTab(true));

function renderVideoFiles() {
  videoFiles.innerHTML = videoPaths.length
    ? ""
    : '<span class="muted">Henüz video seçilmedi.</span>';
  for (const p of videoPaths) {
    const div = document.createElement("div");
    div.textContent = "🎬 " + p;
    videoFiles.appendChild(div);
  }
  const out = document.createElement("div");
  out.textContent = "📁 Çıktı: " + (videoOutDir || "(seçilmedi — çalıştırırken sorulur)");
  videoFiles.appendChild(out);
}

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
    videoReq.textContent = missing.length
      ? "⚠ " + missing.join(" · ")
      : "✓ python · ffmpeg · tesseract hazır";
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
  videoStatus.textContent = "Video işleniyor…";
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
    videoStatus.textContent =
      `${dto.okCount}/${dto.items.length} başarılı → ${dto.outDir}`;
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
listen<Record<string, unknown>>("batch-progress", (ev) =>
  vlog(`--- ${ev.payload["index"]}/${ev.payload["total"]} ${ev.payload["name"]} ---`),
);

renderVideoFiles();
refreshVideoReq();

// Bölge yakalama tamamlandığında overlay penceresinden gelen sonuç
listen<OcrDocument>("ocr-result", (ev) => showResult(ev.payload));
listen<string>("ocr-error", (ev) => setStatus(ev.payload, "err"));

// Ayarlar değişince backend'i bilgilendir
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
pushOptions();
