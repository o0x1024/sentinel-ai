# Recon Playbook

## Goal

Map all reachable attack surfaces with minimal noise.

## Fast Checklist

- Enumerate endpoints from HTML, JS bundles, API docs, and robots.
- Compare behaviors across methods: GET/POST/PUT/DELETE/OPTIONS.
- Test content types: JSON, form-urlencoded, multipart, plain text.
- Inspect cookies, JWT claims, and state transitions.
- Probe path normalization: `..`, `%2e`, double-encoding, trailing slash.
- Probe host/proxy trust headers when reverse proxy behavior is present.

## Request Baseline

Capture one clean baseline request per endpoint with:

- Full URL and method
- Headers
- Body
- Response status, length, key headers, and response time

Use the baseline as the control for every payload experiment.

## Static + Dynamic Pairing

- From source code, list: user-controlled inputs, transformation functions, sink calls.
- From live traffic, confirm each sink is reachable with controllable data.
- Prioritize code paths that touch filesystem, template engines, shell calls, and DB query builders.

## Recon Stop Condition

Stop reconnaissance when each candidate endpoint has:

- A baseline request
- At least one mutation test
- A hypothesis label (or explicit discard reason)
