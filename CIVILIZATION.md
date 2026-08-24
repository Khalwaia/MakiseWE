# Причинные действия, техника и цивилизация Makise V1

Статус: нормативная целевая спецификация; реализация по фазовым gates
Дата: 2026-08-24
Связанные документы: [CONTEXT.md](CONTEXT.md), [ARCHITECTURE.md](ARCHITECTURE.md), [WORLD_V1.md](WORLD_V1.md), [INVARIANTS.md](INVARIANTS.md), [PROTO.md](PROTO.md), [SECURITY.md](SECURITY.md)

## 1. Область решения

Эта спецификация определяет общую причинную основу действий, техники, приложений, организаций, услуг, экономики, производства и строительства. Она не вводит отдельный «civilization engine»: все перечисленные явления принадлежат единому `CausalGraph`, используют общую authoritative timeline и подчиняются тем же правилам units, provenance, uncertainty, conservation, fidelity и replay.

Фраза «максимальный реализм» означает максимальную проверяемую причинную достоверность внутри объявленного `FidelityEnvelope`, а не постоянную симуляцию каждого атома, транзистора, нейрона или социального взаимодействия. Каждый Mechanism объявляет состояние, масштабы, error bounds, validity range, validation evidence и deterministic path к более точному Resolution. Выход за эту область требует refinement, typed failure либо `CapacityExceeded`; скрытая подмена упрощённым правдоподобным результатом запрещена.

## 2. Semantic outcome prohibition

Название действия выражает цель или observer classification, но не является механизмом изменения мира. `cook`, `clean`, `eat`, `sleep`, `build_house`, `deploy_service`, `install_application` и аналогичные операции не могут непосредственно создавать финальный state delta.

Причинная последовательность:

```text
perception + interoception + memory
  -> proposal
  -> CognitiveDisposition
  -> [Accepted only] Intention
  -> PlanHypothesis
  -> ControlEpisode
  -> следующий доступный affordance
  -> motor/digital/institutional request
  -> Mechanism proposal
  -> authoritative validation and commit
  -> изменённый state и новые observables
  -> продолжение, replanning, pause, failure либо termination
```

`Intention` не содержит physical, biological, neural, digital или institutional outcome. `PlanHypothesis` не фиксирует обязательный сценарий. `ControlEpisode` хранит текущее состояние управления, причины, constraints, acquired capabilities, наблюдаемый progress, blockers и следующую canonical reevaluation boundary. Он не содержит обещанный completion mutation; expected duration является прогнозом с uncertainty.

Фактический результат может быть успешным, частичным, ошибочным, побочным или отсутствовать. Прерывание питания, усталость, травма, потеря инструмента, изменение среды, отказ устройства, изменение цены или действие другого Consciousness становятся обычными входами replanning.

## 3. Действие как closed-loop causality

Физическая попытка проходит perception, neural control, muscle activation, articulated dynamics, contacts и material mechanisms. Принятое намерение «взять нож» не меняет possession: рука должна достичь рукоятки, создать контакт и достаточную силу трения; нож может выскользнуть, оказаться слишком горячим или быть убран другим участником.

Приготовление пищи является совокупностью процессов:

- locomotion и postural control;
- visual, tactile, proprioceptive и olfactory feedback;
- grasp, cutting, fracture и topology change;
- mass transfer между продуктом, инструментами, посудой и средой;
- conduction, convection, radiation, evaporation и phase change;
- chemical/structural transformations продукта;
- energy consumption, heat release, contamination и wear;
- subjective assessment готовности с возможной ошибкой.

«Блюдо готово» является observer-dependent projection состава, геометрии, температуры, water activity, pathogen load, reaction progress, texture и предпочтений, а не authoritative boolean.

Тот же принцип действует для сна, еды, уборки, лечения, обучения, программирования, оказания услуг и строительства.

## 4. Canonical scheduling и stochastic causality

Мир использует hybrid event/continuous scheduling. Каждый Mechanism объявляет допустимый temporal interval и error control: contacts и neural dynamics могут требовать коротких интервалов, digestion и logistics — более длинных, development и wear — существенно более длинных. Глобальный tick одинаковой длины для всех domains не требуется.

Детерминизм означает одинаковые transitions и state hash при одинаковых authoritative inputs, artifacts, resolution, interval и seed. Он не означает гарантированный успех или отсутствие стохастики. Стохастический Mechanism получает воспроизводимый random stream, адресованный world/timeline seed, mechanism, entity, canonical interval и cause. Thread count, wall-clock pacing и replay не меняют выборку.

## 5. DigitalDevice

Телефон, сервер, маршрутизатор, робот и промышленный controller являются `DigitalDevice`: единым физико-цифровым объектом, а не UI-инвентарём. Его authoritative state связывает:

- geometry, mass, materials, contacts, damage, temperature и contamination;
- battery chemistry, voltage/current, supplied energy и degradation;
- processors, memory, storage, buses и peripherals;
- firmware, boot state, operating system, filesystem и running processes;
- display, camera, microphone, speaker, IMU, location receivers и radios;
- credentials, accounts, permissions, network sessions и user data.

