import { invoke } from "@tauri-apps/api/core";
import { emitTo } from "@tauri-apps/api/event";

const timeEl = document.getElementById("time") as HTMLElement;

// Mini sozluk (ana sayfadaki dille eslenir).
function recLang(): "tr" | "en" {
  try {
    return localStorage.getItem("mimo-ui-lang") === "en" ? "en" : "tr";
  } catch {
    return "tr";
  }
}

try {
  const en = recLang() === "en";
  document.title = en ? "Recording" : "Kayıt";
  const stop = document.getElementById("stop");
  if (stop) stop.textContent = en ? "Stop" : "Durdur";
} catch { /* yoksay */ }

function fmt(s: number): string {
  const m = Math.floor(s / 60);
  return `${String(m).padStart(2, "0")}:${String(Math.floor(s % 60)).padStart(2, "0")}`;
}

async function tick() {
  try {
    const s = await invoke<{ recording: boolean; elapsedSec: number }>("rec_status");
    timeEl.textContent = fmt(s.elapsedSec);
  } catch {
    /* kayit bitmis olabilir */
  }
}

document.getElementById("stop")?.addEventListener("click", async () => {
  await emitTo("main", "rec-stop-request", null).catch(() => {});
});

void tick();
window.setInterval(tick, 1000);
