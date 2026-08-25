# Phase 2 Slice Status — causal-kernel apartment physics

## Status: ACCEPTANCE COMPLETE — все slices и `apartment-v2` scenario доказаны; остался отдельный gate commit перед Phase 3

План: [0003-phase2-apartment-embodiment.md](0003-phase2-apartment-embodiment.md).
Evidence-детали каждого slice — в [coverage matrix](../coverage/phase0-coverage-matrix.md).

## Что реализовано и доказано тестами

| Slice (план 0003) | Commit | Evidence |
|---|---|---|
| 1. Metric geometry package | `8d877e5` | `world/tests/metric_geometry.rs` |
| 2. Rigid bodies, mass/inertia, energy | `24a9be6`, `7d22c4e` | `tests/rigid_body.rs` |
| ADR-0014 fidelity envelope | `92c4264` | принятый ADR |
| 3. Contacts и grasp cone | `409cfba` | `tests/contact.rs` |
| 4a. Articulated skeleton | `00f1121` | `tests/articulation.rs`, `tests/anatomy_binding.rs` |
| 5. Active islands и scheduling | `47ca186` | `tests/physics_island.rs` |
| Collision response с momentum conservation | `f381d14` | `tests/collision.rs` |
| Coulomb friction (stick/slide) | `df6d0ec` | `tests/friction.rs` |
| Island rest trigger через support | `eb26221` | `tests/island_rest.rs` |
| Bipedal balance feedback | `1afb51e` | `tests/balance.rs` |
| 4b. Durable walk `ControlEpisode` | `4c4cb7c` | `tests/walk_control.rs` |
| 6a. Fluid statics (measured ρ) | `ca581fa` | `tests/fluids.rs` |
| 6b. Pour/spill accounting | `1b02512` | `tests/liquid_pour.rs` |
| 7a. Room atmosphere Compartment | `07c0ef7` | `tests/atmosphere.rs` |
| 8. Point-source acoustics/light/odors | `d7805db` | `tests/propagation.rs` |
| 9. Electricity/water networks | `30a8f34` | `tests/infrastructure.rs` |
| 10. Cook/clean/dress episodes | `047ad2f` | `tests/episodes.rs` |
| + Bodies persisted через `WorldEngine::commit` | `eb7aabf` | `tests/body_persistence.rs` |
| + `apartment-v2` acceptance scenario | `0813af8` | `tests/apartment_v2.rs` |

Новые механизмы сессии slices 9–14 наследуют дисциплину предыдущих: exact
integer arithmetic, typed rejections вместо clamps, pure functions от входов,
hand-derived учебниковые якоря независимо от production code.

## Критерии gate из плана 0003 §5

| Criterion | Status | Gap |
|---|---|---|
| walk через durable closed-loop control | ✅ в kernel | `WalkControlEpisode`: balance feedback, blockers, replanning, observed completion |
| grasp через contact + friction feasibility | ✅ | friction cone + hold projection; carry pending |
| cook/clean/dress episodes | ✅ в kernel | `ControlEpisode`s поверх heat/power/water/spill; duration возникает из физики (10 секунд нагрева), interruption/partial/failure первичны |
| spill с сохранением объёма | ✅ | pour/spill conservation bit-exact; liquid↔vapour мост в atmosphere; puddle-on-floor связка с contacts pending |
| Bodies persisted через `WorldEngine::commit` | ✅ | `place_body` upsert через единственный mutation path; restart bit-exact; retry/conflict/stale-version typed; corruption typed на чтении |
| Partition/restart/replay parity | ✅ доказано | `apartment-v2`: identical event stream, bit-exact body restore, identical state hash и idempotent replay старых request id после reopen |
| Invalid units/preconditions отклоняются | ✅ | typed failures во всех новых модулях |
| Coverage matrix с фактическим evidence | ✅ | обновляется каждым slice |

## Честная оценка

Kernel покрывает physical embodiment ядро: metric rigid bodies с точной
консервацией, контакты с трением, острова с физическим rest trigger, баланс,
walk как durable closed-loop episode, жидкостный учёт со spill, room
atmosphere с measured суховоздушной теплоёмкостью, конвекцией через shared
thermal port (pot/organism coupling доказан тем же `ThermalProposal`), точным
liquid↔vapour массовым мостом, point-source полями с declared затуханием,
electricity/water сетями (отключение питания останавливает нагрев typed
rejection'ом без promised outcome) и cook/clean/dress `ControlEpisode`ми,
где duration возникает из физики, а interruption/partial result — первичные
исходы. Тела стали durable timeline state: `place_body` проводит named
metric body через единственный mutation path, reopen восстанавливает поля
bit-exact, corruption читается как typed rejection. Walk ведёт foot/COM
кинематику гайта; coupling
joint torques к placement стоп объявлен вне текущего envelope; evaporation —
mass-only без latent heat и saturation curve (declared gaps). Сквозной
`apartment-v2` scenario прошёл: walk→grasp→fill→place→heat→spill→clean→dress
на одном timeline с отрицательными тестами §4 плана — grasp без контакта и со
слабым friction cone отклоняется, boil-over conserves total water bit-exact,
отключение питания замораживает нагрев typed blocker'ом, dress-прерывание
оставляет durable partial 1-of-2, reopen восстанавливает stream/bodies/hash.
Остаётся отдельный gate commit перед Phase 3.
