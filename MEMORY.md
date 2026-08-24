# Память и субъективная история Makise V1

Статус: нормативный Phase 0 design; runtime memory появляется по roadmap
Дата: 2026-08-19
Связанные документы: [CONTEXT.md](CONTEXT.md), [ARCHITECTURE.md](ARCHITECTURE.md), [PROTO.md](PROTO.md), [SECURITY.md](SECURITY.md), [INVARIANTS.md](INVARIANTS.md)

## 1. Назначение

Память хранит субъективную историю отдельного Consciousness: что было доступно восприятию, как это было интерпретировано, какие commitments были приняты и откуда получено знание. Она не является копией objective world state и не исправляет его.

Каждое Consciousness имеет собственные perception, cognition, memory и privacy streams. Несколько сознаний могут наблюдать один world event по-разному или не наблюдать его вовсе. Organism может существовать без подключённого Consciousness; его physical/biological history при этом продолжается в World Engine.

## 2. Границы authority

World Engine остаётся единственным автором authoritative physical, biological, neural, digital и institutional state. Внешний stimulus, LLM response и cognitive decision входят в причинную историю через `WorldEngine::commit`.

Memory service владеет только subjective records и derived retrieval indexes. Он не может:

- менять world, organism, neural или hormone state;
- создавать perception без соответствующей доступности;
- принимать goal, intention или commitment;
- превращать model interpretation в факт;
- назначать action outcome;
- скрыто редактировать уже записанную историю.

Objective event становится кандидатом subjective memory только после observer-specific projection. Память может содержать ошибочное убеждение, но обязана хранить source, confidence kind и связь с доступным evidence.

## 3. Causal flow

```text
Committed world transition
  -> observer-specific Projection
  -> perceived subjective event
  -> CortexFrame and retrieval
  -> CortexProposal
  -> CognitiveDisposition
  -> accepted interpretation or commitment event
  -> append-only memory ingest
```

`CortexProposal` может предложить memory interpretation. Только `CognitiveDisposition::Accepted` разрешает отдельную cognitive transition, после которой interpretation становится принятым belief или memory annotation. `Rejected`, `Deferred` и `NeedsRevision` сохраняются как decision evidence, но не меняют adopted memory state.

## 4. Subjective record contract

Каждая запись содержит как минимум:

- стабильные `subjective_event_id` и `consciousness_id`;
- ссылки на perception, cognitive disposition и доступные world transition IDs;
- canonical simulation time и время записи;
- modality/source и observer position;
- structured subject/predicate/object либо versioned content payload;
- provenance chain и artifact content digests;
- privacy owner, audience и citation rules;
- uncertainty model или определённую probability, если она нужна;
- schema version и append hash.

Текстовый summary является projection structured record. Он не заменяет source facts и не содержит скрытый chain-of-thought. Generic importance, valence, urgency или memory-strength scores не становятся authoritative state.

## 5. Working cognition

Рабочее состояние Consciousness хранит только принятые cognitive transitions:

- active goals и intentions;
- commitments и deadlines;
- unresolved questions и conversations;
- adopted plans и blocking conditions;
- reconsideration triggers для deferred proposals;
- identity-relevant beliefs с provenance.

Proposal не равен commitment. LLM transcript не равен working memory. Истёкший provider request, restart или повторная доставка не создаёт новое принятое решение.

## 6. Retrieval

Retrieval является observer- и audience-aware projection. Он может комбинировать exact identifiers, full-text search, embeddings, entity relations, canonical time, active commitments и source reliability. Результат возвращает причины выбора, provenance и uncertainty; пустой результат является нормальным outcome.

Retrieval не выдаёт:

- records другого Consciousness без явного права;
- скрытый objective state, которого наблюдатель не воспринимал;
- сведения вне privacy audience;
- debug/admin context как личное воспоминание;
- model interpretation как подтверждённый внешний факт.

Timeout или provider failure не заменяется выдуманной памятью. Решение без retrieval возможно только по явной policy и фиксируется в cognitive trace.

## 7. Consolidation, learning and forgetting

Raw subjective events и dispositions остаются append-only evidence. Consolidation создаёт новые derived records со ссылками на источники и mechanism/model digests. Она не переписывает прошлые записи.

Forgetting означает изменение доступности retrieval или uncertainty, а не скрытое удаление evidence. Любая будущая модель consolidation/decay обязана иметь `MechanismContract`, validity range, validation data и resolution-upgrade path. Phase 0 не задаёт численную psychology model; Phase 6 добавляет нейробиологическую связь.

## 8. Diary and self-reflection

Diary entry создаётся только принятым intention соответствующего Consciousness. Она:

- ссылается на реально доступные perceptions и memories;
- написана от субъективного лица и может ошибаться;
- является append-only;
- исправляется новой записью;
- не создаётся администратором или recovery process;
- не считается objective truth.

## 9. Privacy и изоляция нескольких сознаний

Privacy policy применяется до retrieval, context assembly и outgoing communication. Знание может влиять на внутреннее решение без права раскрыть его. Shared Organism или shared room не дают автоматический доступ к memory stream другого Consciousness.

При attachment/detachment Consciousness World Engine фиксирует objective event, а memory service сохраняет continuity собственного stream. Перенос Consciousness между organisms требует отдельного post-Phase design и не подразумевается обычным restart.

## 10. Persistence, replay and recovery

Subjective stream хранит causation links, schema versions и content digests. Fast recovery проверяет append chain и восстанавливает indexes из records. Audit связывает memory record с archived world projection, cognitive artifacts и dispositions.

Downtime не создаёт perceptions, interpretations, diary entries или commitments. Недоставленные уже committed subjective events остаются в durable outbox и ingest-ятся идемпотентно после recovery.

Snapshot ускоряет загрузку, но не является единственным источником истории. Corruption, missing artifact или broken causation link вызывает diagnostic `SafeStop` соответствующего consumer; система не синтезирует правдоподобную замену.

## 11. Migration

Legacy single-agent memory компилируется в отдельный archive bundle с явными `organism_id` и `consciousness_id`. Новая V1 использует отдельные streams и dual readers. Legacy records не переписываются in place; rollback не downcast-ит новые cognitive events.

## 12. Validation roadmap

Phase 0 проверяет schemas и cognitive fixtures. Phase 1 использует scripted cortex и accepted/rejected/deferred scenario. Phase 6 вводит causal consolidation, learning и decay mechanisms. Phase 7 проверяет privacy, relationships и несколько сознаний. Shadow launch с real LLM проверяет restart, downtime, provider failures и отсутствие прямой state mutation.
