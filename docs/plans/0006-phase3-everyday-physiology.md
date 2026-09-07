# Phase 3 — everyday physiology: план максимальной проверяемой реалистичности

Статус: implementation plan и evidence record Phase 3 после Phase 2 gate `220de6d`. Coarse реализации 3.1–3.2 существуют; их system gates ещё не закрыты. Разделы будущих механизмов описывают предложения, а не действующие contracts или доказанную физиологию.
Дата: 2026-09-05
Связанные документы: [INVARIANTS.md](../../INVARIANTS.md), [ARCHITECTURE.md](../../ARCHITECTURE.md), [WORLD_V1.md](../../WORLD_V1.md), [ROADMAP.md](../../ROADMAP.md), [CONTEXT.md](../../CONTEXT.md), [CIVILIZATION.md](../../CIVILIZATION.md), [ADR-0014](../adr/0014-fidelity-envelope-and-validation-evidence.md), [biology-realism.md](../research/biology-realism.md), [phase0-coverage-matrix.md](../coverage/phase0-coverage-matrix.md)

## 1. Что значит «максимальная реалистичность» в Phase 3

Максимум — это максимальная **проверяемая причинность внутри объявленного `FidelityEnvelope`**, а не максимум параметров. Каждый mechanism обязан нести `units`, `provenance`, `uncertainty`, `validity_range`, `conservation`, `failure_policy`, `validation_scenarios` и `resolution_upgrade_path` (`ARCHITECTURE.md:97`, `INVARIANTS.md:6-18,76`). Выход за envelope → `reject_transition` или `SafeStop`, не silent clamp (`INVARIANTS.md:18,42`).

Категории provenance и условия realism claim определяет [ADR-0014](../adr/0014-fidelity-envelope-and-validation-evidence.md): `measured`, `derived`, `expert_estimate`, `synthetic_fixture`. Peer review и calibration описывают evidence, но не вводят дополнительные категории этого ADR. Species proxy для Neko требует явного обоснования и uncertainty; он не становится measured physiology Neko. Calibration/validation split, sensitivity и propagation uncertainty описаны в [biology-realism.md](../research/biology-realism.md).

Поэтому максимум достигается не добавлением «ещё одного органа», а: 1) фиксированные compartments с dimensional check, 2) exact conservation `Δstore = inflow - outflow + production - consumption` на каждом canonical interval, 3) measured или peer_reviewed параметры где доступны, иначе узкий expert interval с явной маркировкой, 4) focused validation на независимых time-series, 5) upgrade path к fine resolution.

## 2. Порядок вертикальных слайсов (dependency graph)

Порядок из `ROADMAP.md:49` уточнён причинными зависимостями; каждый slice — отдельный `MechanismContract` с собственным gate.

| # | System | Зависит от | Почему этот порядок |
|---|---|---|---|
| 3.1 | **Cardiovascular / Respiratory** `cardiorespiratory.gas-exchange` | thermal core (есть) | Кровь переносит O2/CO2, тепло, субстраты, гормоны — требуется всем остальным. **Реализован coarse** `contracts/fixtures/mechanisms/cardiorespiratory-gas-exchange-v1.json` |
| 3.2 | **Renal / Fluids / Electrolytes / Water balance** `renal.fluid-electrolyte` | 3.1 (MAP, plasma volume, GFR) | Без объёма/осмолярности невозможен MAP, без GFR нет экскреции; блокирует edema/гидратацию |
| 3.3 | **Digestive / Liver / Metabolism (substrate)** `digestive.substrate` | 3.1+3.2 | Пища должна стать `glucose/AA/FA + Na + water` с absorption fluxes, liver `RQ`, не общий `chemical_store:uj` |
| 3.4 | **Endocrine (HPA/HPG/HPT, insulin/glucagon, ADH/RAAS)** `endocrine.signaling` | 3.1-3.3 | Сигналы модулируют renal, metabolic, thermo, muscle; вводятся после сосудов/субстратов чтобы иметь эффект |
| 3.5 | **Thermoregulation / Skin (active)** `thermoregulation.active` | 3.1+3.4 | Активные effectors: vasoconstriction/dilation (меняет `ambient_conductance`), sweating (evaporation), shivering/piloerection; сейчас passive `G = h·A` (`causal-kernel/src/morphotype.rs:60`) |
| 3.6 | **Musculoskeletal / Fatigue / Pain** `musculoskeletal.fatigue` | 3.1+3.4+3.5 | Work → mechanical power/efficiency, O2 debt, lactate proxy, peripheral/central fatigue, nociception; требует `physics_island` + `walk.rs` |
| 3.7 | **Excretion / Hygiene / Microbiome** `excretion.microbiome` | 3.2+3.3 | Кал/моча boundary fluxes, skin contamination, gut flora coarse `fraction` — последний чтобы не блокировать core |

