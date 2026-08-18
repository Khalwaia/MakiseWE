---
status: accepted
date: 2026-08-19
---

# Canonical simulation time across execution modes

Production, acceleration, recovery и audit используют один canonical transition scheduler и одинаковые интервалы simulation time. Wall clock определяет темп production, но не причинную семантику; разбиение интервала, число потоков, restart и ускорение обязаны давать тот же transition stream и state hash. ADR-0003 сохраняется как история real-time deployment, но его правила downtime читаются через этот более общий контракт.
