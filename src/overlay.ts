import { invoke } from "@tauri-apps/api/core";
import { emitTo } from "@tauri-apps/api/event";

interface OcrDocument {
  plain_text: string;
  [k: string]: unknown;
}

interface Rect {
  x: number;
  y: number;
  w: number;
  h: number;
}

const rect = document.getElementById("rect") as HTMLDivElement;
const sizeLbl = document.getElementById("size") as HTMLDivElement;
const hint = document.getElementById("hint") as HTMLDivElement;

let startX = 0;
let startY = 0;
let curX = 0;
let curY = 0;
let dragging = false;
let busy = false;
// Ctrl ile biriktirilen bölgeler (Önizleme seç / çoklu alan)
let queued: Rect[] = [];
const boxes: HTMLDivElement[] = [];

// Ana penceredeki dil secimiyle eslenir (localStorage).
type OvLang = "tr" | "en";
function ovLang(): OvLang {
  try {
    return localStorage.getItem("mimo-ui-lang") === "en" ? "en" : "tr";
  } catch {
    return "tr";
  }
}

const OV: Record<OvLang, Record<string, string>> = {
  tr: {
    hint: "Sürükle-bırak: tek alan · <b>Ctrl+sürükle</b>: çoklu alan · <b>Enter</b>: bitir · <b>Esc</b>: iptal",
    multi: "{n} alan seçildi · <b>Enter</b>: OCR · <b>Ctrl+sürükle</b>: ekle · <b>Esc</b>: iptal",
    max: "En fazla 12 alan · <b>Enter</b>: OCR · <b>Esc</b>: iptal",
    working: "OCR çalışıyor…",
    workingMulti: "OCR çalışıyor… ({n} alan)",
  },
  en: {
    hint: "Drag-drop: single area · <b>Ctrl+drag</b>: multi-area · <b>Enter</b>: finish · <b>Esc</b>: cancel",
    multi: "{n} areas selected · <b>Enter</b>: OCR · <b>Ctrl+drag</b>: add · <b>Esc</b>: cancel",
    max: "Max 12 areas · <b>Enter</b>: OCR · <b>Esc</b>: cancel",
    working: "Working…",
    workingMulti: "Working… ({n} areas)",
  },
};

function ovT(key: string, params: Record<string, string | number> = {}): string {
  let s: string = OV[ovLang()][key] ?? key;
  for (const [k, v] of Object.entries(params)) s = s.replace(`{${k}}`, String(v));
  return s;
}

function baseHint(): string {
  return ovT("hint");
}

function norm(): Rect {
  const x = Math.min(startX, curX);
  const y = Math.min(startY, curY);
  const w = Math.abs(curX - startX);
  const h = Math.abs(curY - startY);
  return { x, y, w, h };
}

function paint() {
  const { x, y, w, h } = norm();
  rect.style.display = "block";
  rect.style.left = x + "px";
  rect.style.top = y + "px";
  rect.style.width = w + "px";
  rect.style.height = h + "px";
  sizeLbl.style.display = "block";
  sizeLbl.textContent = `${w} × ${h}`;
  const lx = Math.min(x + w + 10, window.innerWidth - 90);
  const ly = y + h + 34 > window.innerHeight ? y - 32 : y + h + 10;
  sizeLbl.style.left = lx + "px";
  sizeLbl.style.top = ly + "px";
}

function paintQueued() {
  // Eski kutuları temizle, birikenleri çiz
  for (const b of boxes) b.remove();
  boxes.length = 0;
  queued.forEach((r, i) => {
    const d = document.createElement("div");
    d.style.cssText =
      `position:fixed;left:${r.x}px;top:${r.y}px;width:${r.w}px;height:${r.h}px;` +
      `border:2px dashed #4ADE80;background:rgba(74,222,128,.1);` +
      `color:#4ADE80;font-size:11px;padding:2px 6px;pointer-events:none;`;
    d.textContent = `${i + 1}`;
    document.body.appendChild(d);
    boxes.push(d);
  });
  hint.innerHTML =
    queued.length > 0
      ? ovT("multi", { n: queued.length })
      : baseHint();
}

function resetOverlay() {
  queued = [];
  for (const b of boxes) b.remove();
  boxes.length = 0;
  rect.style.display = "none";
  sizeLbl.style.display = "none";
  hint.innerHTML = baseHint();
}

window.addEventListener("mousedown", (e) => {
  if (busy || e.button !== 0) return;
  dragging = true;
  startX = curX = e.clientX;
  startY = curY = e.clientY;
  paint();
});

window.addEventListener("mousemove", (e) => {
  if (!dragging) return;
  curX = e.clientX;
  curY = e.clientY;
  paint();
});

window.addEventListener("mouseup", async (e) => {
  if (!dragging || busy || e.button !== 0) return;
  dragging = false;
  const r = norm();
  rect.style.display = "none";
  sizeLbl.style.display = "none";
  if (r.w < 8 || r.h < 8) return; // çok küçük seçim: iptal say
  // Ctrl basılıysa biriktir, overlay açık kalır
  if (e.ctrlKey || e.metaKey) {
    if (queued.length >= 12) {
      hint.innerHTML = ovT("max");
      return;
    }
    queued.push(r);
    paintQueued();
    return;
  }
  busy = true;
  hint.innerHTML = ovT("working");
  try {
    const doc = await invoke<OcrDocument>("complete_capture", {
      x: r.x,
      y: r.y,
      width: r.w,
      height: r.h,
    });
    await emitTo("main", "ocr-result", doc);
  } catch (err) {
    await emitTo("main", "ocr-error", String(err));
  } finally {
    busy = false;
    resetOverlay();
  }
});

async function finishQueued() {
  if (busy || queued.length === 0) return;
  busy = true;
  hint.innerHTML = ovT("workingMulti", { n: queued.length });
  try {
    const docs = await invoke<OcrDocument[]>("ocr_preview_regions", {
      regions: queued.map((r) => [r.x, r.y, r.w, r.h]),
    });
    await emitTo("main", "ocr-results", docs);
  } catch (err) {
    await emitTo("main", "ocr-error", String(err));
  } finally {
    busy = false;
    resetOverlay();
  }
}

window.addEventListener("keydown", async (e) => {
  if (e.key === "Escape") {
    resetOverlay();
    await invoke("cancel_capture");
  } else if (e.key === "Enter") {
    await finishQueued();
  }
});

hint.innerHTML = baseHint();