Instruction execution потребляет cycles и electrical energy, создаёт heat, contention, latency и wear. Упрощённое execution-to-energy отображение допустимо внутри `FidelityEnvelope`; overheating, storage wear, sensor noise, memory fault или radio interference могут причинно потребовать refinement.

Персонаж воспринимает приложение через физическую цепь: rendered pixels создают свет, retina выполняет transduction, neural mechanisms строят perception, attention выбирает content, а memory/appraisal могут сохранить либо исказить его. Приложение не может напрямую записать знание или воспоминание Consciousness.

## 6. Исполнение и создание приложений

Приложение исполняет фактический `CodeArtifact` в детерминированном sandbox runtime. Runtime получает virtual clock, bounded compute/storage, deterministic entropy и mediated syscalls. Прямой доступ к host filesystem, host clock, host network или authoritative world state запрещён.

Линия программного продукта разделяет:

- immutable source artifact;
- build recipe с toolchain и dependency digests;
- immutable binary artifact;
- signed application package/release;
- installation на конкретном DigitalDevice;
- running process с собственным machine state;
- mutable user data;
- credentials и capability grants.

Персонаж создаёт приложение через cognition, физическое взаимодействие с редактором, документацию, compilation, tests, observation ошибок и последующие изменения. Компетентность возникает из semantic/procedural memory, practice evidence, attention, fatigue и feedback; arbitrary `coding_skill` не назначает outcome.

Самоулучшение создаёт новый candidate artifact с parent digest, изменениями, build provenance, evaluation evidence и author/automation authority. Текущий release не переписывается. Candidate не публикуется, не устанавливается и не получает новые capabilities без соответствующих policies и approvals. JIT/self-modifying execution может менять sandbox memory, но не identity опубликованного release и не scope его authority.

## 7. Marketplace и цифровые услуги

Внутренний marketplace является реальной системой мира: organizations, compute devices, storage, accounts, signing keys, listings, releases, dependencies, licenses, prices, payments, reviews, moderation, search, recommendation, malware analysis, download traffic, entitlement и update channels.

Установка приложения требует network transfer, storage capacity, signature/dependency validation, compatibility и выдачи capabilities. Она может прерваться, повредиться, не пройти verification, столкнуться с недостатком места или энергии. Рейтинг является projection конкретных reviews, crash evidence, refunds, retention и версии ranking algorithm, а не intrinsic quality score.

SaaS или hosting service выполняет реальные simulated requests на simulated hardware. Requests расходуют compute, storage, bandwidth и energy; создают heat, latency, failures, operating costs и contractual consequences. При отказе cooling возможны throttling, data errors, shutdown, equipment damage и breach obligations.

## 8. Capability-mediated world access

CodeArtifact взаимодействует с миром только через `CapabilityGrant`. Grant связывает principal, artifact/install instance, конкретный ресурс или scope, разрешённые операции, срок, foreground/background conditions, privacy, rate limits, consent evidence и revocation.

Примеры capability scopes:

- capture camera frame или microphone sample;
- read observer-appropriate location;
- emit sound/display output;
- establish network connection;
- access selected contact or file;
- use credential for a constrained purpose;
- request payment authorization;
- control named lamp, vehicle lock, robot actuator или industrial device.

Grant разрешает запрос, но не гарантирует результат. Команда лампе проходит OS policy, protocol encoding, radio propagation, receiver validation, controller execution, circuit dynamics и light/heat emission. Потеря packet, interference, depleted battery или broken lamp дают иное наблюдаемое состояние.

## 9. External world gateway

Simulated network и настоящая host network разделены. Внешний вход становится причиной только как authenticated, timestamped и provenance-bearing observation, принятый через `WorldEngine::commit`.

Внешний выход использует две независимые authority checks:

1. diegetic permission внутри мира;
2. host authorization владельца инфраструктуры.

После них создаётся `ExternalEffectIntent`. Idempotent executor выполняет разрешённое воздействие и возвращает `ExternalEffectReceipt` с фактическим результатом. Replay применяет receipt и никогда повторно не отправляет реальное сообщение, платёж, заказ или команду устройству. Персонаж, Organization или вредоносное приложение не может самостоятельно расширить host capabilities.

## 10. Organization, contracts и экономика

`Organization` состоит из roles, membership, governance rules, authority delegations, assets, accounts, credentials, contracts и obligations. Она не является Consciousness и не получает собственную волю: решения происходят из принятых intentions конкретных Consciousness либо из CodeArtifact в пределах явно делегированной authority.

`ServiceOffering` выражает предложение выполнить работу при объявленных условиях. Его принятие создаёт `ServiceContract`: parties, scope, consideration, acceptance evidence, deadlines, privacy, liability, cancellation и remedies. Contract создаёт institutional obligations, но не физический результат.

Authoritative institutional state различает:

- physical `Possession`;
- `TitleClaim` и territorial claims;
- offers, orders, contracts и obligations;
- invoices, payments, debt и escrow;
- employment, lease, insurance и licenses;
- dispute, evidence, judgment и enforcement.

