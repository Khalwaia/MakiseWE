---
status: partially-superseded-by-adr-0009
date: 2026-08-05
updated: 2026-08-19
---

# Real-time production and durable activities

Production связывает simulation time с wall clock 1:1. Physical activity начинается только после durable commit и хранит canonical start/end boundaries. Clock anomaly и restart фиксируются как explicit recovery inputs.

[ADR-0009](0009-canonical-simulation-time.md) supersedes прежнюю общую модель времени: production, acceleration, recovery и replay теперь используют одинаковые canonical scheduling rules и resolution profile. Wall clock меняет execution pace, но не transition semantics.

Сохранённые следствия: LLM latency не сокращает physical duration; downtime не создаёт cognition; active process без автономного physical continuation безопасно останавливается или ждёт explicit recovery transition.
