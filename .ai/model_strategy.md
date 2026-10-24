# AI Model Strategy & Routing Architecture

## Role Allocation

The allocator covers six roles: `boss`, `code`, `debug`, `judge`, `chat`, and
`hermes`. The router is queried for a live assignment; if the assignment
endpoint is unavailable or returns 404, `GET /v1/models` feeds the heuristic
auto-combo. A pinned `--model` wins over every other source.

## Resolution Order

1. `--model` on the CLI.
2. `.ai/models/<mode>.txt` or `.ai/model_<mode>.txt` (per-mode override).
3. Live Vibe Router assignment for the active role.
4. First usable entry in `.ai/model_combo.txt` (auto-seeded when absent).
5. Conservative default (`gpt-4o-mini`).

The judge is always deterministic and independent of the coding model.
`--judge-model` is recorded for audit/display only.

## Operational Parameters

- Temperature: Boss 0.2, Code 0.2, Debug 0.15, Chat 0.5 (configurable in the planner).
- Context management: compact at 75% of the active model window; keep the most
  recent 8 surface items; preserve the DAG.
- Retry: one transparent retry for transport/5xx faults with `X-Vibe-Retry: 1`;
  auth failures are never retried.
- Telemetry: ingest `X-Vibe-Router-Cost-Micros` / `X-VR-Cost-Micros` and the
  matching latency headers into the session DAG.

Do not alter mode boundaries or the read-only Chat contract without a structural
review and an ADR.
