# AI Assistant Paper Trading Mission Plan

## Background

Target user request:

> 请你开始模拟炒股，要求收益率最大化，起始资金 10000 元，10 天后给我结果并把完全的过程复盘给我。

Current AI Assistant can run tool-enabled turns, use web/search/http tools, persist task ledgers, and record harness events. It cannot yet execute this request as a reliable 10-day autonomous mission because the request requires durable scheduling, market data, portfolio accounting, trading constraints, and structured replay evidence.

## Product Goal

Build a paper-trading branch of AI Assistant that can create, run, monitor, and close a simulated trading mission.

The first deliverable should support:

- Starting capital: configurable, default `10000`.
- Duration: configurable by trading days or calendar days, default `10`.
- Market scope: explicitly selected before mission start.
- Paper orders only: no real broker execution.
- Full audit trail: every quote, decision, order, fill, portfolio change, and final review must be persisted.
- Final report: generated automatically when the mission reaches its end condition.

## Non-Goals

- No real-money trading.
- No broker account login.
- No hidden execution through browser automation.
- No simulated performance claim without persisted inputs and calculations.
- No fallback data source that silently changes semantics. If the selected market data source fails, the mission records failure and waits for retry or user action.

## First-Principles Breakdown

The user asks for an outcome after 10 days, but the real system problem is not "make the LLM remember the task". The system must own the mission state.

Core facts that must be represented outside the model:

- Time: mission start, end, trading sessions, scheduled decision windows.
- Data: quote snapshots, source identity, fetch timestamp, raw response digest.
- State: cash, positions, pending orders, filled orders, realized/unrealized PnL.
- Policy: risk limits, allowed symbols, order sizing, maximum turnover, stop rules.
- Reasoning: LLM decision prompt, selected evidence, decision text, rejected alternatives.
- Completion: final state must be derived from ledger data, not from the assistant's final wording.

## Capability Gaps

### 1. Mission Runner

Need a backend-owned mission runner, separate from one ordinary assistant turn.

Required behavior:

- Create a durable mission row.
- Schedule recurring decision ticks.
- Recover active missions after app restart.
- Prevent duplicate execution for the same mission tick.
- Persist terminal states: `completed`, `failed`, `cancelled`, `expired`.
- Generate final report when end condition is reached.

### 2. Market Data Tooling

Need first-class market data tools instead of generic web search.

Required tools:

- `market_quote`: get latest quote for one or more symbols.
- `market_history`: get historical candles for strategy context.
- `market_calendar`: resolve trading sessions and holidays.
- `market_news`: optional, source-specific market news retrieval.

Data source must be configured explicitly per market. The mission should fail loudly if the source is missing, rate-limited, or returns incomplete data.

### 3. Paper Trading Ledger

Need domain tables and service methods for portfolio accounting.

Minimum entities:

- `paper_trading_missions`
- `paper_trading_accounts`
- `paper_trading_positions`
- `paper_trading_orders`
- `paper_trading_fills`
- `paper_trading_quote_snapshots`
- `paper_trading_decisions`
- `paper_trading_daily_nav`
- `paper_trading_reports`

### 4. Trading Policy

Need explicit user-visible constraints before mission start.

Minimum policy fields:

- Market: `US`, `HK`, `CN`, or custom.
- Tradable universe: fixed symbols or dynamic discovery rule.
- Initial capital.
- Max position weight.
- Max single-order value.
- Cash reserve.
- Stop-loss and take-profit rules.
- Trading frequency.
- Whether short selling is allowed. Default: no.
- Whether margin is allowed. Default: no.

### 5. AI Decision Contract

Need a structured decision output, not free-form prose.

Each decision tick should produce:

- Observed market data ids.
- Current portfolio snapshot.
- Candidate actions considered.
- Final action list.
- Risk check result.
- Explanation.
- Confidence.
- Reasons for holding cash or skipping trades.

The executor should validate the structure before writing orders.

### 6. UI Surfaces

Need a dedicated AI Assistant branch UI or panel for paper trading missions.

Minimum UI:

- Start mission dialog.
- Active mission card.
- Portfolio summary.
- Order/fill ledger.
- Decision timeline.
- Data-source health.
- Final report viewer.

The default AI chat can still be the conversational entry point, but mission state should be visible and controllable outside chat text.

## Proposed Architecture

### Backend Modules

Add new modules under `src-tauri/src/commands` and domain services under a dedicated service namespace.

Suggested split:

- `paper_trading_commands.rs`: Tauri command boundary.
- `paper_trading_service.rs`: mission lifecycle and portfolio operations.
- `paper_trading_scheduler.rs`: backend-owned tick scheduling and restart recovery.
- `paper_trading_market_data.rs`: provider interface and quote normalization.
- `paper_trading_accounting.rs`: cash, position, fill, NAV calculations.
- `paper_trading_report.rs`: final report assembly.

If any file approaches 2000 lines, split by function area before continuing.

### Frontend Modules

Suggested split:

- `src/api/paperTrading.ts`
- `src/components/Agent/PaperTradingMissionDialog.vue`
- `src/components/Agent/PaperTradingMissionPanel.vue`
- `src/components/Agent/paperTradingPresentation.ts`
- `src/components/Agent/paperTradingTypes.ts`

### Tool Integration

Add tools only after the domain service exists:

- `paper_trading_create_mission`
- `paper_trading_get_mission`
- `paper_trading_record_decision`
- `paper_trading_place_order`
- `paper_trading_close_mission`
- `market_quote`
- `market_history`
- `market_calendar`

