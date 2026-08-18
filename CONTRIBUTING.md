# Участие в разработке MakiseWE

Спасибо за интерес к проекту. MakiseWE развивается фазами: новая работа должна соответствовать текущему gate и не обещать fidelity без evidence, validity range и upgrade path.

## Перед началом

1. Прочитайте [CONTEXT.md](CONTEXT.md), [INVARIANTS.md](INVARIANTS.md) и связанные ADR.
2. Проверьте [ROADMAP.md](ROADMAP.md) и [coverage matrix](docs/coverage/phase0-coverage-matrix.md).
3. Для заметного изменения откройте issue и опишите observable outcome, causal boundary и non-goals.
4. Не начинайте следующую фазу до отдельного gate commit текущей.

Security reports не публикуются в issues. Следуйте [SECURITY.md](SECURITY.md).

## Development setup

Rust toolchain закреплён в `rust-toolchain.toml`.

```bash
git clone https://github.com/Khalwaia/MakiseWE.git
cd MakiseWE
cargo test --workspace --all-targets
```

Для C++ WorldClient установите CMake, compiler, Protobuf и gRPC development packages, затем выполните:

```bash
cmake -S brain -B build/brain -DCMAKE_BUILD_TYPE=RelWithDebInfo
cmake --build build/brain --parallel
ctest --test-dir build/brain --output-on-failure
```

## Как вносить изменения

Работайте короткими vertical slices:

1. Определите public seam и observable behavior.
2. Добавьте один failing test.
3. Запустите его и зафиксируйте ожидаемый red result.
4. Внесите минимальное изменение до green.
5. Повторите для следующего behavior.
6. После завершения выполните review и полный gate.

Тесты должны проверять public interfaces, а не private functions, внутренние call counts или DB side channels. Expected values берутся из specification, measured reference или явно разобранного примера, а не вычисляются тем же алгоритмом, что production code.

## Изменения causal contracts

Pull request, меняющий mechanism, resolution, morphotype или cognition contract, обязан включать:

- schema и совместимый versioning decision;
- causal inputs/outputs и read/write sets;
- authoritative quantities с units;
- provenance, uncertainty и validity range;
- conservation и failure policies;
- fixture с независимыми expected values;
- resolution-upgrade/rollback evidence, если применимо;
- replay и migration impact;
- обновление coverage matrix и нормативной документации.

Arbitrary normalized authoritative scores, hidden fidelity downgrade, morphotype-specific runtime branches и прямые LLM state deltas не принимаются.

## Проверка

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
```

Если менялся C++ client:

```bash
cmake -S brain -B build/brain -DCMAKE_BUILD_TYPE=RelWithDebInfo
cmake --build build/brain --parallel
ctest --test-dir build/brain --output-on-failure
```

## Pull request

PR должен быть focused и содержать:

- проблему и observable outcome;
- scope и explicit non-goals;
- red/green evidence;
- causal, persistence и compatibility impact;
- validation commands и результаты;
- docs/fixtures/migration updates;
- rollback или `SafeStop` behavior для failure paths.

Используйте Conventional Commits, например `docs:`, `test:`, `fix:`, `feat:` и `refactor:`. Не включайте generated build output, runtime DB, secrets, private conversations или machine-specific paths.

## Лицензирование contributions

Отправляя contribution, вы соглашаетесь распространять его по лицензии проекта [AGPL-3.0-only](LICENSE).
