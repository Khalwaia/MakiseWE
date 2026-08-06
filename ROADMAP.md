# Дорожная карта Makise V1

Статус: зафиксированная архитектурная база V1  
Дата: 2026-08-05  
Связанные документы: [VISION.md](VISION.md), [ARCHITECTURE.md](ARCHITECTURE.md), [WORLD_V1.md](WORLD_V1.md), [INVARIANTS.md](INVARIANTS.md)

## 1. Правило выполнения

Каждый этап заканчивается исполняемым результатом, тестами и проверяемым gate. Следующий этап не должен скрывать незавершённые инварианты предыдущего.

До завершения V1 новые идеи помещаются в backlog, если они не исправляют угрозу данным или фундаментальную архитектурную ошибку.

## 2. Этап 0: изоляция и документация

Результат:

- отдельный `/home/artem/makise`;
- отдельный будущий `/home/artem/makise_run`;
- восемь согласованных архитектурных документов;
- автоматический path guard против данных Мины;
- структура монорепозитория;
- базовые ADR для необратимых решений.

Gate:

- ни одна конфигурация, команда или тест не разрешает путь внутри `/home/artem/kuni_run`;
- документы проходят проверку ссылок и терминологии;
- нет runtime-данных Makise в Git.

## 3. Этап 1: контракты

Результат:

- Protobuf packages для команд, событий, восприятия, health и admin;
- типизированные ID и error model;
- JSON Schema world package;
- version handshake;
- C++ и Rust generated bindings.

Gate:

- backward/forward compatibility tests;
- invalid/fuzz payloads отклоняются;
- одна команда с одним ID не может выполниться дважды.

## 4. Этап 2: детерминированное ядро мира

Результат:

- Rust single-writer core;
- управляемые clock interfaces;
- SQLite WAL event store;
- durable timers;
- snapshots и replay;
- test/simulation mode;
- saved seeded PRNG hooks.

Gate:

- одинаковый log приводит к одинаковому state hash;
- restart между любыми двумя событиями не меняет результат;
- clock jump переводит систему в `TIME_ANOMALY`;
- duplicate command возвращает исходный результат.

## 5. Этап 3: первая сквозная вертикаль

Минимальный сценарий:

1. Test-Makise пробуждается в одной комнате.
2. Получает ограниченное восприятие.
3. Fake Brain выбирает перемещение.
4. World Engine проверяет и исполняет действие с длительностью.
5. Объект изменяет наблюдаемое состояние.
6. Событие превращается в subjective event.
7. Memory stub сохраняет и возвращает его.
8. Replay воспроизводит весь сценарий.

Gate:

- нет прямого state mutation вне command handler;
- stale Brain response отклоняется;
- падение клиента не блокирует мир.

## 6. Этап 4: полная квартира

Результат:

- topology и 27 anchors;
- surfaces, containers и object templates;
- стартовая обстановка;
- свет, звук, температура и запах;
- размещение/перестановка предметов;
- electricity, charge, cleanliness, quantities и temperature;
- погода Новосибирска с fallback;
- визуальные SVG-координаты и пути.

Gate:

- validation всего world package;
- все anchors достижимы;
- предмет нельзя поместить в невозможное место;
- скрытое состояние не попадает в perception.

### 6.1 Подэтапы реализации Stage 4

- 4B.1: пакет квартиры, топология, карта, шаблоны и стартовая обстановка — завершён;
- 4B.2: типизированные параметры действий и проверяемая перестановка предметов — завершён;
- 4B.3: типизированные charge, cleanliness, quantity и temperature; event-sourced действия clean/consume; динамические affordances — завершён;
- 4B.4: детерминированная пассивная зарядка, остывание/нагрев и расход по абсолютному времени с replay/downtime — завершён;
- 4C.1: неблокирующий Open-Meteo, проверяемые типизированные снимки, долговечный cache/replay и сохранение последнего состояния при сбое — завершён;
- 4C.2: динамические поля света, звука, температуры и запаха с явной уверенностью live/cache/seasonal fallback.

Перенос предмета между anchors требует ресурсов тела и относится к Stage 5. В Stage 4 перестановка ограничена текущим anchor.

## 7. Этап 5: тело, действия и внутренняя динамика

Результат:

- resource-based parallel activities;
- soft body needs;
- сон 1:1 и свободно меняющийся циркадный режим;
- Affect Engine;
- motivation candidates, commitments и goal inertia;
- skill learning;
- recipes и гардероб;
- adaptive cognition scheduler.

