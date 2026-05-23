# 生成 Lively Wallpaper 预览 GIF
# 用法: 
#   1. 先在一个终端运行 `npm run dev`
#   2. 再在另一个终端运行此脚本
#
# 前置要求: ffmpeg (https://ffmpeg.org/download.html)
# 可通过 winget 安装: winget install ffmpeg

$url = "http://localhost:5173"
$output = Join-Path (Split-Path $PSScriptRoot -Parent) "public\preview.gif"

Write-Host "录制壁纸预览 GIF..." -ForegroundColor Cyan
Write-Host "确保 npm run dev 已在运行中 ($url)" -ForegroundColor Yellow
Write-Host ""

# 录制 5 秒，每 0.1 秒一帧 → 50 帧
# 调低质量缩小文件体积
ffmpeg -y `
    -f gdigrab `
    -framerate 10 `
    -video_size 640x360 `
    -i "" `
    -t 5 `
    -vf "fps=10,scale=320:-1:flags=lanczos,split[s0][s1];[s0]palettegen=max_colors=128[p];[s1][p]paletteuse=dither=bayer" `
    $output

if ($LASTEXITCODE -eq 0) {
    Write-Host "预览 GIF 已生成: $output" -ForegroundColor Green
} else {
    Write-Host ""
    Write-Host "生成失败。请确保:" -ForegroundColor Red
    Write-Host "  1. ffmpeg 已安装 (winget install ffmpeg 或 https://ffmpeg.org)" -ForegroundColor Yellow
    Write-Host "  2. npm run dev 已启动" -ForegroundColor Yellow
    Write-Host "  3. 或者手动录制: 用 Lively 的 '导出壁纸' 功能生成 preview.gif" -ForegroundColor Yellow
}
