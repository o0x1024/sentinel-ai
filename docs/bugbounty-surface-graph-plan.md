# Bug Bounty Surface Graph Refactor Plan

## Background

The current Bug Bounty asset model stores domains, IPs, ports, web URLs, and certificates in a
single `bounty_assets` table. That model is too flat for network asset mapping:

- it treats different object types as peer assets
- it mixes current state with discovery observations
- it cannot represent graph relationships cleanly
- it weakens workflow, scheduling, and change-detection integration

This refactor keeps the feature inside Bug Bounty. It does not create a separate ASM product.
Instead, it upgrades the `assets` tab into a program-scoped network surface graph.

## Goals

1. Keep `program` as the top-level boundary.
2. Model network assets by object type, not by display rows.
3. Separate discovery observations from normalized inventory.
4. Make relationships first-class data.
5. Make scheduler, workflows, monitor plugins, and change events flow through the same graph model.
6. Stop writing new network mapping data into `bounty_assets`.

## Target Object Model

Phase 1 focuses on the most useful external mapping objects:

1. organization
2. domain
3. ip_cidr
4. host
5. port
6. service
7. web
8. certificate

Phase 2 can add:

1. asn
2. cloud_resource
3. business_system
4. device

## Data Model

### Core tables

- `surface_assets`
  - shared fields for all asset objects
- `surface_relations`
  - graph edges between assets
- `surface_fingerprints`
  - HTTP/TLS/banner/favicon/JARM/JA3/etc
- `surface_evidence`
  - screenshots, DNS samples, HTTP samples, certificate raw text
- `surface_change_logs`
  - state transitions and graph diffs
- `surface_discovery_runs`
  - workflow/scheduler/manual run records
- `surface_observations`
  - raw discovered facts before merge
- `surface_seeds`
  - scope-derived or manual discovery seeds

### Type extension tables

- `surface_org_assets`
- `surface_domain_assets`
- `surface_ip_assets`
- `surface_host_assets`
- `surface_port_assets`
- `surface_service_assets`
- `surface_web_assets`
- `surface_cert_assets`

## Shared Asset Fields

Each object keeps a common baseline:

- `id`
- `program_id`
- `asset_type`
- `asset_name`
- `display_name`
- `description`
- `org_id`
- `business_unit`
- `project`
- `owner`
- `maintainer`
- `contact`
- `env`
- `internet_exposure`
- `criticality`
- `data_level`
- `source`
- `first_seen_at`
- `last_seen_at`
- `last_verified_at`
- `discovery_task_id`
- `status`
- `alive_status`
- `confidence_score`
- `fingerprint_confidence`
- `risk_score`
- `risk_level`
- `vulnerabilities_count`
- `weak_password_flag`
- `expired_cert_flag`
- `exposed_to_internet_flag`
- `metadata_json`
- `created_at`
- `updated_at`
- `created_by`
- `updated_by`

## Required Relations

Minimum graph edges:

- `organization -> domain`
- `domain -> ip`
- `domain -> host`
- `ip -> host`
- `host -> port`
- `port -> service`
- `service -> web`
- `web -> certificate`
- `asset -> organization`
- `asset -> business_system`
- `asset -> owner`

## Ingestion Pipeline

Plugins and workflows must no longer write directly into normalized inventory.

New pipeline:

1. plugin run creates `surface_discovery_runs`
2. raw plugin output becomes typed artifacts
3. typed artifacts are stored as `surface_observations`
4. merge logic normalizes and deduplicates objects
5. merge logic creates or updates assets
6. merge logic creates graph relations
7. diff logic creates `surface_change_logs`
8. high-risk changes can create Bug Bounty `change_events` or finding candidates

## Workflow Artifact Contract

The existing `subdomains/live_hosts/technologies` artifact contract is too loose. It should evolve
to typed surface artifacts:

- `surface_organizations`
- `surface_domains`
- `surface_ips`
- `surface_hosts`
- `surface_ports`
- `surface_services`
- `surface_webs`
- `surface_certificates`
- `surface_fingerprints`
- `surface_relations`
- `surface_evidence`
- `surface_changes`

Backward-compatible extraction can remain during transition, but new plugins should emit typed
surface objects.

## Plugin Alignment

Relevant plugins in `/Users/like/code/sentinel-plugin` should start producing graph-friendly data:

- `subdomain_enumerator`
  - output typed domains and domain-to-root relations
- `http_prober`
  - output host/service/web observations and HTTP evidence
- `tech_fingerprinter`
  - output web fingerprints and web-to-technology relationships
- `port_monitor`
  - output host, port, service, and change observations
- `cert_monitor`
  - output certificate objects plus web/service/cert relations

## Frontend Direction

The Bug Bounty assets tab becomes a program-scoped `ASM Workspace` with:

1. overview
2. inventory
3. topology
4. discovery runs
5. changes

The frontend must query graph-aware APIs instead of `bounty_list_assets`.

## Breaking Change Strategy

This refactor intentionally does not preserve compatibility or fallback behavior.

Rules:

1. stop writing new network mapping records into `bounty_assets`
2. build new UI against `surface_*` tables only
3. keep `bounty_assets` as legacy read-only data until one-time migration is written
4. migrate old rows later by splitting them into typed objects

## Implementation Stages

### Stage 1

- add design doc
- add `surface_*` database tables
- add shared Rust models
- update workflow artifact types
- enable plugin port registry for built-ins

### Stage 2

- add discovery run and observation write path
- update monitor/workflow ingestion to use typed surface artifacts
- stop direct writes to `bounty_assets` for new recon runs

### Stage 3

- add graph query commands
- implement overview, inventory, topology, changes APIs
- rebuild assets tab UI

### Stage 4

- connect change logs to Bug Bounty change events
- add finding-candidate generation for high-risk changes
- add one-time legacy migration from `bounty_assets`
