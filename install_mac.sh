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

echo "==> Копирование в $DEST (имена файлов: tool, toolcmd)..."
chmod +x "$TMPDIR/tool" "$TMPDIR/toolcmd"
cp -f "$TMPDIR/tool"     "$DEST/tool"
cp -f "$TMPDIR/toolcmd"  "$DEST/toolcmd"

# Обязательно: файлы должны называться ровно tool и toolcmd
if [ ! -f "$DEST/tool" ] || [ ! -f "$DEST/toolcmd" ]; then
  echo "Ошибка: в $DEST должны лежать файлы с именами 'tool' и 'toolcmd'."
  exit 1
fi

# Обход Gatekeeper на уже установленных файлах (quarantine копируется вместе с файлом)
echo "==> Обход Gatekeeper (xattr -d com.apple.quarantine)..."
for f in "$DEST/tool" "$DEST/toolcmd"; do
  xattr -d com.apple.quarantine "$f" 2>/dev/null || true
  chmod +x "$f"
  echo "    $f"
done

# Опционально — settings.cfg.template
TEMPLATE_URL="$BASE/settings.cfg.template"
if curl -fsSL -I -o /dev/null -w "%{http_code}" "$TEMPLATE_URL" 2>/dev/null | grep -q "200"; then
  curl -fsSL -o "$DEST/settings.cfg.template" "$TEMPLATE_URL" || true
fi

echo "==> Добавление $DEST в PATH и алиасов tool/toolcmd..."

# Находим реальный каталог конфигов zsh: ZDOTDIR может быть не $HOME
ZD="${ZDOTDIR:-}"
if [ -z "$ZD" ]; then
  ZD="$(zsh -lc 'printf %s "$ZDOTDIR"' 2>/dev/null || true)"
fi
ZD="${ZD:-$HOME}"
echo "    ZDOTDIR=$ZD"

add_to_rc() {
  local rc="$1"
  local line='export PATH="$HOME/.local/tool:$PATH"'
  local alias_tool='alias tool="$HOME/.local/tool/tool"'
  local alias_toolcmd='alias toolcmd="$HOME/.local/tool/toolcmd"'
  # Создаём файл, если его нет (особенно важно для .zprofile / .bash_profile)
  touch "$rc" 2>/dev/null || return 0
  if ! grep -qF '.local/tool' "$rc" 2>/dev/null; then
    echo "" >> "$rc"
    echo "# Tool AI Assistant" >> "$rc"
    echo "$line" >> "$rc"
    echo "$alias_tool" >> "$rc"
    echo "$alias_toolcmd" >> "$rc"
    echo "    Добавлено в $rc"
  else
    echo "    Уже есть в $rc"
  fi
}

# zsh (основной shell на современных macOS), с учётом ZDOTDIR
add_to_rc "$ZD/.zshrc"
[ -f "$ZD/.zprofile" ] && add_to_rc "$ZD/.zprofile"
[ -f "$ZD/.zshenv" ] && add_to_rc "$ZD/.zshenv"

# bash
add_to_rc "$HOME/.bash_profile"
add_to_rc "$HOME/.bashrc"
add_to_rc "$HOME/.profile"

# Сразу делаем доступным в текущей сессии
export PATH="$HOME/.local/tool:$PATH"
eval "$(echo 'alias tool="$HOME/.local/tool/tool"')"
eval "$(echo 'alias toolcmd="$HOME/.local/tool/toolcmd"')"

# Проверяем xattr после установки: если карантин остался — снимаем ещё раз
echo "==> Проверка xattr (карантин должен быть снят)..."
Q=0
for f in "$DEST/tool" "$DEST/toolcmd"; do
  if xattr -p com.apple.quarantine "$f" >/dev/null 2>&1; then
    echo "    Карантин найден на $f — снимаю (xattr -c)..."
    xattr -c "$f" 2>/dev/null || true
    xattr -d com.apple.quarantine "$f" 2>/dev/null || true
    Q=1
  fi
done
if [ "$Q" = "0" ]; then
  echo "    OK: карантин снят с tool и toolcmd"
else
  echo "    Готово: карантин снят повторно"
fi

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