Gate:

- конфликтующие ресурсы не используются одновременно;
- сон переживает restart/downtime;
- настроение имеет объяснимые причины;
- активная цель не меняется без события или достаточной причины;
- без LLM мир не придумывает новые решения.

## 8. Этап 6: Brain и память

Результат:

- отдельный C++ brain fork;
- provider-independent main/fallback adapters;
- стабильный tool protocol;
- while-thinking buffer;
- cache-aware prompt layout и блочная ротация;
- `makise-memory`, hybrid retrieval и privacy provenance;
- durable memory outbox;
- автономный append-only diary;
- identity package Makise.

Gate:

- 400/422/429 не вызывают бесконечный retry;
- dynamic context не меняет стабильный prompt prefix;
- objective unseen event не становится memory;
- memory outage не теряет subjective events;
- ни одно воспоминание Мины не импортировано.

## 9. Этап 7: телефон и закрытый Telegram

Результат:

- отдельный аккаунт и TDLib session;
- notification/read/reply state machine;
- единый последовательный inbox;
- read/typing semantics;
- privacy guard;
- allowlist, mute, block и rate limiting;
- безопасные текст, stickers, reactions и attachments.

Gate:

- непрочитанный текст отсутствует в сознании;
- повторная отправка после timeout не дублирует сообщение;
- чужая приватная информация блокируется до отправки;
- Telegram Makise не использует сессию Мины.

## 10. Этап 8: панель и эксплуатация

Результат:

- React/TypeScript PWA;
- SVG-карта и semantic graph;
- timeline, decision trace и метрики;
- бюджеты провайдеров;
- audit и ограниченные admin actions;
- passkey/2FA, gateway и VPN policy;
- systemd services, health checks и watchdog;
- atomic releases и согласованные backups;
- отдельный admin notifier.

Gate:

- панель read-only по умолчанию;
- критические действия требуют повторной аутентификации;
- скрытый текст не отображается без debug session;
- два процесса с одним `identity_id` не запускаются.

## 11. Этап 9: медиа и кодовый помощник

Результат:

- persistent music player;
- canonical AppearanceProfile и image adapter;
- stable VoiceProfile, TTS/STT;
- причинный soundscape mixer;
- code model adapter и isolated worktree;
- patch review и allowlisted tests.

Gate:

- фото соответствует world snapshot;
- soundscape не добавляет отсутствующий источник;
- кодовая модель не видит секреты/runtime;
- patch не может развернуться без явного одобрения.

## 12. Этап 10: испытания

### Автоматические

- unit tests;
- property-based invariants;
- deterministic replay;
- fuzzing protocol и packages;
- integration tests;
- fault/chaos tests;
- C++ sanitizers и Rust strict checks;
- тест запрета путей Мины.

### Симуляция

- 30 виртуальных дней;
- несколько циклов сна и сбитого режима;
- cooking, notification storms, memory outage и provider limits;
- service restarts, duplicate delivery и clock anomaly;
- итоговый replay state hash совпадает.

### Реальный shadow

- 8–12 часов суммарной работы без Telegram, не обязательно подряд;
- минимум один длительный процесс через выключение ПК;
- намеренный restart каждого сервиса;
- корректная субъективная отметка разрыва времени.

### Закрытый запуск

- 7 календарных дней только с Артёмом; непрерывный uptime не требуется;
- затем allowlist;
- потом invite-only;
- публичные DM только после отдельного security review.

## 13. Критерии готовности V1

V1 готова, когда одновременно выполнено следующее:

- World Engine устойчиво хранит и воспроизводит состояние;
- Makise самостоятельно выбирает занятия и осознанный отдых;
- сон и процессы соответствуют реальному времени;
- память, дневник и отношения переживают restart;
- partial observability не раскрывает скрытое;
- Telegram является телефоном, а не прямым prompt injection;
- панель объясняет решение без ложного chain-of-thought;
- budgets и failures приводят к контролируемой паузе;
- backups проверены восстановлением;
- отсутствуют обращения к runtime Мины;
- пройдены simulation, shadow и закрытый запуск.

## 14. Backlog после V1

- подъезд, двор, город и транспорт;
- NPC и питомцы;
- voice/video calls и live streaming;
- причинные вероятностные аварии;
- группы Telegram;
- реальные финансовые интеграции только после отдельного threat model;
- preference/LoRA tuning на вручную одобренных обезличенных примерах;
- расширенная экономика;
- предложения Makise по собственному коду, прошедшие полный release pipeline.

