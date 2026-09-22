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