The assistant should not directly mutate portfolio tables through generic shell/http calls. All trading mutations go through domain commands/tools.

## Data Model Draft

### `paper_trading_missions`

- `id`
- `conversation_id`
- `title`
- `market`
- `status`
- `initial_cash`
- `current_cash`
- `currency`
- `policy_json`
- `started_at`
- `ends_at`
- `last_tick_at`
- `next_tick_at`
- `completed_at`
- `failure_reason`
- `created_at`
- `updated_at`

### `paper_trading_orders`

- `id`
- `mission_id`
- `decision_id`
- `symbol`
- `side`
- `order_type`
- `quantity`
- `limit_price`
- `status`
- `submitted_at`
- `cancelled_at`
- `rejected_reason`

### `paper_trading_fills`

- `id`
- `order_id`
- `mission_id`
- `symbol`
- `side`
- `quantity`
- `price`
- `fee`
- `filled_at`

### `paper_trading_decisions`

- `id`
- `mission_id`
- `tick_id`
- `input_snapshot_json`
- `decision_json`
- `risk_check_json`
- `model_provider`
- `model_name`
- `created_at`

## Execution Flow

1. User asks to start simulated stock trading.
2. Assistant detects this as a paper-trading mission request.
3. System opens a start dialog if required fields are missing.
4. Backend creates mission, account, and initial schedule.
5. Scheduler runs tick:
   - Load mission and account state.
   - Fetch quote/history/calendar data.
   - Build LLM decision prompt.
   - Validate structured decision.
   - Run risk checks.
   - Create orders.
   - Simulate fills using deterministic fill rules.
   - Update cash, positions, NAV, and decision ledger.
   - Schedule next tick.
6. On end condition, generate report from persisted ledger.
7. Assistant posts final summary and links the full report.

## Fill Simulation Rules

V1 should use deterministic rules:

- Market buy: fill at latest ask if available, otherwise latest close.
- Market sell: fill at latest bid if available, otherwise latest close.
- Limit buy: fill only if latest low or current price is less than or equal to limit.
- Limit sell: fill only if latest high or current price is greater than or equal to limit.
- Fee model: configured fixed percent with minimum fee.

No random fill behavior in V1.

## Report Requirements

The final report must include:

- Starting capital and final NAV.
- Absolute return and percentage return.
- Benchmark comparison if configured.
- Daily NAV table.
- Holdings timeline.
- Order and fill list.
- Decision-by-decision replay.
- Largest gain/loss contributors.
- Mistakes and missed opportunities.
- Data gaps or failed ticks.
- Whether the mission respected its risk policy.

## Implementation Phases

### Phase 1: Durable Domain Model

- Add database tables and migration.
- Add Rust types and service methods.
- Add account/NAV calculation tests.
- Add Tauri commands for create/get/list/cancel mission.

Exit criteria:

- A mission can be created, loaded, cancelled, and listed after app restart.
- Account state is calculated from persisted fills.

### Phase 2: Market Data Provider

- Add explicit provider config.
- Add quote/history/calendar normalization.
- Add `market_quote`, `market_history`, and `market_calendar` tools.
- Add provider failure surfaces.

Exit criteria:

- A quote snapshot is persisted with source metadata.
- Missing provider config fails with a clear error.

### Phase 3: Paper Trading Executor

- Add decision tick runner.
- Add structured LLM decision contract.
- Add risk checks.
- Add deterministic order fill simulation.
- Add mission tick lock to prevent duplicate ticks.

Exit criteria:

- A mission tick can fetch data, get a decision, place simulated orders, and update NAV.
- Invalid model output does not mutate portfolio state.

### Phase 4: Scheduler And Recovery

- Add backend-owned scheduler loop.
- Resume active missions on startup.
- Handle missed ticks explicitly.
- Add retry and terminal failure states.

Exit criteria:

- Closing and reopening the app does not lose active mission state.
- Scheduler does not double-run the same tick.

### Phase 5: Assistant And UI Integration

- Add chat intent detection for paper trading mission creation.
- Add start mission dialog.
- Add active mission panel.
- Add decision/order/NAV timeline.
- Add final report viewer.

Exit criteria:

- User can start a 10-day mission from AI Assistant.
- Mission progress is visible without reading raw chat logs.

### Phase 6: Final Report And Review

- Add report generation service.
- Add final assistant message with report link.
- Add regression tests for report calculations.

Exit criteria:

- Completed mission produces a report entirely from persisted ledger data.
- Report numbers match account/NAV calculations.

## Validation Plan

- Unit tests for accounting calculations.
- Unit tests for risk-policy rejection.
- Unit tests for fill simulation.
- Integration tests for mission lifecycle commands.
- Scheduler test for restart recovery.
- Frontend component tests for mission panel rendering.
- Manual run with a short 2-tick mission before enabling 10-day duration.

## Open Decisions

- Which market should V1 support first: US stocks, A shares, HK stocks, or crypto-like always-open symbols?
- Which market data provider should be the first supported provider?
- Should tick frequency be daily close, market open/close, or configurable intraday?
- Should the assistant optimize only return, or return under max drawdown constraints?
- Should mission creation require user confirmation every time because this is financial-domain behavior, even though it is paper trading?

## Recommended V1 Scope

Start with a conservative V1:

- Paper trading only.
- One selected market.
- User-provided symbol universe.
- Daily tick.
- Long-only.
- No margin.
- Deterministic fills.
- Full ledger and final report.

This gives a reliable product path without pretending the current AI Assistant can safely act as a 10-day autonomous trading agent.
