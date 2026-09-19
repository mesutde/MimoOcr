# Dil seçici düzeltmeleri

## uiLangShort yazıyordu
- `t()` artık bilinmeyen anahtarı ekrana yazmaz
- Başlık etiketi `UI_LANG_SHORT` haritasından gelir:
  - tr → **Dil**, en → **UI**, zh → **界面**, ar → **اللغة**, ru → **Язык**, ja → **UI**
- Motor etiketi de `t("lblEngine")` ile güncellenir (Motor / Engine / …)

## Konum
Arayüz dili seçicisi **başlıkta**, Motor alanının **solunda**:
```
[ UI | Dil ▾ ]  [ Motor | Tesseract ▾ ]  [on device]
```

## Dil listesi (eksik olanlar eklendi)
| Kod | Etiket |
|-----|--------|
| en | English |
| tr | Türkçe |
| es | Español |
| fr | Français |
| de | Deutsch |
| pt | Português |
| **zh** | **中文 (简体)** |
| **ja** | **日本語** |
| **ar** | **العربية** |
| ru | Русский |
| hi | हिन्दी |

Liste yeniden oluşturulurken zh/ja/ar zorunlu tutulur.

## Rozet
`cihazda` / `on device` çevirisi `statusDevice` ile dile göre değişir.

## Deneme
1. Başlıkta dil listesinde **中文, 日本語, العربية** görünmeli  
2. Etiket **uiLangShort** değil **Dil / UI / 界面…** olmalı  
3. Dil değiştirince rozet ve sekmeler de değişmeli  
