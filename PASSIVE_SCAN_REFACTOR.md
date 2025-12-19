# Passive Scan Refactoring Plan

This document tracks the progress of refactoring the passive scanning module to improve performance and correctness, specifically for history traffic scanning.

## Objectives
1.  **Replace separate `scan_request`/`scan_response`** with a transaction-oriented `scan_transaction` for passive analysis.
2.  **Implement Plugin Engine Pooling**: Avoid creating a new V8 runtime for every request by reusing plugin instances.
3.  **Fix Context Loss**: Ensure the scanning process has access to both Request and Response data simultaneously.

## Implementation Steps

### Phase 1: Sentinel Plugins Core (`sentinel-plugins`)

- [ ] **1.1 Update `PluginEngine` interface**
    - Add `scan_transaction` method.
    - Implement logic to inject both `request` and `response` objects into the JS runtime.
    - Ensure JS plugins can access `req` and `res` globals or arguments.

- [ ] **1.2 Create `PluginExecutor` and Pool**
    - Create a structure to hold a long-lived `PluginEngine`.
    - Implement a pooling mechanism (e.g., using `deadpool` or a simple generic pool) or a long-running Task worker that holds the engine.
    - Update `PluginManager` or creating a specific `PassivePluginManager` to manage these pools.

### Phase 2: Sentinel Passive Pipeline (`sentinel-passive`)

- [ ] **2.1 Refactor code in `scanner.rs`**
    - Remove the inefficient `tokio::task::spawn_blocking` loop that creates new engines.
    - Integrate with the new pooled `scan_transaction` API.
    - Modify `process_response` to verify it retrieves the cached request and submits the pair as a transaction.

- [ ] **2.2 Clean up Deprecated Logic**
    - Remove the Mock Request usage in `scan_response`.
    - Remove single-side `scan_request` for passive history (unless deemed necessary for specific checks, but prioritize transaction scan).

### Phase 3: Verification

- [ ] **3.1 Test compilation**
- [ ] **3.2 Verify performance improvement (code walkthrough)**
