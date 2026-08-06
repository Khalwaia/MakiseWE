# Протоколы Makise V1

Статус: зафиксированная архитектурная база V1  
Дата: 2026-08-05  
Связанные документы: [ARCHITECTURE.md](ARCHITECTURE.md), [SECURITY.md](SECURITY.md), [INVARIANTS.md](INVARIANTS.md)

## 1. Цели протокола

- высокая скорость локального контура;
- строгие типы и совместимость Rust/C++;
- идемпотентные команды;
- восстановление после разрыва соединения;
- явная защита от устаревших решений;
- эволюция схем без переписывания истории;
- наблюдаемость без утечки содержимого.

## 2. Транспорт

Внутренний контур `makise-brain <-> makise-world` использует Protobuf RPC и server-streaming events по постоянному Unix Domain Socket. Реализация может использовать gRPC/HTTP2, но нижеприведённая семантика обязательна независимо от библиотеки.

Панель использует HTTPS/JSON и WebSocket через gateway. Внешний JSON не является альтернативным путём изменения мира: gateway преобразует разрешённое действие в ту же внутреннюю команду.

## 3. Идентификаторы

Все идентификаторы непрозрачны, стабильны и типизированы:

- `identity_id` — личность Makise;
- `agent_id` — действующий субъект;
- `command_id` — UUID/ULID команды;
- `event_id` — уникальное событие;
- `event_seq` — монотонная последовательность журнала;
- `world_version` — версия состояния после последнего commit;
- `schema_version` — версия сообщения;
- `world_definition_hash` — канонический hash package мира;
- `decision_id` — один осознанный цикл Brain;
- `perception_id` — snapshot доступного восприятия.

## 4. CommandEnvelope

Нормативные поля:

```proto
message CommandEnvelope {
  string command_id = 1;
  string identity_id = 2;
  string agent_id = 3;
  uint64 expected_world_version = 4;
  uint32 schema_version = 5;
  string decision_id = 6;
  google.protobuf.Timestamp issued_at = 7;
  google.protobuf.Duration ttl = 8;
  CommandPayload payload = 9;
}
```

Правила:

- `command_id` создаётся клиентом один раз и сохраняется при retry.
- Несовпадение `expected_world_version` возвращает `STALE_WORLD` без выполнения.
- Истёкший TTL возвращает `EXPIRED_DECISION`.
- Команда подтверждается только после durable commit результата.
- Повтор известного `command_id` возвращает сохранённый результат.

## 5. CommandResult

```proto
message CommandResult {
  string command_id = 1;
  CommandStatus status = 2;
  uint64 committed_world_version = 3;
  uint64 first_event_seq = 4;
  uint64 last_event_seq = 5;
  ErrorDetail error = 6;
  repeated Affordance suggested_recovery = 7;
}
```

Статусы минимум:

- `COMMITTED`;
- `ALREADY_COMMITTED`;
- `REJECTED_PRECONDITION`;
- `RESOURCE_CONFLICT`;
- `STALE_WORLD`;
- `EXPIRED_DECISION`;
- `UNAUTHORIZED`;
- `INVALID_ARGUMENT`;
- `RATE_LIMITED`;
- `TEMPORARILY_UNAVAILABLE`;
- `INTERNAL_ERROR`.

Сетевой timeout не означает, что команда не выполнена. Клиент обязан запросить результат по `command_id` перед созданием новой команды.

## 6. EventEnvelope

```proto
message EventEnvelope {
  string event_id = 1;
  uint64 event_seq = 2;
  uint64 world_version = 3;
  uint32 event_schema_version = 4;
  google.protobuf.Timestamp occurred_at = 5;
  optional string causation_command_id = 6;
  optional string correlation_id = 7;
  EventPayload payload = 8;
}
```

События публикуются только после commit и всегда в порядке `event_seq`. Клиент подтверждает последний устойчиво обработанный номер. После reconnect запрашивается `SubscribeEvents(after_seq)`.

Пропуск последовательности является ошибкой синхронизации; клиент не продолжает на неполной истории.

## 7. PerceptionWindow

Brain получает не полный JSON мира, а компактное типизированное восприятие:

- текущая локация и anchor;
- качественные телесные ощущения;
- замеченные объекты и их наблюдаемые свойства;
- доступные affordances;
- слышимые/видимые/ощущаемые события с уверенностью;
- текущие занятия и занятые ресурсы;
- обзор уведомлений без скрытого текста;
- значимые изменения после предыдущего perception;
- `world_version` и `perception_id`.

Скрытые свойства не сериализуются. Административный snapshot использует отдельную схему и никогда не входит в Brain prompt.

## 8. Стабильный набор команд Brain

Нормативные пространства имён:

