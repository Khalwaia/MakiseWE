# ADR-0004: WorldService работает через bounded actor и Unix Domain Socket

Статус: принят  
Дата: 2026-08-05

## Контекст

Brain написан на C++, World Engine — на Rust. Прямой FFI сделал бы время жизни,
падения и владение SQLite частью одного процесса. Прямой доступ Brain к базе нарушил
бы single-writer и позволил бы обходить проверку команд.

## Решение

- `makise-world` владеет единственным `WorldEngine` в отдельном actor-потоке.
- RPC handlers передают запросы в ограниченную очередь и не получают ссылок на
  `WorldState` или SQLite.
- Локальный транспорт — постоянный gRPC/HTTP2 поверх Unix Domain Socket с правами
  `0600`.
- Переполненная очередь немедленно возвращает `RESOURCE_EXHAUSTED`.
- События сначала фиксируются в SQLite и только затем публикуются подписчикам.
- Subscriber после lag или reconnect продолжает с `after_seq` из durable event log.
- Повтор команды использует исходный `command_id`; сетевой timeout не создаёт новую
  команду.
- C++ Brain использует generated protobuf/gRPC bindings и тонкий `WorldClient`, а не
  собственные JSON-модели протокола.

## Последствия

- Падение Brain не останавливает clock и durable activities мира.
- Медленный подписчик не удерживает writer thread.
- У процесса World Engine есть явная граница доверия и независимый lifecycle.
- Для сборки C++ клиента требуются системные protobuf/gRPC development packages.
- UDS не публикуется наружу; gateway и панель не получают к нему прямой доступ.
