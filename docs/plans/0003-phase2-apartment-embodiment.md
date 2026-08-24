# Implementation plan 0003: Phase 2 apartment and physical embodiment

Статус: план; выполнение начинается только после отдельного gate commit Phase 1
Дата: 2026-08-25
Scope: метрическое физическое воплощение в causal-kernel и data-driven apartment package

## 1. Результат

Phase 2 добавляет metric 3D geometry, materials, mass/inertia, articulated
bodies, active physics islands, fluids, atmosphere, heat/humidity/gases,
acoustics/light/odors, electricity и water как causal mechanisms поверх
существующего kernel seam. Anchors остаются semantic projection. Accepted
intentions запускают durable `ControlEpisode` с closed-loop motor programs;
`duration_ms`, completion mutation и resource/cleanliness/charge scores не
становятся authoritative state.

## 2. Non-goals

- everyday physiology (Phase 3), cells/immunity/drugs (Phase 4),
  reproduction/neuroscience/economy/construction (Phases 5–7);
- distributed authoritative state и performance optimization;
- LLM runtime integration и external side effects;
- in-place migration legacy DB или переписывание legacy fixtures.

## 3. Vertical slices

Каждый slice — отдельный reviewable commit с failing test через public seam
(`open`, `commit`, `project`, `events`).

1. **Metric geometry package.** Расширить world-package schema обратимым
   envelope: комнаты/объекты получают metric bounds `{value, unit}` вместо
   anchors-only. Legacy readers продолжают читать прежний manifest. Runtime не
   содержит branch по room/object ID.
2. **Rigid bodies и mass/inertia.** Authoritative body state: mass kg,
   center-of-mass m, inertia tensor component values с units, pose. Gravity и
   contact-free free fall — первый mechanism contract с exact conservation of
   momentum/energy на declared interval boundaries.
3. **Contacts и grasp.** Contact manifold как typed proposal output; grasp
   требует contact + friction force feasibility. Possession является projection
   фактического контакта, а не флагом.
4. **Articulated body / locomotion.** Human/Neko skeleton из morphotype anatomy
   graph получает joint limits и motor torque ports. `walk` — ControlEpisode:
   perception → balance feedback → torque proposals → physics validation →
   committed deltas; падение/спотыкание возможны.
5. **Active islands и scheduling.** Bodies группируются по contacts; острова
   исполняются deterministic reduction order; sleeping тела — explicit
   representation transition с recovery proof, без hidden LOD.
6. **Fluids и spill.** Water как amount-based particle/cohort representation с
   mass conservation; spill переносит amount между containers/floor по
   contact/precondition evidence.
7. **Atmosphere и heat.** Air volume per room, temperature/humidity/gases с
   units; conduction/convection связывает stove, воздух, предметы и organism
   thermal reservoirs через existing thermal port discipline.
8. **Acoustics/light/odors.** Point-source propagation с attenuation внутри
   declared validity range; sensory transduction Phase 1 продолжает получать
   physical fields вместо prescribed surrogates.
9. **Electricity/water infrastructure.** Power/water networks как unit-typed
   flow conservation mechanisms; device power draw становится physical delta.
10. **Cook/clean/dress episodes.** Semantic intentions запускают multi-step
    ControlEpisodes поверх перечисленных mechanisms; outcomes определяются
    simulation, включая partial results и interruption.

## 4. Acceptance scenario

Один scripted Human в `apartment-v2`: walk к кухне → взять кастрюлю → наполнить
водой → поставить на плиту → включить нагрев → непреднамеренный spill → clean →
dress. Каждая цепь обязана показать causes, artifact digests, canonical
interval, unit-typed deltas, uncertainty, conservation report и state hash.
Отрицательные тесты: grasp без контакта отклоняется; spill сохраняет total
water mass; отключение питания останавливает нагрев без promised outcome;
interruption ControlEpisode оставляет partial state без completion mutation;
restart/replay дают identical stream/hash.

## 5. Gates

- walk, grasp, carry, cook, spill, heat, clean, dress развиваются через durable
  closed-loop control, physics, conservation, feedback, interruption и replay;
- ни один semantic intent не содержит completion mutation или guaranteed
  duration; estimates хранятся отдельно с uncertainty;
- partition/restart/replay/worker parity наследует Phase 0–1 matrix;
- invalid units, preconditions, conservation и missing artifacts отклоняются до
  commit либо переводят timeline в `SafeStop`;
- coverage matrix обновляется фактическим evidence без realism claims вне
  declared validity ranges.
