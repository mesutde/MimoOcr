# Tesseract runtime (app-local)
#
# Copies a minimal OCR runtime + TR/EN tessdata for bundling.
# Usage (from repo root):
#   powershell -File scripts\copy_tesseract_runtime.ps1
#
# Source: C:\Program Files\Tesseract-OCR (UB Mannheim build)

param(
    [string]$Source = "C:\Program Files\Tesseract-OCR",
    [string]$Dest = "assets\tesseract-runtime"
)

$ErrorActionPreference = "Stop"
$dest = Join-Path (Get-Location) $Dest
New-Item -ItemType Directory -Force -Path $dest | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $dest "tessdata") | Out-Null

# Core OCR binaries + leptonica + common runtime DLLs (not training tools)
$files = @(
    "tesseract.exe",
    "libtesseract-5.dll",
    "libleptonica-6.dll",
    "libarchive-13.dll",
    "libb2-1.dll",
    "libbrotlicommon.dll",
    "libbrotlidec.dll",
    "libbz2-1.dll",
    "libcairo-2.dll",
    "libcrypto-3-x64.dll",
    "libdatrie-1.dll",
    "libdeflate.dll",
    "libexpat-1.dll",
    "libffi-8.dll",
    "libfontconfig-1.dll",
    "libfreetype-6.dll",
    "libfribidi-0.dll",
    "libgcc_s_seh-1.dll",
    "libgif-7.dll",
    "libgio-2.0-0.dll",
    "libglib-2.0-0.dll",
    "libgmodule-2.0-0.dll",
    "libgobject-2.0-0.dll",
    "libgraphite2.dll",
    "libharfbuzz-0.dll",
    "libiconv-2.dll",
    "libicudt75.dll",
    "libicuin75.dll",
    "libicuuc75.dll",
    "libintl-8.dll",
    "libjbig-0.dll",
    "libjpeg-8.dll",
    "libLerc.dll",
    "liblz4.dll",
    "liblzma-5.dll",
    "libopenjp2-7.dll",
    "libpango-1.0-0.dll",
    "libpangocairo-1.0-0.dll",
    "libpangoft2-1.0-0.dll",
    "libpangowin32-1.0-0.dll",
    "libpcre2-8-0.dll",
    "libpixman-1-0.dll",
    "libpng16-16.dll",
    "libsharpyuv-0.dll",
    "libstdc++-6.dll",
    "libthai-0.dll",
    "libtiff-6.dll",
    "libwebp-7.dll",
    "libwebpmux-3.dll",
    "libwinpthread-1.dll",
    "libzstd.dll",
    "zlib1.dll"
)

$copied = 0
foreach ($name in $files) {
    $src = Join-Path $Source $name
    if (Test-Path $src) {
        Copy-Item $src (Join-Path $dest $name) -Force
        $copied++
    } else {
        Write-Warning "missing: $name"
    }
}

# Prefer project tessdata_fast (tur+eng); fall back to install
$projTess = "assets\models\tessdata"
foreach ($lang in @("tur.traineddata", "eng.traineddata")) {
    $fromProj = Join-Path $projTess $lang
    $fromInst = Join-Path (Join-Path $Source "tessdata") $lang
    $to = Join-Path (Join-Path $dest "tessdata") $lang
    if (Test-Path $fromProj) {
        Copy-Item $fromProj $to -Force
    } elseif (Test-Path $fromInst) {
        Copy-Item $fromInst $to -Force
    } else {
        Write-Warning "tessdata missing: $lang"
    }
}

Write-Host "Copied $copied binaries to $dest"
Get-ChildItem $dest -File | Measure-Object Length -Sum | ForEach-Object {
    Write-Host ("binaries: {0} files, {1:N1} MB" -f $_.Count, ($_.Sum / 1MB))
}
Get-ChildItem (Join-Path $dest "tessdata") -File | ForEach-Object {
    Write-Host ("  tessdata {0} {1:N1} MB" -f $_.Name, ($_.Length / 1MB))
}
