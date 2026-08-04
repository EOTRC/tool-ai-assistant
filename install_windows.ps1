# ============================================================
#  Установка Tool + toolcmd + Ollama + моделей для Windows
#  Запуск:  powershell -ExecutionPolicy Bypass -File install_windows.ps1
#  Или просто скопируй и вставь в PowerShell.
#  Что делает:
#    1) скачивает tool.exe и toolcmd.exe из релиза GitHub
#    2) кладёт их в ~\tool и добавляет папку в PATH пользователя
#    3) ставит последнюю версию Ollama (tool install ollama)
#    4) ставит ИИ-модели (tool install models)
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
    Write-Host "Папка добавлена в PATH (пользовательский)."
} else {
    Write-Host "Папка уже в PATH."
}
$env:Path = "$env:Path;$dir"
Write-Host ""
Write-Host "ВАЖНО: если tool/toolcmd не находятся в уже открытых терминалах —"
Write-Host "закрой и открой терминал заново (PATH подхватывается новым окном)."
Write-Host "В этом же окне они уже доступны."

Write-Host ""
Write-Host "==> Установка последней версии Ollama..."
& (Join-Path $dir "tool.exe") install ollama

Write-Host ""
Write-Host "==> Установка ИИ-моделей (~8 ГБ, может занять время)..."
& (Join-Path $dir "tool.exe") install models

Write-Host ""
Write-Host "Готово. Проверка:"
Write-Host "  tool --version"
Write-Host "  tool help"
Write-Host "  toolcmd"
