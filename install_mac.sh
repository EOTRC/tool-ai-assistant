#!/bin/bash
# ============================================================
#  Установка Tool + toolcmd для macOS одной командой:
#
#    curl -fsSL https://raw.githubusercontent.com/EOTRC/tool-ai-assistant/main/install_mac.sh | bash
#
#  Что делает:
#    1) скачивает бинари из последнего релиза GitHub (под вашу архитектуру: M1-M4 или Intel)
#    2) снимает карантин Gatekeeper (xattr com.apple.quarantine)
#    3) делает бинари исполняемыми
#    4) копирует их в ~/.local/tool и добавляет в PATH (zsh/bash)
#    5) проверяет Ollama, при наличии ставит ИИ-модели (tool install)
# ============================================================

set -e

RELEASE_TAG="v0.2.0"
REPO="EOTRC/tool-ai-assistant"
BASE="https://github.com/$REPO/releases/download/$RELEASE_TAG"
DEST="$HOME/.local/tool"

ARCH="$(uname -m)"
case "$ARCH" in
  arm64|aarch64) SUFFIX="macos-aarch64" ;;
  x86_64|amd64)  SUFFIX="macos-x86_64" ;;
  *)
    echo "Неизвестная архитектура: $ARCH (поддерживаются arm64 и x86_64)"
    exit 1
    ;;
esac

DIR="$(cd "$(dirname "$0")" && pwd)"

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

TOOL_BIN="$(find_bin tool || true)"
TOOLCMD_BIN="$(find_bin toolcmd || true)"

if [ -z "$TOOL_BIN" ] || [ -z "$TOOLCMD_BIN" ]; then
  echo "Бинарей рядом нет — скачиваю из релиза ($SUFFIX)..."
  mkdir -p "$DIR"
  curl -fsSL -o "$DIR/tool"     "$BASE/tool-$SUFFIX"
  curl -fsSL -o "$DIR/toolcmd"  "$BASE/toolcmd-$SUFFIX"
  TOOL_BIN="$DIR/tool"
  TOOLCMD_BIN="$DIR/toolcmd"
fi

echo "Бинари:"
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
