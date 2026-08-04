#!/bin/bash
# ============================================================
#  Установка Tool + toolcmd для macOS
#  Запусти:  bash install_mac.sh
#  Скрипт должен лежать рядом с бинарями tool и toolcmd.
#  Что делает:
#    1) снимает карантин Gatekeeper (xattr com.apple.quarantine)
#    2) делает бинари исполняемыми
#    3) копирует их в ~/.local/tool и добавляет в PATH (zsh/bash)
#    4) проверяет Ollama, при наличии ставит ИИ-модели (tool install)
# ============================================================

set -e

DIR="$(cd "$(dirname "$0")" && pwd)"
DEST="$HOME/.local/tool"

find_bin() {
  local name="$1" f base
  for f in "$DIR"/*; do
    [ -f "$f" ] || continue
    base="$(basename "$f")"
    case "$base" in
      toolcmd*|settings.cfg*|*.sh|install*|LICENSE*|README*) continue ;;
    esac
    case "$base" in
      "$name"|"$name"[-.]*) printf '%s' "$f"; return 0 ;;
    esac
  done
  return 1
}

TOOL_BIN="$(find_bin tool)" || { echo "ОШИБКА: не найден бинарь 'tool' рядом со скриптом."; exit 1; }
TOOLCMD_BIN="$(find_bin toolcmd)" || { echo "ОШИБКА: не найден бинарь 'toolcmd' рядом со скриптом."; exit 1; }

echo "Найдены:"
echo "  $TOOL_BIN"
echo "  $TOOLCMD_BIN"

for f in "$TOOL_BIN" "$TOOLCMD_BIN"; do
  xattr -d com.apple.quarantine "$f" 2>/dev/null || true
  chmod +x "$f"
done

mkdir -p "$DEST"
cp -f "$TOOL_BIN" "$DEST/tool"
cp -f "$TOOLCMD_BIN" "$DEST/toolcmd"
if [ -f "$DIR/settings.cfg.template" ]; then
  cp -f "$DIR/settings.cfg.template" "$DEST/settings.cfg.template"
fi

add_to_rc() {
  local rc="$1"
  [ -f "$rc" ] || return 0
  grep -qF 'export PATH="$HOME/.local/tool:$PATH"' "$rc" \
    || echo 'export PATH="$HOME/.local/tool:$PATH"' >> "$rc"
}
add_to_rc "$HOME/.zshrc"
add_to_rc "$HOME/.bash_profile"
add_to_rc "$HOME/.bashrc"

export PATH="$HOME/.local/tool:$PATH"

echo ""
echo "Установлено:"
echo "  tool    -> $DEST/tool"
echo "  toolcmd -> $DEST/toolcmd"
echo "PATH добавлен в ~/.zshrc, ~/.bash_profile, ~/.bashrc"
echo "Открой новый терминал (или: source ~/.zshrc) — команды tool и toolcmd будут доступны отовсюду."
echo ""

if command -v ollama >/dev/null 2>&1; then
  echo "Ollama CLI найдена: $(ollama --version 2>/dev/null || echo '?')"
else
  echo "Ollama CLI не найдена."
  if command -v brew >/dev/null 2>&1; then
    echo "Установить Ollama через Homebrew? [y/N]"
    read -r ans
    if [ "$ans" = "y" ] || [ "$ans" = "Y" ]; then
      brew install ollama
    fi
  else
    echo "Скачай Ollama с https://ollama.com/download/macOS"
    echo "или установи Homebrew: /bin/bash -c \"\$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)\""
  fi
fi

echo ""
echo "Установка ИИ-моделей (это займёт время, ~8 ГБ):"
echo "  qwen3:1.7b, qwen2.5-coder:7b, qwen2.5vl:3b, nomic-embed-text"
echo ""
if command -v ollama >/dev/null 2>&1; then
  if curl -s --max-time 3 http://localhost:11434/api/version >/dev/null 2>&1; then
    "$DEST/tool" install
  else
    echo "Ollama не запущена. Запусти Ollama.app или 'ollama serve', затем выполни: tool install"
  fi
else
  echo "После установки Ollama выполни: tool install"
fi

echo ""
echo "Готово! Проверка:"
echo "  tool --version"
echo "  tool help"
echo "  toolcmd"
