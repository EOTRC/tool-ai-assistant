Я собрал этот инструмент для себя, билд делаю для друга только поэтому оно здесь, этот инструмент полностью написан ИИ.


# Tool — локальный ИИ-ассистент

Консольный ИИ-ассистент для локальной работы с Ollama. Чистый Rust, без Python.
Свои команды (chat, файловые операции, RAG, скриншоты) + обычные команды
системы (как в cmd).

## Возможности

- `chat` — диалог с моделью со стримингом (`exit`/`пока` — выход)
- `ask`, `ask-file`, `code`, `summarize`, `translate` — работа с файлами
- `index` / `ask` (RAG) — поиск по своей папке через эмбеддинги
- `clip` / `screen` — буфер обмена и скриншоты с запросом к vision-модели
- `web` — поиск в интернете
- `selftest` — встроенная самодиагностика
- `toolcmd` — интерактивная консоль: команды Tool без префикса + алиасы

Настройки — `settings.cfg` рядом с бинарём (копия `rs/settings.cfg.template`):
модели, устройства `cpu`/`gpu`, язык ответов, режим думанья, системный промт.

## Сборка

Требуется [Rust](https://rustup.rs) + [Ollama](https://ollama.com) с моделями
(по умолчанию `qwen3:1.7b`, см. `settings.cfg`).

```sh
cd rs
cargo build --release
# бинарь: target/release/tool (+ target/release/toolcmd)
```

Для другой платформы добавьте `--target`:

```sh
cargo build --release --target x86_64-pc-windows-gnu    # Windows
cargo build --release --target x86_64-unknown-linux-gnu # Linux
cargo build --release --target aarch64-apple-darwin     # macOS (M1/M2/M3)
cargo build --release --target x86_64-apple-darwin      # macOS (Intel)
```

## Готовые бинари (CI)

GitHub Actions собирает бинари для всех платформ автоматически
(`.github/workflows/build-all.yml`): Windows, Linux, macOS на чипах Intel и
Apple Silicon (M1/M2/M3). Артефакты — во вкладке Actions.

## Использование

```
Tool chat                        # диалог
Tool screen "что на экране?"     # скриншот + vision-модель
Tool summarize doc.txt           # краткое содержание файла
toolcmd                          # интерактивная консоль (help для справки)
```

## Лицензия

MIT — см. `LICENSE`.
