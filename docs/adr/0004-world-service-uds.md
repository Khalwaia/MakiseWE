---
status: accepted-for-legacy-runtime
date: 2026-08-05
updated: 2026-08-19
---

# WorldService uses a bounded actor and Unix Domain Socket

Текущий executable использует отдельный World Engine process, bounded actor queue и persistent gRPC/HTTP2 connection через Unix Domain Socket с permissions `0600`. Rust владеет state и SQLite; C++ WorldClient использует generated Protobuf/gRPC bindings.

Это legacy runtime adapter для migration, а не новый authority boundary. RPC не добавляет mutation path: будущий transport переводит requests в `WorldEngine::commit`, `project` и `events`, не выдавая references на state или DB.

Переполненная очередь возвращает `RESOURCE_EXHAUSTED`. Event публикуется только после durable append. Subscriber после lag/reconnect продолжает с durable cursor. Timeout использует исходный request ID.

Следствия: Brain failure не останавливает committed physical processes; медленный subscriber не удерживает writer; UDS, DB и admin interfaces не публикуются напрямую; protocol migration сохраняет legacy reader до отдельного contraction decision.
