//! Arayuz dili (TR/EN) icin backend mesaj tablosu.
//!
//! On yuz `set_ui_lang` ile dili bildirir; komutlar kullaniciya donen
//! hata/durum metinlerini bu tablodan secer. Nadir ic hatalar ham gecer
//! (planda belirtildigi uzere); `OcrError` tasiyici olarak aynen kalir.

/// "tr" disindaki her sey Ingilizce sayilir.
pub fn norm(lang: &str) -> bool {
    lang.trim().to_ascii_lowercase().starts_with("tr")
}

/// TR/EN mesaj. `tr` tam metin, `en` karsiligi.
pub fn msg(lang: &str, key: &str) -> String {
    let en = !norm(lang);
    match key {
        // Motor
        "engine_missing" => if en {
            "Tesseract not found. Install Tesseract 5 or pick its path.".into()
        } else {
            "Tesseract bulunamadı. Tesseract 5 kurun ya da exe yolunu seçin.".into()
        },
        "engine_unavailable" => if en {
            "Engine unavailable".into()
        } else {
            "Motor kullanılamıyor".into()
        },
        "regions_empty" => if en {
            "No regions given.".into()
        } else {
            "Bölge verilmedi.".into()
        },
        "engine_not_found_file" => if en {
            "Selected file not found.".into()
        } else {
            "Seçilen dosya bulunamadı.".into()
        },
        "engine_not_runnable" => if en {
            "This file cannot run as Tesseract.".into()
        } else {
            "Bu dosya Tesseract olarak çalıştırılamadı.".into()
        },
        "tessdata_missing" => if en {
            "tessdata directory could not be resolved.".into()
        } else {
            "tessdata dizini çözümlenemedi.".into()
        },
        "winrt_no_pack" => if en {
            "No Windows OCR language pack (install it in Settings → Language)."
                .into()
        } else {
            "Windows OCR dil paketi yok (Ayarlar → Dil paketlerini kurun).".into()
        },
        "win_only" => if en {
            "Windows OCR is Windows-only.".into()
        } else {
            "Windows OCR yalnız Windows'ta.".into()
        },
        // Yakalama
        "no_region" => if en {
            "No captured region yet.".into()
        } else {
            "Henüz yakalanmış bölge yok.".into()
        },
        "max_regions" => if en {
            "At most 12 regions.".into()
        } else {
            "En fazla 12 bölge.".into()
        },
        "capture_outside" => if en {
            "Selection is outside the screen.".into()
        } else {
            "Seçim ekran sınırları dışında.".into()
        },
        "no_monitors" => if en {
            "No monitor found.".into()
        } else {
            "Monitör bulunamadı.".into()
        },
        "capture_fail" => if en {
            "Screen could not be captured.".into()
        } else {
            "Ekran yakalanamadı.".into()
        },
        "clipboard_no_image" => if en {
            "No image on clipboard.".into()
        } else {
            "Panoda görsel yok.".into()
        },
        // Dosya / toplu / video
        "no_files" => if en {
            "No files selected.".into()
        } else {
            "Dosya seçilmedi.".into()
        },
        "bad_batch_format" => if en {
            "Format must be txt, md or pdf.".into()
        } else {
            "Format txt, md veya pdf olmalı.".into()
        },
        "out_dir_fail" => if en {
            "Output folder could not be opened.".into()
        } else {
            "Çıktı klasörü açılamadı.".into()
        },
        "file_missing" => if en {
            "File not found.".into()
        } else {
            "Dosya bulunamadı.".into()
        },
        "ext_unsupported" => if en {
            "Unsupported type.".into()
        } else {
            "Desteklenmeyen tür.".into()
        },
        "doc_bad_ext" => if en {
            "Unsupported file type: use an image, PDF/DOCX/XLSX/PPTX/UDF or text.".into()
        } else {
            "Desteklenmeyen dosya türü: görsel, PDF/DOCX/XLSX/PPTX/UDF veya metin olmalı.".into()
        },
        "script_missing" => if en {
            "Helper script not found.".into()
        } else {
            "Yardımcı betik bulunamadı.".into()
        },
        "spawn_fail" => if en {
            "Helper could not be started.".into()
        } else {
            "Yardımcı başlatılamadı.".into()
        },
        "task_cut" => if en {
            "Task was interrupted.".into()
        } else {
            "Görev yarıda kesildi.".into()
        },
        "write_fail" => if en {
            "Could not write.".into()
        } else {
            "Yazılamadı.".into()
        },
        "combined_fail" => if en {
            "Combined file could not be written.".into()
        } else {
            "Birleşik dosya yazılamadı.".into()
        },
        "image_decode" => if en {
            "Image could not be decoded.".into()
        } else {
            "Görsel çözülemedi.".into()
        },
        // Video
        "video_no_files" => if en {
            "No video selected.".into()
        } else {
            "Video seçilmedi.".into()
        },
        "video_bad_mode" => if en {
            "Invalid scroll mode (auto/fast/slow).".into()
        } else {
            "Geçersiz scroll modu (auto/fast/slow).".into()
        },
        "video_no_script" => if en {
            "Video script not found.".into()
        } else {
            "Video betiği bulunamadı.".into()
        },
        "video_spawn" => if en {
            "Python could not be started.".into()
        } else {
            "Python başlatılamadı.".into()
        },
        "video_wait" => if en {
            "Video job could not be awaited.".into()
        } else {
            "Video işi beklenemedi.".into()
        },
        "video_ext_unsupported" => if en {
            "Unsupported extension.".into()
        } else {
            "Desteklenmeyen uzantı.".into()
        },
        "video_task_cut" => if en {
            "Video task was interrupted.".into()
        } else {
            "Video görevi yarıda kesildi.".into()
        },
        "video_exit_code" => if en {
            "Exit code".into()
        } else {
            "Çıkış kodu".into()
        },
        "video_done_ok" => if en {
            "succeeded".into()
        } else {
            "başarılı".into()
        },
        "monitors_fail" => if en {
            "Monitors could not be listed.".into()
        } else {
            "Monitörler alınamadı.".into()
        },
        "pdf_fail" => if en {
            "PDF could not be produced (reportlab needed)".into()
        } else {
            "PDF üretilemedi (reportlab gerekli)".into()
        },
        "merge_fail" => if en {
            "PDFs could not be merged (pypdf needed)".into()
        } else {
            "PDF birleştirilemedi (pypdf gerekli)".into()
        },
        "udf_pdf_fail" => if en {
            "UDF→PDF could not be produced (reportlab needed)".into()
        } else {
            "UDF→PDF üretilemedi (reportlab gerekli)".into()
        },
        "doc_exit_code" => if en {
            "Document extraction failed (code".into()
        } else {
            "Belge çıkarılamadı (kod".into()
        },
        // Model yoneticisi
        "model_unknown" => if en {
            "Unknown model.".into()
        } else {
            "Bilinmeyen model.".into()
        },
        "model_protected" => if en {
            "Base models (tur/eng/osd) cannot be removed.".into()
        } else {
            "Temel modeller (tur/eng/osd) silinemez.".into()
        },
        _ => key.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iki_dilde_doner() {
        assert!(msg("tr", "engine_missing").contains("bulunamadı"));
        assert!(msg("en", "engine_missing").contains("not found"));
        assert_eq!(msg("tr", "bilinmeyen-anahtar"), "bilinmeyen-anahtar");
    }
}