```text
world.move_to
world.perform
world.inspect
planning.manage
planning.wait_until
phone.execute
```

`world.perform` принимает `action_id`, `target_id` и типизированные параметры из action registry. Добавление нового предмета или рецепта не меняет tool schema.

В protocol V1 значения `PerformAction.parameters` передаются строками и проверяются по `parameters_schema_json`. Числа кодируются десятичной строкой; схема обязана объявлять `type: string` и точный `pattern`, чтобы Brain не отправлял несовместимый JSON number.

Stage 4B.3 регистрирует `object.clean` и `object.consume_quantity`. Наблюдаемые причинные состояния имеют явные целочисленные единицы: `charge_permille`, `cleanliness_permille`, `quantity_amount` + `quantity_unit`, `temperature_millicelsius`. Handshake объявляет capability `causal-object-condition-v1`.
Stage 4B.4 добавляет data-defined пассивную эволюцию. Событие `passive_conditions_advanced` хранит точный UTC-интервал, обновлённые состояния объектов и дробные остатки целочисленной интеграции. Границы завершения действий обрабатываются до смены power/open/placement, поэтому replay и разные частоты tick дают одинаковое причинное состояние.

Perception публикует `receiving_power` для chargeable-предметов. Handshake объявляет capability `passive-object-evolution-v1`.


Admin-команды находятся в отдельном сервисе и не публикуются Brain.

## 9. BrainDecision

Один decision cycle содержит:

- `decision_id`;
- triggering events;
- `perception_id` и `world_version`;
- версии identity/context/retrieval blocks;
- model profile и request fingerprint;
- выбранную команду;
- краткое структурированное explanation summary;
- расход токенов, cache hit и latency;
- итог `applied`, `discarded_stale`, `rejected` или `failed`.

Raw chain-of-thought не требуется и не хранится как explanation. Debug mode может временно сохранять технический request/response в защищённом хранилище.

## 10. While-thinking buffer

Пока Brain думает:

- World Engine продолжает timer/background events;
- новые события добавляются в ограниченный buffer;
- критическое событие помечает decision как invalidated;
- обычные события ждут следующего perception;
- по возврате ответа Brain сверяет `world_version`;
- устаревшая команда не адаптируется автоматически и не применяется частично.

## 11. Phone protocol

Входящее сообщение создаёт notification event. В perception попадают только разрешённые preview-данные. Полный текст выдаётся после валидного `phone.read_message`, затем отправляется Telegram read acknowledgement.

Исходящий ответ требует:

- прочитанного или явно выбранного контекста;
- privacy check;
- доступного телефона и сети;
- ресурсов внимания/рук/речи по формату;
- отдельного durable outbox event до отправки;
- идемпотентного reconciliation с Telegram после timeout.

## 12. Memory protocol

Memory ingest использует `subjective_event_id` как idempotency key. Retrieval-запрос содержит identity, текущую аудиторию, цели, темы, сущности, временную область и token budget.

Ошибки 400/422 означают ошибку контракта и не повторяются бесконечно. 429/5xx используют ограниченный exponential backoff с jitter. Невручённые события остаются в durable outbox.

## 13. Provider adapter protocol

Профиль модели объявляет:

- native tools;
- structured output/JSON schema;
- streaming;
- cache hints и метрики cache tokens;
- context/output limits;
- поддерживаемые роли и мультимодальность;
- правила интерпретации HTTP/status ошибок.

При отсутствии надёжного function calling используется строгий JSON envelope, локальная валидация и не более одной попытки исправления формата. Три неуспеха не создают бесконечный цикл: Brain переходит в контролируемую паузу.

## 14. Backpressure и лимиты

- Все очереди ограничены и имеют метрики заполнения.
- Критические технические события не вытесняются пользовательским спамом.
- Массовые уведомления агрегируются без автоматического чтения текста.
- Медленный subscriber восстанавливается по event log, а не удерживает writer thread.
- Oversized payload отклоняется до десериализации вложенных данных, насколько позволяет transport.

## 15. Эволюция схем

- Поля Protobuf не переиспользуются после удаления.
- Breaking change создаёт новый service/package version.
- Event types имеют собственную версию и upcaster chain.
- Старые события не переписываются.
- Unknown event блокирует replay с диагностикой.
- Handshake проверяет protocol range, identity, world definition hash и client capabilities.

## 16. Health и наблюдаемость

Каждый сервис предоставляет локальные `liveness`, `readiness` и version info. Метрики не содержат текстов сообщений или памяти.

Обязательные показатели:

- RPC latency и error rate;
- command deduplication;
- stale decision count;
- event lag и queue depth;
- DB commit latency;
- provider latency, tokens, cost и cache hit;
- memory retrieval hits/latency;
- backup/replay status.

