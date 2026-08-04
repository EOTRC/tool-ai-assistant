# ============================================================
#  Установка Tool + toolcmd для Windows
#  Запуск:  powershell -ExecutionPolicy Bypass -File install_windows.ps1
#  Или просто скопируй и вставь в PowerShell.
#  Что делает:
#    1) скачивает tool.exe и toolcmd.exe из релиза GitHub
#    2) кладёт их в ~\tool и добавляет папку в PATH пользователя
#    3) проверяет Ollama и ставит ИИ-модели (tool install)
# ============================================================

$ErrorActionPreference = "Stop"

$base = "https://github.com/EOTRC/tool-ai-assistant/releases/download/v0.2.0"
$dir = Join-Path $env:USERPROFILE "tool"
New-Item -ItemType Directory -Force -Path $dir | Out-Null

Write-Host "Скачивание бинарей..."
Invoke-WebRequest -UseBasicParsing -Uri "$base/tool-windows-x86_64.exe" -OutFile (Join-Path $dir "tool.exe")
Invoke-WebRequest -UseBasicParsing -Uri "$base/toolcmd-windows-x86_64.exe" -OutFile (Join-Path $dir "toolcmd.exe")

$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($userPath -notlike "*$dir*") {
    [Environment]::SetEnvironmentVariable("Path", "$userPath;$dir", "User")
    Write-Host "Папка добавлена в PATH. Открой новый терминал."
} else {
    Write-Host "Папка уже в PATH."
}
$env:Path = "$env:Path;$dir"

Write-Host ""
if (Get-Command ollama -ErrorAction SilentlyContinue) {
    Write-Host "Ollama найдена: $(ollama --version 2>$null)"
} else {
    Write-Host "Ollama CLI не найдена. Скачай с https://ollama.com/download/windows"
}

Write-Host ""
Write-Host "Установка ИИ-моделей (qwen3:1.7b, qwen2.5-coder:7b, qwen2.5vl:3b, nomic-embed-text)..."
if (Get-Command ollama -ErrorAction SilentlyContinue) {
    & (Join-Path $dir "tool.exe") install
} else {
    Write-Host "После установки Ollama выполни: tool install"
}

Write-Host ""
Write-Host "Готово. Проверка:"
Write-Host "  tool --version"
Write-Host "  tool help"
Write-Host "  toolcmd"
