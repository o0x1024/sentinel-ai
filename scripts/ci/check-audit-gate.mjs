#!/usr/bin/env node

import fs from 'node:fs'
import process from 'node:process'

function parseBool(input, fallback) {
  if (input === undefined) return fallback
  const value = String(input).trim().toLowerCase()
  if (['1', 'true', 'yes', 'on'].includes(value)) return true
  if (['0', 'false', 'no', 'off'].includes(value)) return false
  return fallback
}

function getArg(flag) {
  const index = process.argv.indexOf(flag)
  if (index < 0 || index + 1 >= process.argv.length) return undefined
  return process.argv[index + 1]
}

function printUsage() {
  console.log(`Usage:
  node scripts/ci/check-audit-gate.mjs [--input <file>] [--json <payload>] [--fail-on-missing <true|false>]

Input JSON supports:
  1) Direct result:
     { "passed": true, "should_block": false, "reason": "...", "conversation_id": "..." }
  2) Tauri wrapper:
     { "success": true, "data": { ...AuditGateCiResult... }, "error": null }
`)
}

function readStdinSync() {
  try {
    if (process.stdin.isTTY) return ''
    return fs.readFileSync(0, 'utf8')
  } catch {
    return ''
  }
}

function parsePayload(raw) {
  if (!raw || !raw.trim()) return null
  try {
    return JSON.parse(raw)
  } catch {
    return null
  }
}

function normalizeResult(payload, failOnMissing) {
  if (!payload || typeof payload !== 'object') {
    return {
      passed: !failOnMissing,
      should_block: failOnMissing,
      reason: failOnMissing
        ? 'Missing policy gate payload (fail_on_missing=true)'
        : 'Missing policy gate payload (fail_on_missing=false)',
      conversation_id: 'unknown',
      source: 'fallback_missing_payload',
    }
  }

  if (Object.prototype.hasOwnProperty.call(payload, 'success')) {
    if (!payload.success) {
      return {
        passed: false,
        should_block: true,
        reason: payload.error || 'Tauri command returned success=false',
        conversation_id: 'unknown',
        source: 'tauri_error',
      }
    }
    return normalizeResult(payload.data, failOnMissing)
  }

  const passed = payload.passed === true
  const shouldBlock = payload.should_block === true || !passed
  return {
    passed,
    should_block: shouldBlock,
    reason: payload.reason || (shouldBlock ? 'Blocked by policy gate' : 'Passed policy gate'),
    conversation_id: payload.conversation_id || 'unknown',
    source: payload.source || 'direct_payload',
    profile: payload.profile,
    generated_at: payload.generated_at,
  }
}

if (process.argv.includes('--help') || process.argv.includes('-h')) {
  printUsage()
  process.exit(0)
}

const failOnMissing = parseBool(getArg('--fail-on-missing'), true)
const inputPath = getArg('--input')
const inlineJson = getArg('--json')

let raw = ''
if (inlineJson) {
  raw = inlineJson
} else if (inputPath) {
  try {
    raw = fs.readFileSync(inputPath, 'utf8')
  } catch (error) {
    console.error(`[audit-gate] Failed to read input file: ${inputPath}`)
    console.error(String(error))
    process.exit(2)
  }
} else {
  raw = readStdinSync()
}

const payload = parsePayload(raw)
const result = normalizeResult(payload, failOnMissing)

const summary = `[audit-gate] conversation=${result.conversation_id} passed=${result.passed} should_block=${result.should_block} source=${result.source}`
console.log(summary)
if (result.reason) {
  console.log(`[audit-gate] reason: ${result.reason}`)
}

if (result.should_block) {
  process.exit(1)
}
process.exit(0)
