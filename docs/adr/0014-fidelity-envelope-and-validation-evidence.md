---
status: accepted
date: 2026-08-25
---

# Fidelity envelope and validation evidence

## Context

[ADR-0005](0005-stable-causal-interfaces.md) требует явных resolution upgrades
и error bounds, но не фиксирует минимальный стандарт для realism claims.
[Biology research](../research/biology-realism.md) показала, что Phase 1
механизмы проходят conservation и replay тесты на synthetic fixtures, при этом
ни один параметр не имеет measured provenance. Coverage matrix помечает
unknowns, но не устанавливает обязательный формат их объявления.

## Decision

1. Каждый mechanism contract объявляет **fidelity envelope**: declared units,
   validity range, provenance tier, uncertainty bound и validation horizon.
2. Provenance tiers (в порядке убывания силы):
   - `measured` — published reference data или direct measurement с DOI/URL;
   - `derived` — расчёт из measured inputs с указанной формулой;
   - `expert_estimate` — оценка с named source;
   - `synthetic_fixture` — test-only value без external validity claim.
3. Realism claim допустим только если все параметры внутри envelope имеют
   provenance `measured` или `derived`, validation scenario сравнивает output
   с independent reference в пределах declared uncertainty, и claim scope
   ограничен validation horizon. Один seed или replay hash не является
   validation evidence.
4. Phase 1 organism parameters имеют текущий provenance `synthetic_fixture`.
   Они корректны как causal integration tests, но не поддерживают biological
   realism claims. Замена на measured/derived значения — отдельный slice с
   explicit artifact activation по [ADR-0010](0010-content-addressed-artifacts.md).
5. Phase 2 physics mechanisms наследуют тот же standard: gravity constant
   (`9.80665 m/s²`) имеет provenance `measured` [CGPM 1901], kinematics —
   exact integer arithmetic; material properties до появления calibrated
   values остаются `synthetic_fixture`.
6. Выход за validity range приводит к typed rejection или `SafeStop`,
   никогда к silent extrapolation.
7. Negative validation tests обязательны: extreme input должен производить
   `OutsideValidityRange`, а не численно стабильный бессмысленный результат.

## Consequences

- Coverage matrix становится operational checklist: каждая строка должна
  ссылаться на envelope или явно оставаться unknown.
- Phase gate review обязан проверять provenance tier каждого нового mechanism
  parameter, а не только conservation и replay parity.
- Термин «realistic» может применяться к конкретному mechanism только после
  mechanism-specific validation; глобальный проектный claim остаётся
  «causally and dimensionally verifiable» до полного покрытия envelopes.