TitleClaim не создаёт физический барьер: theft и unauthorized occupation остаются возможны. Price не является свойством объекта; она возникает из offers, negotiation, scarcity, costs, risk, credit и beliefs участников.

Услуга развивается через реальные действия. Заказ доставки создаёт commitments и information flow; затем ресторан готовит физическую пищу, courier физически перемещает её, а recipient независимо воспринимает доставку. Taxi app не перемещает автомобиль, medical app не изменяет организм, dating app не создаёт отношения.

## 11. Производство и строительство

`DesignArtifact` описывает намеренную geometry, materials, tolerances, load paths, utilities и process plan. Он не является построенным объектом и может расходиться с фактическим состоянием из-за ошибок, замен материалов, деформации или неполного выполнения.

Строительство разбивается на causally constrained work:

- survey, measurements и geotechnical observations;
- design, analysis, revision и approval;
- finance, procurement и logistics;
- site preparation и earthworks;
- assembly, joining, curing и installation;
- electrical, water, sewage, ventilation и control networks;
- inspection, testing, defect discovery и rework;
- operation, maintenance, damage, repair и demolition.

Каждый work order требует физических материалов, инструментов, access, competence evidence, времени, энергии и исполнителя. Погода, fatigue, injury, shortage, theft, tolerance error и competing work могут изменить процесс. Mass, energy и material provenance сохраняются; импортированный initial asset обязан иметь declared provenance и uncertainty.

Дом является derived classification фактической конструкции. Habitability проецируется из structural integrity, weather protection, temperature, humidity, air quality, water, sanitation, electrical и fire safety. `build_house()` не может установить `house.complete = true`.

## 12. Датацентр как сквозной сценарий

Датацентр связывает institutional, digital и physical causality:

1. Consciousness или Organization формирует requirement и финансирование.
2. Выбираются участок, TitleClaim, permits и utility contracts.
3. DesignArtifact задаёт structure, floor loading, power, cooling, network и safety.
4. Materials и equipment производятся, приобретаются и доставляются.
5. Физически строятся building, transformers, switchgear, UPS, generators, cooling loops, fire suppression и racks.
6. Устанавливаются servers, storage, network links, firmware и orchestration artifacts.
7. ServiceOffering создаёт hosting obligations.
8. Customer requests фактически исполняются и потребляют compute, bandwidth и energy.
9. Heat, cooling, wear, maintenance, failures, billing и SLA evidence возвращаются в общий CausalGraph.

Diegetic compute capacity влияет на доступные сервисы внутри мира, но не выделяет реальные CPU/RAM хоста. Такое расширение требует отдельной host-authorized внешней операции.

## 13. Глубокие module seams

World Engine остаётся единственным authoritative writer. Внутри него используются небольшие стабильные interfaces:

- Mechanism получает causal snapshot/interval/artifacts и возвращает proposed transition либо typed failure;
- deterministic execution runtime получает machine state и mediated inputs, возвращает machine delta и syscall requests;
- capability policy принимает principal, operation, resource и evidence, возвращает grant decision без выполнения эффекта;
- institutional rules принимают claims, authority и evidence, возвращают proposed institutional transition;
- external gateway принимает approved intent и возвращает idempotent receipt;
- projection строит observer-appropriate physical, digital, institutional или subjective observables.

Эти seams не создают дополнительных mutation paths. Любой adapter предлагает данные или эффект; authoritative validation и durable commit остаются едиными.

## 14. Security, privacy и adverse behavior

Мир допускает malware, phishing, credential theft, vulnerable dependencies, privilege escalation, fraud, counterfeit releases, supply-chain attacks, censorship, moderation errors и forensic investigation. Outcomes следуют из фактического CodeArtifact, vulnerabilities, grants, credentials, topology и поведения участников, а не из `hacking_skill`.

Вредоносный simulated code остаётся внутри sandbox и не атакует host. Privacy проверяется на perception, storage, retrieval, communication и capability seams. Organization authority, device ownership или administrator role не заменяют consent Consciousness.

## 15. Acceptance evidence

Архитектура считается реализованной только после сквозных доказательств:

- приготовление пищи проходит movement, grasp, transformations, heat, perception и replanning без semantic outcome mutation;
- персонаж создаёт source, получает воспроизводимый binary, публикует release, устанавливает его и выполняет через capabilities;
- приложение управляет физическим устройством через network/permission/physics и наблюдает подтверждённый либо ошибочный результат;
- Organization оказывает услугу через Contract, реальные work transitions, payment и acceptance evidence;
- дом возникает из design, materials, logistics, labor и inspection, а не completion flag;
- датацентр связывает construction, power, cooling, code execution, service obligations и failures;
- restart, acceleration, worker-count variation и replay дают одинаковые canonical transition stream/state hash;
- внешний effect имеет ровно один receipt и не повторяется audit/fast replay;
- каждый realism claim ограничен `FidelityEnvelope` и validation evidence.
