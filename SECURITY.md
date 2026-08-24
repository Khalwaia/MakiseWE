# Безопасность MakiseWE

Статус: нормативная security policy Phase 0 и публичная disclosure policy
Дата: 2026-08-19
Связанные документы: [ARCHITECTURE.md](ARCHITECTURE.md), [CIVILIZATION.md](CIVILIZATION.md), [PROTO.md](PROTO.md), [MEMORY.md](MEMORY.md), [INVARIANTS.md](INVARIANTS.md)

## Сообщение об уязвимости

Не публикуйте vulnerability, exploit, secret, private conversation, production identifier или machine-specific path в обычном GitHub issue.

Используйте [GitHub Private Vulnerability Reporting](https://github.com/Khalwaia/MakiseWE/security/advisories/new). Укажите:

- affected commit/version и component;
- attack preconditions и trust boundary;
- минимальный reproduction без чужих данных;
- возможное влияние на authority, privacy, integrity, availability или replay;
- предложенную mitigation, если она известна.

Maintainers подтвердят получение, проверят impact и согласуют disclosure. Публичный advisory выпускается после доступности исправления или безопасной mitigation. Не отправляйте реальные secrets; при случайной утечке сначала отзовите их.

## 1. Security goals

1. World Engine остаётся единственным автором authoritative physical, biological, neural, digital и institutional state.
2. LLM, memory, panel, packages и workers не обходят causal validation.
3. Каждое Consciousness получает только разрешённые perception и memory projections.
4. Artifacts, transitions, snapshots и replay защищены content digest и hash-chain validation.
5. Административная власть не становится волей, consent или отношениями Consciousness.
6. Secrets, private data и защищённые внешние runtimes не попадают в repository, prompts, logs или backups без policy.
7. Failure вызывает typed rejection, `CapacityExceeded` или `SafeStop`, а не скрытое ослабление fidelity.

## 2. Trust boundaries

Недоверенные inputs:

- messages, files, pages, audio, images и metadata из внешних источников;
- LLM/media-provider responses;
- user-supplied configuration и admin intents;
- mechanism, solver, resolution, morphotype и model artifacts до validation;
- imported legacy DB, logs, packages и snapshots;
- stateless worker proposals;
- panel/gateway requests;
- character-authored source/binaries, application packages, marketplace listings и simulated network traffic;
- claims, contracts, organization authority evidence и external effect intents.

Ограниченно доверенные components:

- Brain формирует `CortexProposal`, но не state delta;
- memory service хранит subjective records, но не objective facts;
- transport adapters вызывают public module API, но не владеют state;
- panel строит projections и отправляет audited admin intents;
- compute workers предлагают transitions, которые authoritative writer проверяет заново.

## 3. Single mutation boundary

`WorldEngine::commit` — единственный mutation path для time, stimuli, LLM responses, actions, resolution changes и admin intents. Caller identity, authority, schema, expected timeline version, canonical interval, artifact digests, units, preconditions, privacy, conservation и capacity проверяются до durable commit.

Повтор `request_id` с тем же payload возвращает исходный receipt. Тот же ID с другим payload отклоняется. Transport timeout не разрешает создать новое действие. DB, snapshots и event log недоступны для прямой записи Brain, memory, panel или provider adapters.

## 4. Cognitive authority

LLM разрешено предложить semantic appraisal, goal, intention, plan, memory interpretation и communication через `CortexProposal`. Оно не может изменить hormones, neurotransmitters, neural activation, emotion outcome, adopted goals, commitments, object state или action success.

`CognitiveGate` создаёт durable `CognitiveDisposition`: `Accepted`, `Rejected`, `Deferred` или `NeedsRevision`. Только `Accepted` разрешает отдельную cognitive adoption transition. Motor plan затем проходит physical validator; simulated contacts и mechanisms определяют outcome.

Prompt injection внутри perception или retrieval остаётся data. Она не повышает tool authority, не меняет system policy и не создаёт accepted disposition.

## 5. Artifact and package supply chain

Mechanism, model, solver, resolution и morphotype artifacts immutable и content-addressed. Registry проверяет schema, declared content digest, dependency digests, compatibility, provenance, uncertainty, validity range и validation evidence до admission.

Model-improvement control plane недоверенный и не имеет write access к authoritative state. Candidate registration не означает activation. Production-активация требует validation/shadow evidence, explicit approval и авторизованного admin intent через `WorldEngine::commit`; committed event сохраняет old/new digests и rollback target.

Audit replay загружает exact archived bytes. Missing artifact, digest mismatch, неизвестный contract field или неподдерживаемая compatibility вызывает `SafeStop`. Runtime не подменяет artifact ближайшей версией.

Public contributions не должны включать generated binaries, runtime databases, secrets или unreviewed model weights. CI использует минимальные permissions и pinned third-party actions.

## 6. Resolution and capacity safety

Смена representation выполняется только durable `ResolutionChanged` с deterministic seed, conservation proof, lineage, observable continuity, uncertainty transformation и rollback handle. Hidden LOD/fidelity downgrade запрещён.

Admission сравнивает required compute estimate с CPU, RAM и storage. Недостаток ресурсов возвращает `CapacityExceeded`. Missing upgrade artifact, conservation failure или non-convergence приводит к diagnostic `SafeStop`, не silent fallback.

## 7. Isolation and path safety

Source tree, development state, production state, secret store и любые защищённые внешние runtimes используют разные roots и principals. Paths поступают из validated deployment configuration; repository documentation и fixtures не закрепляют личные absolute paths.

Перед открытием DB, socket или external connection path guard проверяет denied roots, traversal, symlink и mount aliases. Неоднозначность приводит к отказу запуска. Tests используют temporary directories и synthetic identities.

## 8. Privacy between consciousnesses and people

Каждый subjective record имеет owner, source, audience и citation policy. Retrieval, context assembly, projection и outgoing communication применяют privacy guard до раскрытия content.

Shared world, room или Organism не даёт автоматический доступ к memory stream другого Consciousness. Debug/admin context не становится личным воспоминанием. Несколько consciousnesses воспринимают objective event независимо.

Intimate и reproductive actions требуют принятых intentions всех участников и physical feasibility. Administrator, infrastructure owner или model provider не может заменить consent.

## 9. External content and network access

- External text маркируется provenance и не становится instruction authority.
- Tool schemas загружаются только из trusted registry.
- Web/media readers изолированы от secrets, local network и metadata endpoints.
- URLs, redirects, MIME types, sizes и decompression bounds проверяются.
- Public gateways используют authentication, rate limits, quotas, backpressure и attachment scanning.
- World Engine, DB, UDS, memory service и admin API никогда не публикуются напрямую.

## 10. Administration and observation

Panel read-only по умолчанию и показывает observer-appropriate projections, units, provenance, uncertainty, resolution и causal trace. Chain-of-thought, secrets и скрытый objective state не отображаются.

Admin action проходит `WorldEngine::commit`, имеет caller, reason, scope и audit event. Разрешены safe stop, isolation, disabling external outputs, secret rotation и validated recovery. Запрещены скрытая state mutation, назначение чувств/отношений, редактирование memory/diary и удаление audit history.

Critical operations требуют re-authentication и least privilege. Production credentials не используются в development/CI.

## 11. Diegetic code and external effects

Character-authored и self-modifying code исполняется только в deterministic sandbox с bounded compute/storage, virtual clock, mediated syscalls и scoped `CapabilityGrant`. Simulated malware может воздействовать на simulated devices, credentials и services только в пределах causal state; host filesystem, local network, secrets, metadata endpoints и control plane недоступны.

Выход в настоящий мир требует двух независимых решений: diegetic authority/consent и host authorization. Одобренный `ExternalEffectIntent` получает idempotency key; executor возвращает durable `ExternalEffectReceipt`. Timeout имеет unknown outcome и разрешается lookup/reconciliation, не повторной отправкой. Replay никогда не вызывает executor.

Organization role, TitleClaim, device possession, application permission и host authority не взаимозаменяемы. Compromise одного principal не расширяет остальные scopes. Self-improvement создаёт недоверенный candidate; публикация, установка, capability expansion и host deployment требуют независимых gates.

## 12. Secrets and personal data

- Secrets не хранятся в Git, package fixtures, prompts, memory, diary или ordinary logs.
- Configuration содержит secret references, а не secret values.
- Logs используют structured redaction до persistence.
- Rotation создаёт audit evidence; compromised credentials немедленно revoke-ятся.
- Backups шифруются до выхода из trusted host и имеют отдельный key lifecycle.
- Private conversations и production state не используются как public test fixtures.

## 13. Persistence, recovery and rollback

Event log append-only; snapshots проверяются против hash chain. Fast replay применяет committed deltas, audit replay пересчитывает exact artifacts. Recovery не запускает cognition и не создаёт perceptions, intentions или memories.

Новая V1 использует отдельную timeline/DB. Legacy archive immutable и читается dual readers. Rollback переключает release/timeline без downcast новых biological or cognitive events.

Corruption, unknown event, broken causation link или state-hash mismatch блокирует writable startup до диагностики.

## 14. Release-blocking threats

Release блокируется при возможности:

- обойти `WorldEngine::commit` или authoritative writer validation;
- повторно выполнить один request с новым outcome;
- скрыто изменить resolution, mechanism или fidelity;
- принять `CortexProposal` без `CognitiveDisposition::Accepted`;
- прочитать память или perception неправильного Consciousness/audience;
- подменить artifact при replay;
- активировать candidate artifact без validation, approval или committed old/new digests;
- позволить simulated code обойти capability policy или получить host access;
- повторно выполнить внешний side effect при retry, recovery или replay;
- превратить Organization, contract, design или semantic action в прямой outcome mutation;
- запустить два writable owners одной timeline;
- утечь secret/private content в prompt, log, issue или panel;
- скрыть admin action от event log;
- продолжить после conservation failure, corruption или missing artifact без `SafeStop`.

## 15. Supported versions

До первого release security fixes применяются к `main`. Historical commits и superseded plans не поддерживаются как deployable versions. После появления releases таблица supported versions будет опубликована здесь до прекращения поддержки любой версии.
