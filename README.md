# Makise

Отдельная кодовая база для автономной цифровой личности Makise. Проект не является
обновлением работающей Мины и не использует её runtime, память, Telegram-сессию или
дневник.

## Текущий исполнимый срез

- Protobuf V1: handshake, commands, results, events, perception и health.
- Rust makise-world: single-writer domain core.
- Bounded actor и gRPC WorldService по Unix Domain Socket.
- C++20 WorldClient с generated protobuf/gRPC bindings для будущего Brain.
- SQLite WAL event log, command deduplication, snapshots и deterministic replay.
- Неблокирующая погода Open-Meteo: типизированные снимки проходят физическую проверку, сохраняются как события и переживают replay/offline.
- Durable activities с реальным временем и восстановлением после downtime.
- Ресурсные конфликты вместо глобального BUSY.
- Частичное восприятие: скрытые свойства package не попадают в PerceptionWindow.
- Path guard, блокирующий защищённый runtime Мины до открытия БД.

Сейчас это тестовое ядро, а не production-запуск Makise. Оно не подключено к
Telegram, LLM, новой памяти или /home/artem/makise_run.

## Проверка

Из /home/artem/makise:

    . "$HOME/.cargo/env"
    cargo fmt --all -- --check
    cargo clippy --workspace --all-targets -- -D warnings
    cargo test --workspace --all-targets

Проверка тестового world package:

    cargo run -p makise-world -- verify-package \
      /home/artem/makise/world-packages/test-room-v1/manifest.json

Детализированная квартира проверяется той же командой:

    cargo run -p makise-world -- verify-package \\
      /home/artem/makise/world-packages/apartment-v1/manifest.json

Команда status создаёт БД, поэтому для разработки ей передают только временный
абсолютный путь вне production runtime:

    cargo run -p makise-world -- status \
      /tmp/makise-dev/world.db \
      /home/artem/makise/world-packages/test-room-v1/manifest.json \
      test-makise bed

Локальный WorldService запускается только на отдельном тестовом runtime-пути:

    cargo run -p makise-world -- serve \
      /tmp/makise-dev/world.sock \
      /tmp/makise-dev/world.db \
      /home/artem/makise/world-packages/test-room-v1/manifest.json \
      test-makise bed

Для `apartment-v1` сервис сразу запускает фоновый опрос Open-Meteo. При сетевом сбое последнее подтверждённое состояние остаётся в БД; `MAKISE_WEATHER_ENDPOINT` можно задать только для локального proxy или тестового сервера.

Socket получает права `0600` и удаляется владельцем после штатной остановки. Уже
существующий путь сервис не перезаписывает.

## C++ клиент Brain

Однократно установите development-зависимости в Ubuntu (команда запросит ваш пароль
`sudo`):

    sudo apt-get install -y --no-install-recommends \
      libprotobuf-dev protobuf-compiler protobuf-compiler-grpc libgrpc++-dev

После этого bindings и клиент собираются из одного нормативного proto-файла:

    cmake -S brain -B build/brain -DCMAKE_BUILD_TYPE=RelWithDebInfo
    cmake --build build/brain --parallel
    ctest --test-dir build/brain --output-on-failure

## Структура

- world — объективное состояние, события, таймеры и восприятие.
- proto — стабильный wire contract Rust/C++.
- world-packages — версионированные определения миров.
- brain, memory, panel, gateway — изолированные будущие компоненты.
- identity — versioned identity package, не runtime-память.
- deploy — systemd, releases и recovery tooling.
- tests — межсервисные, replay и fault-сценарии.
- docs/adr — принятые необратимые технические решения.

Нормативные требования находятся в INVARIANTS.md; дорожная карта — в ROADMAP.md.