Каждый slice следует [compatibility migration](../../PROTO.md#6-compatibility-migration). Конкретное расширение schema определяется после выбора public seam и проверки архивов; наличие default-колонки само по себе не доказывает совместимость hashes или replay.

## 3. Slice 3.1 — реализованный coarse (что есть и пределы)

**State** `causal-kernel/src/blood.rs`: `blood_volume_mm3:mm3`, `hb_tetramer_umol:umol`, `arterial_o2_umol:umol`, `venous_co2_umol:umol`, `map_mpa:mPa`, `lung_diffusion_umol_per_s:umol/s` (`BloodState`). O2 capacity = `hb*4`, saturation = `arterial*1000/cap` permille.

**Mechanism** `cardiorespiratory.gas-exchange`: `energy_per_umol_o2 = 470000 uj/umol` (Weir 1949, expert_estimate 10%), `RQ=0.85` (0.7–1.0), `lung_diffusion` human 300/neko 180 `umol/s`. За 1 s: `o2_demand = demand_uj /470k`, проверка `arterial < demand → OxygenOverdraft` без partial (`INVARIANTS.md:18`); `arterial-=demand`, `venous+=demand*RQ`, `replenish = min(deficit_to_98%, diffusion)` → saturation держится 980 permille в покое, `co2_exhaled = replenish*RQ`.

**Observables**: `arterial-saturation-permille` ±5 permille, `blood-volume-mm3` exact. **Conservation**: `o2+co2` ±1 umol, `total_energy` ±1 uj. **Gate тесты** `causal-kernel/tests/blood_gas.rs`: resting 950..1000 permille 60 s, overdraft reject, `1×60s == 60×1s` hash, restart parity, neko<human capacity.

**Реализм сейчас**: `expert_estimate`/`synthetic_fixture`. Нет Severinghaus `sat(PO2,T,pH)` кривой, нет `pCO2/pH/HCO3-` acid-base (Henderson-Hasselbalch), нет Dissolved O2 vs bound, нет `cardiac_output = stroke_volume × HR` с Frank-Starling, MAP константа не из `compliance·volume`. Допуск широк, horizon только rest…moderate activity. Это `INVARIANTS.md:76` — не заявляет realism вне envelope.

**Upgrade** `blood-coarse-v1 → blood-capillary-v1` `ResolutionContract`: fine добавляет arterial/venous/capillary/interstitial compartments с тем же stable port; lift сохраняет `total O2, total CO2, hb, volume` в `±1 umol`; observable continuity ±5 permille.

## 4. Слайсы 3.2–3.7 — максимальная реалистичность в каждом

Предложение для следующих слайсов: compartments с объёмом и количествами веществ, концентрация как projection. Общая abstraction вводится только при доказанной потребности выбранного среза; `compartment.rs` сейчас не является реализованной инфраструктурой. Canonical scheduling и reduction order должны быть объявлены в соответствующем contract.

### 3.2 Renal / Fluids / Electrolytes

**Реализован coarse slice:** `RenalState` хранит total body water, plasma volume/Na и urine boundary в `mm3`/`umol`; `CommitRequest::ingest_fluid` проходит через `WorldEngine::commit` и меняет MAP через blood port. [renal.rs](../../causal-kernel/tests/renal.rs) проверяет synthetic 24 h balance, bolus MAP, invalid input, restart и partition parity состояния. [renal_replay.rs](../../causal-kernel/tests/renal_replay.rs) проверяет fast/audit replay одного и последовательных intake commits без продвижения времени, reopen, retry/conflict и durable `SafeStop` при повреждении artifact или audit evidence. Envelope остаётся resting baseline adult; nephron/hormonal model отсутствует.

**Исправление replay последовательных intake commits:** fast replay применяет sparse deltas к предшествующему состоянию, сохраняя неизменённые quantities и проверяя units/`before`; audit replay пересчитывает intake из предыдущего проверенного состояния и использует его water balance как opening conservation. Независимый anchor: два приёма по 250 ml и 35 mmol дают 42 500 000 mm³ total-body water и 490 000 µmol plasma Na. Regression сначала падал с `CorruptTransitionEvidence`, затем проходит. Schema, archived bytes и committed events не переписываются; rollback возвращает предыдущий reader, сохраняя DB.

**Открытый gate:** цепь `ingest_fluid → advance_to → ingest_fluid` ещё не покрыта fast/audit replay: advance transitions не имеют полного archived mechanism execution. Имеющийся 24 h тест доказывает состояние после partition/reopen, но не равенство fast/audit replay всей физиологии. Не хватает renal worker parity, sodium conservation report в durable intake evidence, executable resolution upgrade и независимой эмпирической validation. Fixture задаёт upgrade digest как пример, а не готовый nephron artifact.

Renal audit проверяет digest архивных bytes, совпадение artifact references в event/execution и точное совпадение bytes с поддерживаемым fixture. Изменённый artifact с тем же ID и корректно пересчитанным digest вызывает durable `SafeStop`; два regression tests подтверждены red/green. Пересчёт пока вызывает встроенный Rust mechanism: guard ограничивает совместимость, но не заменяет полный admission contract и versioned executable ABI.

**Предлагаемое расширение state**: `total_body_water_mm3`, `plasma_mm3 / interstitial_mm3 / intracellular_mm3`, количества Na/K в `mmol`, `GFR_mm3_per_s`. Osmolality в `mOsm/kg` — физическая величина, не dimensionless fraction; соответствующий unit contract требуется до реализации.

**Предлагаемый mechanism**: intake water/Na из digestive ports, Starling distribution, filtration по net filtration pressure и `Kf` с согласованными units, ADH/aldosterone-mediated reabsorption, urine water/Na/urea boundary. Уравнения, electroneutrality и conservation tolerances требуют отдельного acceptance contract. Названия учебников и общие reference ranges пока не являются measured parameter ledger. Текущий synthetic bolus test ожидает MAP response 3–5 kPa на +500 ml; это coarse fixture assumption, не клиническая validation. Fine nephron segments остаются planned upgrade.

### 3.3 Digestive / Liver / Metabolism

**State**: `stomach_glucose_mmol`, `gut_glucose_mmol`, `liver_glycogen_mmol`, `plasma_glucose_mmol`, `urea_mmol`, fecal `dry_mass_mg`. **Mechanism** `digestive.substrate`: `ingest → stomach → gut absorption` Michaelis-Menten `Vmax/Km` per macronutrient, liver gluconeogenesis/glycogenolysis, `RQ` динамический (carb 1.0, fat 0.7), nitrogen → `urea`. **Conservation**: `mass:mg` ±1 mg, `glucose:mmol` ±0.01, `energy:uj` Weir `VO2/VCO2` coupling. **Provenance**: absorption `Vmax` calibrated на mixed-meal time-series (peer_reviewed), Neko obligate carnivore — species_proxy. **Validation**: postprandial glucose 4–8 mmol/L, absorption 3–5 h, fecal mass conservation. **Upgrade**: gut lumen segments + microbiome interface.

### 3.4 Endocrine

**State**: `cortisol_nmol_per_L`, `insulin_pmol_per_L`, `glucagon`, `ADH`, `T3/T4` — concentrations `nmol/L`. **Mechanism** `endocrine.signaling`: HPA/HPG/HPT с pulsatile release, half-life, receptor `occupancy:fraction` (Hill eq), эффекты — `GFR`, `vasoconstriction`, `metabolic_rate`, `glycogenolysis`. **Provenance**: half-lives/affinities measured (peer_reviewed), circadian cortisol peak calibrated. **Validation**: cortisol circadian amplitude, insulin response to glucose bolus ±15%. **Upgrade**: fine receptor populations.

### 3.5 Thermoregulation active

**State**: `skin_temperature_mk`, `core_temperature_mk` раздельно (сейчас только core `organism.rs:187`), `sweat_rate_mg_per_s`, `skin_blood_flow_mm3_per_s`. **Mechanism** `thermoregulation.active`: controller `error = core - setpoint(T_circadian)`; effectors — `skin_flow` (меняет `ambient_conductance = h·A` `RANGE 5–25 Incropera`), `sweat = f(error,T_skin, osmolality)` evaporation `2260 J/g`, `shivering = f(error)` + `metabolic demand`. **Provenance**: setpoint 37 °C circadian ±0.5 °C Mackowiak 1992 measured, Stolwijk 25-node model peer_reviewed, Neko fur `thermal_conductance` species_proxy 3.2. **Validation**: step ambient 20→10 °C, core drift <1 K, sweat onset >36.8 °C. **Upgrade**: multi-segment body nodes (Stolwijk).

### 3.6 Musculoskeletal / Fatigue / Pain

**State**: `muscle_ATP_mmol`, `phosphocreatine`, `lactate_mmol`, `peripheral_fatigue:fraction`, `central_fatigue:fraction`, `pain:receptor_occupancy`. **Mechanism** `musculoskeletal.fatigue`: `mechanical_power = force·velocity / efficiency` (eff 0.20–0.25), ATP hydrolysis → `O2` demand via 3.1, `lactate = f(O2 deficit)`, fatigue Hill-type. **Provenance**: efficiency 0.22 calibrated ergometry peer_reviewed. **Validation**: `VO2` линейно с power до 75% VO2max, time-to-exhaustion при заданной power ±15%. **Upgrade**: motor-unit populations.

### 3.7 Excretion / Microbiome

**State**: `fecal_water_mm3`, `fecal_microbe_count:counter`, `skin_contamination:mg`. **Mechanism** `excretion.microbiome`: gut `microbe:counter` logistic growth, `SCFA:umol`, hygiene `cleaning` как `CIVILIZATION.md:54` — только physical transfer, не `clean=true`. **Provenance**: microbe counts calibrated 16S peer_reviewed. **Validation**: fecal water 60–85%, microbe density ±1 log. **Upgrade**: individual microbe lineages `CellCohort` analogy Phase 4.

## 5. Поперечные требования максимальной реалистичности

- **Units & conservation**: каждый flux зарегистрирован на обоих концах `Δstore = in - out + prod - cons` с tolerance из solver precision, не fixed `1e-15 kg` (`biology-realism.md:81`).
- **Provenance ledger**: каждый `parameter_id` хранит `DOI/table/figure/population/method/conversion/CI/license` (`biology-realism.md:122`).
- **Calibration vs validation split**: отдельные 24 h traces; cross-mechanism checks (`VO2` vs heat vs water) (`biology-realism.md:136`).
- **Uncertainty propagation**: sensitivity + Monte-Carlo через declared `uncertainty_model`; Neko широкие предсказания (`biology-realism.md:126`).
- **Advance warning**: сохранение прежней трассы, сравнение `coarse vs fine` observables после одинакового stimulus в течение horizon, не только lift totals (`biology-realism.md:127`).
- **Determinism**: один `canonical_scheduling` для 1:1/acceleration/restart/replay, 1 и 16 workers идентичны, wall clock не меняет semantics (`INVARIANTS.md:32-34`).
- **Capacity**: schema без cap, admission по `CPU/RAM` → `CapacityExceeded` без silent downgrade (`INVARIANTS.md:40`).

## 6. Взаимодействие с Phase 2 physics

Airflow/heat через `atmosphere.rs` `RoomAtmosphere`, fluids через `LiquidContainer` `→` water intake, contacts/impulse → muscle work. Термо-контроль должен менять `ambient_conductance` и вызывать `heater_energy_uj`/`sweat evaporation`, а не prescribed `perceived_temperature`.

## 7. Gate каждого system и общий Phase 3 gate

System gates определяет [ROADMAP.md](../../ROADMAP.md#phase-3--everyday-physiology); causal acceptance matrix — [AGENTS.md](../../AGENTS.md#tdd-и-доказательства-приёмки). Для каждого среза нужны contract, reference observables, upgrade path и focused validation. Текущие coarse tests не закрывают эти gates автоматически. 365-day integration и workstation capacity относятся к release/Phase 8 gates в roadmap; этот план не переносит их в Phase 3 и не меняет фазовую последовательность.

## 8. Не-цели и rollback

Не включает: cell division/immunity (Phase 4), genetics/pregnancy (Phase 5), neural plasticity (Phase 6), devices/institutions (Phase 7). Rollback следует [PROTO.md](../../PROTO.md#6-compatibility-migration): отдельная timeline, сохранённые readers и archived bytes, без downcast новых events. Совместимость старых hashes доказывается fixtures/tests, а не наличием default-значений новых колонок.

## 9. Следующий acceptance slice

### Расширение scope 3.2 от 2026-09-07

Пользователь согласовал включение executable renal resolution upgrade в 3.2. Это снимает прежнее исключение fine renal representation, но не разрешает hormones, pathology или переход к 3.3. Upgrade должен пройти существующий `commit` и сохранять water/Na amounts, lineage, observable continuity и обратимую projection; смена одного `resolution_id` без изменения представления не является upgrade evidence. Biological realism требует отдельного независимого reference по ADR-0014; synthetic parity его не заменяет.

### Обязательная эмпирическая приёмка 3.2

Решением пользователя от 2026-09-07 закрытие 3.2 требует независимых эмпирических time-series. Synthetic/expert-estimate integration gate недостаточен. Это уточнение acceptance, а не утверждение о наличии подходящих данных или готовности текущего coarse mechanism.

До калибровки фиксируются question of interest, популяция, route и длительность fluid/sodium input, исходная hydration, поза/activity, временные точки и измеряемые endpoints. Oral intake не приравнивается к мгновенному plasma bolus: absorption либо входит в проверенный input port, либо эксперимент считается несовместимым. Независимый validation dataset отделяется от calibration dataset; выбор модели, параметров и tolerance по validation output запрещён. После настройки по validation набор перестаёт быть независимым и требуется новый holdout.

Evidence ledger содержит DOI/URL, table/figure/supplement, метод измерения, размер выборки, исходные units, преобразования, uncertainty (с различением SD, SEM и CI), missing data, условия использования и digest точных локальных data bytes. Digitized points допускаются только с отдельной extraction uncertainty. Порог ошибки выводится из measurement uncertainty и заранее объявленного model error; он не расширяется после неудачного сравнения. Отсутствующий uncertainty или закрытые данные остаются явным gap, а не заменяются придуманной таблицей.

Public-seam validation сравнивает временной профиль renal water/Na outputs и доступные blood/plasma observables с независимым reference внутри узкого resting envelope. Conservation, negative envelope cases, coarse/fine dynamic continuity и полная replay matrix обязательны дополнительно. Поддержка одного water endpoint не доказывает sodium kinetics, MAP, скрытые nephron states или physiology другого morphotype. Для Neko возможны только явно обозначенные component proxies; human evidence не становится measured Neko physiology.

При отсутствии совместимых независимых данных либо при необходимости endocrine/pathology mechanisms system gate остаётся открытым; меняется acceptance contract отдельным решением, а не маскируется результат. Копии прежних synthetic artifacts и их executable semantics сохраняются для replay; эмпирически обоснованная модель получает новые digests и проходит обычную admission/activation boundary.

Первичный [evidence ledger](../research/renal-validation-evidence.md) содержит water/saline experiments и открытую Table 2 Pedersen 2010 с exact extraction digest. Ни один dataset ещё не принят как validation: oral absorption и исходная hydration не представлены текущим plasma-intake port, а sodium units в Jensen 2013 требуют независимой проверки. Пользователь отклонил включение oral-water absorption и hormonal water-retention связи в 3.2: проверять почки отдельно. Эти механизмы не реализуются в данном scope. Нужны renal-only measurements с известными boundary inputs; oral loading не подменяет их, legacy `ingest_fluid` semantics не меняются.

Первый runtime prerequisite — admission dependency artifacts через существующие `MechanismContract::from_json` и `ArtifactBundle::admit`. Наблюдаемый результат: неполный contract и неизвестные executable fields отклоняются до storage. Acceptance: полный thermal reference продолжает admission/commit/replay, удаление обязательного поля или неизвестное поле не допускается. Это также применяется к новым physiology ABI перед их подключением к canonical executor. Старые archived bytes не переписываются; reader совместимости сохраняется. Rollback возвращает executable при сохранённых DB и artifacts. Для полной проверки contract используется уже присутствующая в workspace библиотека `jsonschema`, без сетевых resolvers; стандартная библиотека и `serde_json` не реализуют JSON Schema validation.

Admission prerequisite реализован в [physiology_admission.rs](../../causal-kernel/tests/physiology_admission.rs): полный schema validator применяется при `admit`, структурное чтение прежних contracts остаётся доступным. RED/GREEN подтверждён для отсутствующего unitful state, неизвестного executable field и Unicode digest, ранее вызывавшего panic. Проверяются все required fields, nested errors и отклонённый `commit` без event/receipt после reopen. `ProgramAbi` теперь проверяет полный набор thermal executable fields и положительную integer conductance. [thermal_replay.rs](../../causal-kernel/tests/thermal_replay.rs) подтверждает отдельный RED/GREEN: audit больше не игнорирует неизвестный field даже с корректным digest, сохраняет `SafeStop` после reopen и не меняет delta replay. Это structural admission prerequisite, не новые physiology ABI, не semantic validation всех dependency contracts и не закрытие system gate.

Проверка prerequisite 2026-09-07: `cargo test --workspace --all-targets`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all -- --check` и `git diff --check` проходят. `graphify update .` выполнен. Markdown gate согласован с public-repository gate: generated и игнорируемый `graphify-out` не относится к публичным source documents. Исходный workspace run выявил именно эту несовместимость checker с generated percent-encoded wiki links; после исправления полный повторный run проходит.

Закрыть replay цепи `ingest_fluid → advance_to → ingest_fluid` через существующие `WorldEngine::commit`, `events`, `fast_replay`, `audit_replay`. До runtime-изменений определить архивируемые artifacts всех задействованных mechanisms и совместимый replay contract. Acceptance: одинаковые canonical transitions/hash при partition, reopen и 1/N workers; retry/conflict; typed rejection или durable `SafeStop` без partial commit. Non-goals: новые органы, hormones, fine nephron solver. Slice 3.3 начинается отдельной работой после устранения обязательных пробелов 3.2.

### Проверка предпосылок следующего среза

Текущий `commit` сохраняет одну aggregate transition на запрос продвижения, а `timeline_version` одновременно используется как event sequence. Тест `committed_intervals_are_durable_and_readable_after_reopen` в [audit_replay.rs](../../causal-kernel/tests/audit_replay.rs) закрепляет эту совместимость: запросы 3 s и 2 s дают ровно две записи. Промежуточные deltas и execution artifacts для этих интервалов не сохранены. Проверка [worker_invariance.rs](../../causal-kernel/tests/worker_invariance.rs) сравнивает только часы независимых engines, а не 1/N workers одной timeline; она не закрывает требуемое evidence.

До изменения формата выполняется узкий prerequisite: renal audit обязан отклонять неизвестные exact bytes даже при совпадающем mechanism ID и корректно пересчитанном digest. Public seams: `commit`, `events`, `fast_replay`, `audit_replay`, `safe_stop`, `open`. Acceptance: исходный artifact продолжает replay; изменённые bytes вызывают durable `SafeStop`, без изменения organism, событий и receipt. Это проверка совместимости существующего встроенного executor, а не admission нового mechanism. Non-goals: новая физиология, изменение archived bytes, новый формат timeline. Rollback возвращает предыдущий executable; DB, receipts и artifacts сохраняются.

Prerequisite реализован в [renal_replay.rs](../../causal-kernel/tests/renal_replay.rs): неизвестный same-ID artifact и несовпадение event/execution references отклоняются, `SafeStop` сохраняется после reopen. Полный смешанный replay ещё не реализован. Предложение совместимого формата, inventory executable dependencies, conservation и acceptance anchors зафиксированы в [ADR-0016](../adr/0016-canonical-physiology-replay-format.md). Изменение формата принято в [ADR-0016](../adr/0016-canonical-physiology-replay-format.md). Runtime срез начинается с explicit format admission; canonical executor остаётся отдельным следующим этапом.

### Expand: проверка формата при открытии

В Phase 3.2 реализована граница `OpenSpec::with_format` / `WorldEngine::open`: новые aggregate timelines сохраняют `timeline_format=aggregate-v1`; metadata прежнего schema без этого поля читается без добавления колонки. Несовместимое требование возвращает `FormatMismatch`, неизвестный durable format — `IncompatibleStorage`, создание ещё не реализованного `CanonicalPhysiologyV2` — `UnsupportedTimelineFormat`. Отказ происходит до recovery writes и не оставляет aggregate DB вместо canonical.

[Public tests](../../causal-kernel/tests/timeline_format.rs) проверяют сохранность bytes, events, projection и retry receipt после отказа, старую metadata без колонки формата, неизвестный формат и отсутствие побочного создания DB. RED подтверждён для игнорирования требования, создания неподдерживаемого формата и чтения старой metadata; focused suite проходит. Это expand prerequisite: canonical executor, исправление sodium units, event ranges и mixed fast/audit replay ещё не реализованы. Rollback возвращает предыдущий executable; aggregate DB остаётся читаемой им, новых canonical records этот срез не создаёт.

### Исправление units renal sodium

Дополнительный replay regression от 2026-09-08: отсутствующий, отрицательный или неизвестный field в archived renal input отклоняется с durable `SafeStop`, сохраняя organism и fast replay после reopen. RED подтвердил ошибку без остановки для `{}`; GREEN проверяет все три случая через public API. Это исправление failure policy, без изменения физиологических coefficients или archive bytes.

Следующий prerequisite ADR-0016 выполнен через существующие `commit`, `events`, `fast_replay`, `audit_replay`, `open` и `safe_stop`. Writer записывает plasma/urine sodium в `umol`; значения, physiological coefficients, state hash algorithm и archived mechanism bytes не менялись. Независимый anchor: intake 35 000 µmol поверх baseline 420 000 µmol, затем 60 s excretion дают plasma 454 940 µmol и urine 60 µmol.

[renal_units.rs](../../causal-kernel/tests/renal_units.rs) сначала падал на `centi_umol` вместо `umol`; после исправления проверяет intake/excretion events, reopen и retry. [Compatibility fixture](../../causal-kernel/tests/fixtures/renal-sodium-legacy-deltas.json) фиксирует прежние sodium deltas. Отдельный RED показал отсутствие явной dimensional diagnostic; теперь оба replay возвращают `IncompatibleQuantityUnit`, audit дополнительно сохраняет `SafeStop` после reopen. Старые events сохраняют исходные units, snapshot projection и retry receipt читаются; read/fast rejection не меняют bytes DB, audit diagnostic не меняет organism или events.

Срез не реализует canonical executor, sodium conservation report или mixed intake/advance replay и не закрывает system gate. Формат остаётся aggregate-v1. Rollback возвращает предыдущий executable с сохранённой DB: events и snapshots читаемы, но прежний replay reader может отклонить новые `umol` records; downcast и переписывание units запрещены. Следующий runtime этап — versioned deterministic executable ABI и dependency contracts из ADR-0016.

### Исполняемая renal dependency: граница следующего этапа

Public seam: `ArtifactBundle::admit` и получение renal proposal из archived program bytes. Новый `renal-fluid-v1` ABI задаёт resting bounds, baseline amounts и corrective water/Na rates явными integer parameters с units в contract. Proposal возвращает новое `RenalState`, не мутирует timeline. Наблюдаемый anchor: из 42 250 000 mm³ body water и 455 000 µmol plasma Na один шаг при 17 mm³/s и 1 µmol/s даёт 42 249 983 mm³ и 454 999 µmol, urine получает 17 mm³ и 1 µmol. Изменение архивной скорости должно менять proposal; unknown fields, неверные ranges, overflow и неполный contract отклоняются. Legacy intake executor и его exact fixture остаются без изменения. Этот этап не активирует новую dependency, не создаёт canonical timeline и не является empirical validation; rollback удаляет неактивный новый ABI при сохранённых legacy readers.

Результат этапа renal dependency: [contract](../../contracts/fixtures/mechanisms/renal-corrective-excretion-v1.json), [program](../../contracts/fixtures/mechanisms/renal-corrective-excretion-v1.program.json) и [public tests](../../causal-kernel/tests/renal_program.rs). Admission связывает complete bounded reference contract с executable ABI и exact program digest; допускает rates только в объявленных интервалах при совпадении contract parameter values. RED/GREEN: отсутствующий ABI, чужой mechanism contract и расширенные bounds. Проверены unknown fields, rate bounds, overflow без candidate, baseline no-op и независимые one-second anchors. Новый ABI пока не подключён к timeline. Upgrade digest в fixture остаётся прежним planning placeholder; это не upgrade artifact и не activation evidence. Full-state bounds закреплены ABI v1 и не являются runtime-настраиваемыми параметрами.

### Дополнительная laboratory validation: решение 2026-09-08

Пользователь разрешил отдельную ex-vivo проверку по человеческим почкам на лабораторном аппарате как дополнительное evidence. Это не заменяет независимую эмпирическую проверку здорового resting организма и не разрешает oral absorption или endocrine coupling. Первичный кандидат — Weissenbacher et al. (DOI 10.1111/ajt.14932). До реализации ex-vivo модели нужны exact input protocol, пригодные numeric water/Na time-series, uncertainty и независимый calibration/holdout split. Если опубликованных данных недостаточно, подготавливается запрос автору; отправка требует отдельного разрешения. Текущий synthetic renal ABI не подгоняется под aggregate laboratory outputs.
