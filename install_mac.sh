#!/bin/bash
# ============================================================
#  Надёжная установка Tool + toolcmd для macOS
#  Работает и через curl | bash, и локально.
#
#  Использование:
#    curl -fsSL https://raw.githubusercontent.com/EOTRC/tool-ai-assistant/main/install_mac.sh | bash
# ============================================================

set -euo pipefail

REPO="EOTRC/tool-ai-assistant"
DEST="$HOME/.local/tool"
API="https://api.github.com/repos/$REPO/releases/latest"

echo "==> Определение архитектуры..."
ARCH="$(uname -m)"
case "$ARCH" in
  arm64|aarch64) SUFFIX="macos-aarch64" ;;
  x86_64|amd64)  SUFFIX="macos-x86_64" ;;
  *)
    echo "Ошибка: неизвестная архитектура '$ARCH'. Поддерживаются arm64 и x86_64."
    exit 1
    ;;
esac
echo "    Архитектура: $ARCH → $SUFFIX"

echo "==> Получение информации о последнем релизе..."
# Получаем tag последнего релиза
RELEASE_TAG=$(curl -fsSL "$API" | grep -o '"tag_name": *"[^"]*"' | head -1 | cut -d'"' -f4)
if [ -z "$RELEASE_TAG" ]; then
  echo "Ошибка: не удалось получить tag последнего релиза."
  exit 1
fi
echo "    Релиз: $RELEASE_TAG"

BASE="https://github.com/$REPO/releases/download/$RELEASE_TAG"
TOOL_URL="$BASE/tool-$SUFFIX"
TOOLCMD_URL="$BASE/toolcmd-$SUFFIX"

# Проверяем, что файлы существуют в релизе
echo "==> Проверка наличия бинарей в релизе..."
if ! curl -fsSL -I -o /dev/null -w "%{http_code}" "$TOOL_URL" | grep -q "200"; then
  echo "Ошибка: файл tool-$SUFFIX не найден в релизе $RELEASE_TAG"
  echo "Доступные файлы можно посмотреть здесь:"
  echo "  https://github.com/$REPO/releases/tag/$RELEASE_TAG"
  exit 1
fi
if ! curl -fsSL -I -o /dev/null -w "%{http_code}" "$TOOLCMD_URL" | grep -q "200"; then
  echo "Ошибка: файл toolcmd-$SUFFIX не найден в релизе $RELEASE_TAG"
  exit 1
fi

echo "==> Создание директории $DEST..."
mkdir -p "$DEST"

TMPDIR=$(mktemp -d)
trap 'rm -rf "$TMPDIR"' EXIT

echo "==> Скачивание tool..."
curl -fsSL -o "$TMPDIR/tool" "$TOOL_URL"
echo "==> Скачивание toolcmd..."
curl -fsSL -o "$TMPDIR/toolcmd" "$TOOLCMD_URL"

# Проверяем, что файлы не пустые
if [ ! -s "$TMPDIR/tool" ] || [ ! -s "$TMPDIR/toolcmd" ]; then
  echo "Ошибка: скачанные файлы пустые или повреждены."
  exit 1
fi

echo "==> Снятие quarantine и установка прав..."
xattr -d com.apple.quarantine "$TMPDIR/tool" 2>/dev/null || true
xattr -d com.apple.quarantine "$TMPDIR/toolcmd" 2>/dev/null || true
chmod +x "$TMPDIR/tool" "$TMPDIR/toolcmd"

echo "==> Копирование в $DEST..."
cp -f "$TMPDIR/tool"     "$DEST/tool"
cp -f "$TMPDIR/toolcmd"  "$DEST/toolcmd"

# Опционально — settings.cfg.template
TEMPLATE_URL="$BASE/settings.cfg.template"
if curl -fsSL -I -o /dev/null -w "%{http_code}" "$TEMPLATE_URL" 2>/dev/null | grep -q "200"; then
  curl -fsSL -o "$DEST/settings.cfg.template" "$TEMPLATE_URL" || true
fi

echo "==> Добавление $DEST в PATH..."

add_to_rc() {
  local rc="$1"
  local line='export PATH="$HOME/.local/tool:$PATH"'
  # Создаём файл, если его нет (особенно важно для .zprofile / .bash_profile)
  touch "$rc" 2>/dev/null || return 0
  if ! grep -qF '.local/tool' "$rc" 2>/dev/null; then
    echo "" >> "$rc"
    echo "# Tool AI Assistant" >> "$rc"
    echo "$line" >> "$rc"
    echo "    Добавлено в $rc"
  else
    echo "    Уже есть в $rc"
  fi
}

# zsh (основной shell на современных macOS)
add_to_rc "$HOME/.zshrc"
add_to_rc "$HOME/.zprofile"

# bash
add_to_rc "$HOME/.bash_profile"
add_to_rc "$HOME/.bashrc"
add_to_rc "$HOME/.profile"

# Сразу делаем доступным в текущей сессии
export PATH="$HOME/.local/tool:$PATH"

echo ""
echo "✓ Tool установлен:"
echo "    tool     → $DEST/tool"
echo "    toolcmd  → $DEST/toolcmd"
echo ""

# Проверка, что команды находятся
if ! command -v tool >/dev/null 2>&1; then
  echo "Внимание: 'tool' пока не в PATH этой сессии."
  echo "Выполни:  source ~/.zshrc   (или открой новый терминал)"
else
  echo "✓ Команда 'tool' доступна в текущей сессии"
  tool --version 2>/dev/null || tool help 2>/dev/null || true
fi

echo ""
echo "==> Установка Ollama (последняя версия)..."
if "$DEST/tool" install ollama; then
  echo "✓ Ollama установлена"
else
  echo "⚠ Не удалось установить Ollama автоматически."
  echo "  Можно поставить вручную: https://ollama.com/download"
fi

echo ""
echo "==> Установка ИИ-моделей (может занять несколько минут, ~8 ГБ)..."
if "$DEST/tool" install models; then
  echo "✓ Модели установлены"
else
  echo "⚠ Не удалось установить модели."
  echo "  Позже выполни: tool install models"
fi

echo ""
echo "============================================================"
echo "  Готово!"
echo ""
echo "  Открой НОВЫЙ терминал (или выполни: source ~/.zshrc)"
echo "  и проверь:"
echo ""
echo "    tool --version"
echo "    tool help"
echo "    toolcmd"
echo "    tool selftest"
echo "============================================================"
