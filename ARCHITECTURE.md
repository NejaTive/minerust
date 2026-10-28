# Архитектура MineRust

Воксельная игра: движок на Rust + wgpu (только WebGPU) собирается в wasm, оболочка — Vue 3 + Vite + TypeScript.

## Движок (`engine/crates`)

Зависимости идут вниз, обратных нет:

```
world ← mesh ← render ← web
world ← game ← render ← web
```

| Крейт    | Роль                                                                                     | GPU / браузер |
| -------- | ---------------------------------------------------------------------------------------- | ------------- |
| `world`  | блоки (`BLOCKS`), колонки и секции, генерация, свет, сохранение (`save.rs`, миграции)    | нет           |
| `mesh`   | мешер: грани, AO, свет вершин                                                            | нет           |
| `game`   | `Game`: мир, игрок, камера, время суток, ввод, правка блоков, `loader` подгрузки         | нет           |
| `render` | `Renderer`: устройство, surface, пайплайны, GPU-меши; `FrameParams` — вход кадра         | wgpu          |
| `web`    | пакет `engine` (cdylib → `engine/pkg`): фасад `Engine`, `Streamer`, `Profiler`, `api.rs` | wasm-bindgen  |

- **`Game`** — вся симуляция, без графики; тестируется на хосте.
- **`Renderer`** не знает про `game`: получает всё через `FrameParams`, которые собирает `web/frame.rs`.
- **`Streamer`** (`web`) связывает их: подгрузка и меширование колонок в бюджете времени, свет, очереди перестройки.
- Единственная граница с JS — `web/src/engine.rs` (методы `Engine`) и `web/src/api.rs` (типы без GPU: `WorldStats`, `blocks()`, `EngineSettings`, `settings_limits()`, `default_settings()`). Типы TypeScript генерирует wasm-bindgen в `engine/pkg/engine.d.ts`.

## Клиент (`src`)

Порядок старта: `main.ts` грузит wasm и читает у движка справочные данные (`engine/info.ts`: блоки, пределы настроек, значения по умолчанию, версия формата), затем монтирует `App.vue`. Значения не дублируются в TS.

| Каталог       | Что внутри                                                                                           |
| ------------- | ---------------------------------------------------------------------------------------------------- |
| `engine/`     | `host.ts` (создание, resize, кадровый цикл), `useEngine.ts` (жизненный цикл), `state.ts`, `apply.ts` |
| `input/`      | `keys.ts`, `wheel.ts` — чистые функции; `bind.ts` — подписки на события; `pointer-lock.ts`           |
| `world/`      | `storage.ts` (IndexedDB), `meta.ts` (версия), `persistence.ts` (загрузка/сохранение), `autosave.ts`  |
| `settings/`   | `settings.ts` (чистая загрузка и зажатие), `hotbar.ts`, `store.ts`, `useSettings.ts`                 |
| `bench/`      | режим `?bench`: разбор адреса, точки, запись кадров, прогон                                          |
| `components/` | `GameCanvas.vue` (тонкая обёртка), `Hud`, `Hotbar`, `PauseMenu` + `menu/` (по файлу на экран)        |

Composables без Pinia — модульные синглтоны: `useSettings()` (настройки, слот хотбара), `useGameState()` (показания движка для HUD), `useEngine()` (жизненный цикл на canvas, вызывается из `GameCanvas.vue`).

Правило для тестируемости: модули с логикой не импортируют `@engine` и Vue; зависимости (хранилище, движок, лимиты) приходят аргументами. Склейка с wasm и DOM — `engine/`, `input/bind.ts`, `components/`.

## Проверки

- Rust (из `engine/`): `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo clippy --target wasm32-unknown-unknown -- -D warnings`, `cargo test`.
- Клиент: `npm run wasm`, затем `npx vue-tsc -b`, `npm run format:check`, `npm run lint`, `npm test`. То же делает CI (задачи `rust` и `web`).
- `src/settings/blocks.fixture.json` — копия `blocks()` для тестов без wasm. Rust-тест `js_fixture_matches_blocks` падает при расхождении; обновить: `UPDATE_FIXTURES=1 cargo test -p engine js_fixture`.

## Рецепты

**Новый блок.** Вариант в `Block` и запись в `Block::ALL` и `BLOCKS` — в конец (номера блока — часть формата сохранения), тайл (см. ниже). Обновить фикстуру. Чтобы блок можно было ставить, добавить имя в `HOTBAR` (`settings/hotbar.ts`); тест проверит имя по фикстуре. Подпись для HUD берётся из `BlockDef.label`.

**Новый тайл.** PNG 16×16 в `engine/crates/world/assets/textures/<имя_в_snake_case>.png`, вариант в `Tile` и в `ALL` (`world/src/tile.rs`); блок ссылается на него в `BLOCKS`. Атлас и иконки хотбара подхватят тайл сами.

**Новая настройка.**

- Если её применяет движок: поле в `EngineSettings`, предел в `SettingsLimits`, значение по умолчанию в `default_settings()` (`web/src/api.rs`), применение в `Engine::apply_settings`; в TS — поле в `EngineSettings`/`SettingsLimits` (`settings/settings.ts`), ключ в `KEYS`, чтение/запись в `loadSettings`/`saveSettings`, преобразование в `engine/info.ts` и `engine/apply.ts`, элемент в `components/menu/SettingsScreen.vue`.
- Чисто клиентская (как `showDebug`): только поле в `Settings`, значение по умолчанию в `defaultSettings`, ключ, элемент меню. Добавить случаи в `settings.test.ts`.

**Новое поле статистики.** Поле с `js_name` и doc-комментарием (единицы) в `WorldStats` (`web/src/api.rs`), заполнить в `Profiler::stats`, показать в `Hud.vue`. Индексов массива нет.

**Новая версия формата сохранения.** Поднять `FORMAT_VERSION` и дописать в конец `MIGRATIONS` шаг «тело версии N → N+1» (`world/src/save.rs`), тест на байты прежней версии оставить. Клиент менять не нужно: версия старше — мир загружается (колонки мигрирует движок), новее — не загружается и не перезаписывается, пока игрок не начнёт новый мир (`world/meta.ts`, тесты `meta.test.ts`, `persistence.test.ts`).

**Новый экран меню.** Значение в `MenuScreen` и заголовок в `menu/screens.ts`, компонент в `components/menu/`, ветка в `PauseMenu.vue`.
