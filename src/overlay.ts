import { invoke } from "@tauri-apps/api/core";
import { emitTo } from "@tauri-apps/api/event";

interface OcrDocument {
  plain_text: string;
  [k: string]: unknown;
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

function norm() {
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
  const { x, y, w, h } = norm();
  rect.style.display = "none";
  sizeLbl.style.display = "none";
  if (w < 8 || h < 8) return; // çok küçük seçim: iptal say
  busy = true;
  hint.innerHTML = "OCR çalışıyor…";
  try {
    const doc = await invoke<OcrDocument>("complete_capture", { x, y, width: w, height: h });
    await emitTo("main", "ocr-result", doc);
  } catch (err) {
    await emitTo("main", "ocr-error", String(err));
  } finally {
    busy = false;
    hint.innerHTML = "Alan seçmek için sürükleyin · <b>Esc</b> iptal";
  }
});

window.addEventListener("keydown", async (e) => {
  if (e.key === "Escape") await invoke("cancel_capture");
});
