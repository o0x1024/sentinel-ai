use std::sync::Arc;

use anyhow::Result;
use sentinel_db::{Database, TrafficPluginMetadata};

use crate::services::{DatabaseService, DictionaryService};

const TECH_FINGERPRINTER_CODE: &str = r#"
function normalizeBaseUrl(target) {
  if (!target || typeof target !== 'string') return null;
  if (target.startsWith('http://') || target.startsWith('https://')) return target;
  return `https://${target}`;
}

function parseTitle(html) {
  const match = html.match(/<title[^>]*>([^<]+)<\/title>/i);
  return match ? match[1] : '';
}

function matcherHit(ctx, matcher) {
  const part = (matcher.part || '').toLowerCase();
  const type = (matcher.type || 'contains').toLowerCase();
  const value = matcher.value;

  if (part === 'status') {
    if (type === 'in' && Array.isArray(value)) return value.includes(ctx.status);
    return ctx.status === value;
  }

  if (part === 'header') {
    const key = String(matcher.key || '').toLowerCase();
    const headerValue = ctx.headers[key];
    if (type === 'exists') return typeof headerValue === 'string' && headerValue.length > 0;
    if (typeof headerValue !== 'string') return false;
    if (type === 'equals') return headerValue === value;
    if (type === 'regex') return new RegExp(value, 'i').test(headerValue);
    return headerValue.toLowerCase().includes(String(value).toLowerCase());
  }

  const subject = part === 'title' ? ctx.title : ctx.body;
  if (type === 'equals') return subject === value;
  if (type === 'regex') return new RegExp(value, 'i').test(subject);
  return subject.toLowerCase().includes(String(value).toLowerCase());
}

function ruleMatched(ctx, metadata) {
  const matchers = Array.isArray(metadata.matchers) ? metadata.matchers : [];
  if (matchers.length === 0) return false;
  const operator = (metadata.operator || 'or').toLowerCase();
  return operator === 'and'
    ? matchers.every(matcher => matcherHit(ctx, matcher))
    : matchers.some(matcher => matcherHit(ctx, matcher));
}

async function loadFingerprintEntries(dictionaryId) {
  const configuredId = dictionaryId || await Sentinel.Dictionary.getDefaultId('fingerprint_rule') || 'builtin_web_fingerprint_rules';
  const entries = await Sentinel.Dictionary.getEntries(configuredId, 10000);
  return entries.filter(entry => !entry.metadata || entry.metadata.enabled !== false);
}

export function get_input_schema() {
  return {
    type: 'object',
    properties: {
      url: { type: 'string' },
      base_url: { type: 'string' },
      targets: { type: 'array', items: { type: 'string' } },
      dictionary_id: { type: 'string' },
      max_targets: { type: 'number', default: 20 }
    }
  };
}

export async function execute(input) {
  const candidates = [];
  if (typeof input.url === 'string') candidates.push(input.url);
  if (typeof input.base_url === 'string') candidates.push(input.base_url);
  if (Array.isArray(input.targets)) candidates.push(...input.targets);
  const targets = [...new Set(candidates.map(normalizeBaseUrl).filter(Boolean))].slice(0, Number(input.max_targets || 20));
  const entries = await loadFingerprintEntries(input.dictionary_id);

  const allResults = [];
  for (const target of targets) {
    try {
      const response = await fetch(target, { timeout: 10000, headers: { 'User-Agent': 'Sentinel-Tech-Fingerprinter' } });
      const body = await response.text();
      const headers = {};
      response.headers.forEach((value, key) => { headers[String(key).toLowerCase()] = value; });
      const ctx = {
        status: response.status,
        headers,
        body,
        title: parseTitle(body)
      };

      const technologies = [];
      const surfaceFingerprints = [];
      for (const entry of entries) {
        const metadata = entry.metadata || {};
        if (!ruleMatched(ctx, metadata)) continue;
        const techName = metadata.name || metadata.product || entry.word;
        technologies.push({
          name: techName,
          category: entry.category || null,
          confidence: Number(metadata.confidence || 0.8)
        });
        surfaceFingerprints.push({
          fingerprint_type: 'technology',
          fingerprint_key: entry.word,
          fingerprint_value: techName,
          confidence_score: Number(metadata.confidence || 0.8),
          source: 'tech_fingerprinter',
          metadata: {
            url: target,
            rule_id: entry.word,
            category: entry.category || null
          }
        });
      }

      allResults.push({
        url: target,
        technologies,
        surface_fingerprints: surfaceFingerprints
      });
    } catch (error) {
      Sentinel.log('warn', `tech_fingerprinter failed for ${target}: ${String(error)}`);
    }
  }

  const first = allResults[0] || { url: input.url || input.base_url || '', technologies: [], surface_fingerprints: [] };
  return {
    success: true,
    url: first.url,
    technologies: first.technologies,
    surface_fingerprints: first.surface_fingerprints,
    results: allResults
  };
}
"#;

const SENSITIVE_FILE_SCANNER_CODE: &str = r#"
function normalizeBaseUrl(target) {
  if (!target || typeof target !== 'string') return null;
  if (target.startsWith('http://') || target.startsWith('https://')) return target.replace(/\/+$/, '');
  return `https://${target}`.replace(/\/+$/, '');
}

function joinUrl(baseUrl, path) {
  const cleanPath = String(path || '').replace(/^\/+/, '');
  return `${baseUrl}/${cleanPath}`;
}

async function loadFileEntries(dictionaryId) {
  const configuredId = dictionaryId || await Sentinel.Dictionary.getDefaultId('sensitive_file') || 'builtin_sensitive_files_web';
  const entries = await Sentinel.Dictionary.getEntries(configuredId, 10000);
  return entries.filter(entry => !entry.metadata || entry.metadata.enabled !== false);
}

function matcherHit(body, matcher) {
  if (!matcher || !matcher.value) return true;
  const type = (matcher.type || 'contains').toLowerCase();
  if (type === 'regex') return new RegExp(matcher.value, 'i').test(body);
  return body.toLowerCase().includes(String(matcher.value).toLowerCase());
}

export function get_input_schema() {
  return {
    type: 'object',
    properties: {
      url: { type: 'string' },
      base_url: { type: 'string' },
      targets: { type: 'array', items: { type: 'string' } },
      dictionary_id: { type: 'string' },
      max_targets: { type: 'number', default: 10 }
    }
  };
}

export async function execute(input) {
  const candidates = [];
  if (typeof input.url === 'string') candidates.push(input.url);
  if (typeof input.base_url === 'string') candidates.push(input.base_url);
  if (Array.isArray(input.targets)) candidates.push(...input.targets);
  const targets = [...new Set(candidates.map(normalizeBaseUrl).filter(Boolean))].slice(0, Number(input.max_targets || 10));
  const entries = await loadFileEntries(input.dictionary_id);
  const findings = [];

  for (const target of targets) {
    for (const entry of entries) {
      const metadata = entry.metadata || {};
      const path = metadata.path || entry.word;
      const severity = metadata.severity || 'medium';
      const url = joinUrl(target, path);
      try {
        const response = await fetch(url, { timeout: 8000, headers: { 'User-Agent': 'Sentinel-Sensitive-File-Scanner' } });
        if (response.status !== 200) continue;
        const body = await response.text();
        const matchers = Array.isArray(metadata.matchers) ? metadata.matchers : [];
        if (matchers.length > 0 && !matchers.every(matcher => matcherHit(body, matcher))) continue;

        findings.push({
          title: `Sensitive file exposed: ${path}`,
          description: metadata.description || `Accessible sensitive resource detected at ${url}`,
          finding_type: 'sensitive_file_exposure',
          severity,
          confidence: 'high',
          url,
          evidence: body.slice(0, 500),
          cwe: metadata.cwe || 'CWE-200',
          remediation: metadata.remediation || 'Restrict access to sensitive files and remove them from public web roots.'
        });
      } catch (error) {
        Sentinel.log('debug', `sensitive_file_scanner skipped ${url}: ${String(error)}`);
      }
    }
  }

  return {
    success: true,
    findings,
    findings_count: findings.length
  };
}
"#;

const RISK_SCANNER_CODE: &str = r#"
const SAFE_METHODS = new Set(['GET', 'HEAD', 'OPTIONS', 'POST']);

function normalizeBaseUrl(target) {
  if (!target || typeof target !== 'string') return null;
  const trimmed = target.trim().replace(/\/+$/, '');
  if (!trimmed) return null;
  if (trimmed.startsWith('http://') || trimmed.startsWith('https://')) return trimmed;
  return `https://${trimmed}`;
}

function joinUrl(baseUrl, path) {
  const rendered = String(path || '').trim();
  if (!rendered) return baseUrl;
  if (rendered.startsWith('http://') || rendered.startsWith('https://')) return rendered;
  return `${baseUrl}/${rendered.replace(/^\/+/, '')}`;
}

function asArray(value) {
  return Array.isArray(value) ? value : value == null ? [] : [value];
}

function parseMetadata(value) {
  if (!value) return {};
  if (typeof value === 'string') {
    try {
      return JSON.parse(value);
    } catch {
      return {};
    }
  }
  return typeof value === 'object' ? value : {};
}

function getInputNumber(input, camelKey, snakeKey, fallback) {
  const value = input?.[snakeKey] ?? input?.[camelKey];
  const parsed = Number(value ?? fallback);
  return Number.isFinite(parsed) ? parsed : fallback;
}

function getInputBoolean(input, camelKey, snakeKey, fallback) {
  const value = input?.[snakeKey] ?? input?.[camelKey];
  return typeof value === 'boolean' ? value : fallback;
}

function getInputString(input, camelKey, snakeKey, fallback = '') {
  const value = input?.[snakeKey] ?? input?.[camelKey];
  return typeof value === 'string' && value.trim().length > 0 ? value : fallback;
}

function parseTitle(html) {
  const match = html.match(/<title[^>]*>([^<]+)<\/title>/i);
  return match ? match[1].trim() : '';
}

function deepGet(source, path) {
  if (!path) return source;
  return String(path).split('.').reduce((acc, segment) => {
    if (acc == null) return undefined;
    const indexed = segment.match(/^([^[\]]+)\[(\d+)\]$/);
    if (indexed) {
      const [, key, index] = indexed;
      return acc?.[key]?.[Number(index)];
    }
    return acc?.[segment];
  }, source);
}

function renderTemplate(value, context) {
  if (typeof value === 'string') {
    return value.replace(/\{\{\s*([^}]+?)\s*\}\}/g, (_, expr) => {
      const resolved = deepGet(context, String(expr).trim());
      if (resolved == null) return '';
      return typeof resolved === 'string' ? resolved : JSON.stringify(resolved);
    });
  }
  if (Array.isArray(value)) return value.map(item => renderTemplate(item, context));
  if (value && typeof value === 'object') {
    return Object.fromEntries(Object.entries(value).map(([key, item]) => [key, renderTemplate(item, context)]));
  }
  return value;
}

function tryParseJson(body) {
  try {
    return JSON.parse(body);
  } catch {
    return undefined;
  }
}

function matcherHit(ctx, matcher) {
  const part = String(matcher?.part || 'body').toLowerCase();
  const type = String(matcher?.type || 'contains').toLowerCase();
  const expected = matcher?.value;

  if (part === 'status') {
    if (type === 'in' && Array.isArray(expected)) return expected.includes(ctx.status);
    return ctx.status === expected;
  }

  const source = part === 'header'
    ? String(ctx.headers[String(matcher?.key || '').toLowerCase()] || '')
    : part === 'title'
      ? ctx.title
      : part === 'json'
        ? deepGet(ctx.json || {}, matcher?.path || matcher?.key || '')
        : ctx.body;

  if (type === 'exists') return source !== undefined && source !== null && String(source).length > 0;
  if (type === 'equals') return String(source) === String(expected ?? '');
  if (type === 'in' && Array.isArray(expected)) return expected.map(String).includes(String(source));
  if (type === 'regex') {
    try {
      return new RegExp(String(expected || ''), 'i').test(String(source || ''));
    } catch {
      return false;
    }
  }
  return String(source || '').toLowerCase().includes(String(expected || '').toLowerCase());
}

function matchAll(ctx, matchers, operator) {
  if (!Array.isArray(matchers) || matchers.length === 0) return false;
  return String(operator || 'and').toLowerCase() === 'or'
    ? matchers.some(matcher => matcherHit(ctx, matcher))
    : matchers.every(matcher => matcherHit(ctx, matcher));
}

function evaluateCondition(condition, context) {
  if (condition?.matchers) {
    return matchAll(context, asArray(condition.matchers), condition.operator || 'and');
  }

  const field = String(condition?.field || '');
  const op = String(condition?.op || 'equals').toLowerCase();
  const actual = deepGet(context, field);
  const expected = condition?.value;

  if (op === 'exists') return actual !== undefined && actual !== null && String(actual).length > 0;
  if (op === 'contains') return String(actual || '').toLowerCase().includes(String(expected || '').toLowerCase());
  if (op === 'in' && Array.isArray(expected)) return expected.map(String).includes(String(actual));
  if (op === 'regex') {
    try {
      return new RegExp(String(expected || ''), 'i').test(String(actual || ''));
    } catch {
      return false;
    }
  }
  return String(actual) === String(expected);
}

function evaluatePreconditions(preconditions, context) {
  if (!Array.isArray(preconditions) || preconditions.length === 0) return true;
  return preconditions.every(condition => evaluateCondition(condition, context));
}

async function loadRules(input) {
  const injectedEntries = input?.dictionary_entries ?? input?.dictionaryEntries;
  if (Array.isArray(injectedEntries) && injectedEntries.length > 0) {
    return injectedEntries.map(entry => ({ ...entry, metadata: parseMetadata(entry.metadata) }));
  }

  const defaultDictionaryId = await Sentinel.Dictionary.getDefaultId('poc_rule');
  const candidates = [
    getInputString(input, 'dictionaryId', 'dictionary_id'),
    defaultDictionaryId,
    'builtin_safe_poc_rules',
    'Safe POC Rules'
  ].filter(Boolean);

  for (const candidate of candidates) {
    const entries = await Sentinel.Dictionary.getEntries(candidate, 10000);
    if (Array.isArray(entries) && entries.length > 0) {
      return entries
        .filter(entry => entry && typeof entry.word === 'string')
        .map(entry => ({ ...entry, metadata: parseMetadata(entry.metadata) }));
    }
  }

  return [];
}

function buildBaseContext(target, input, rule) {
  let parsed;
  try {
    parsed = new URL(target);
  } catch {
    parsed = { hostname: '', host: '', origin: target, pathname: '/', protocol: '' };
  }

  return {
    target,
    base_url: target,
    url: target,
    host: parsed.host || '',
    hostname: parsed.hostname || '',
    origin: parsed.origin || target,
    path: parsed.pathname || '/',
    protocol: parsed.protocol ? String(parsed.protocol).replace(/:$/, '') : '',
    rule_id: rule.word,
    category: rule.category || '',
    variables: input?.variables || {},
    fingerprints: asArray(input?.fingerprints),
    technologies: [
      ...asArray(input?.technologies).map(item => typeof item === 'string' ? item : item?.name),
      ...asArray(input?.surface_bundle?.technologies).map(item => typeof item === 'string' ? item : item?.name),
      ...asArray(input?.surface_bundle?.fingerprints)
    ].filter(Boolean)
  };
}

function ruleApplies(rule, input, baseContext) {
  const metadata = parseMetadata(rule.metadata);
  if (metadata.enabled === false) return false;

  const scope = metadata.match_scope || {};
  const expectedFingerprints = asArray(scope.fingerprints).map(item => String(item).toLowerCase());
  const observedFingerprints = baseContext.technologies.map(item => String(item).toLowerCase());
  if (expectedFingerprints.length > 0 && !expectedFingerprints.some(item => observedFingerprints.includes(item))) {
    return false;
  }

  return evaluatePreconditions(asArray(metadata.preconditions), baseContext);
}

function normalizeRequests(metadata) {
  const requests = Array.isArray(metadata.requests) ? metadata.requests : metadata.request ? [metadata.request] : [];
  return requests.map((request, index) => ({
    id: request?.id || `step_${index + 1}`,
    method: String(request?.method || 'GET').toUpperCase(),
    path: request?.path || '/',
    headers: request?.headers || {},
    body: request?.body,
    operator: request?.operator || 'and',
    timeout_ms: Number(request?.timeout_ms || 8000),
    matchers: Array.isArray(request?.matchers) ? request.matchers : [],
    extractors: Array.isArray(request?.extractors) ? request.extractors : [],
    preconditions: Array.isArray(request?.preconditions) ? request.preconditions : []
  }));
}

function applyExtractors(extractors, responseContext) {
  const extracted = {};
  for (const extractor of asArray(extractors)) {
    const name = String(extractor?.name || '').trim();
    if (!name) continue;

    const part = String(extractor?.part || 'body').toLowerCase();
    const source = part === 'header'
      ? String(responseContext.headers[String(extractor?.key || '').toLowerCase()] || '')
      : part === 'title'
        ? responseContext.title
        : part === 'json'
          ? deepGet(responseContext.json || {}, extractor?.path || extractor?.key || '')
          : responseContext.body;

    if (extractor?.type === 'regex') {
      try {
        const match = String(source || '').match(new RegExp(String(extractor?.pattern || extractor?.value || ''), 'i'));
        if (match?.[1]) extracted[name] = match[1];
      } catch {
        continue;
      }
      continue;
    }

    if (extractor?.type === 'json_path') {
      extracted[name] = deepGet(responseContext.json || {}, extractor?.path || '');
      continue;
    }

    extracted[name] = source;
  }
  return extracted;
}

async function fetchWithTimeout(url, init, timeout) {
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), timeout);
  try {
    return await fetch(url, { ...init, signal: controller.signal });
  } finally {
    clearTimeout(timer);
  }
}

async function executeRule(target, rule, input, settings) {
  const metadata = parseMetadata(rule.metadata);
  const baseContext = buildBaseContext(target, input, rule);
  if (!ruleApplies(rule, input, baseContext)) return { matched: false };

  const requests = normalizeRequests(metadata);
  if (requests.length === 0) return { matched: false };

  const executionContext = { steps: {}, extracted: {} };
  const evidenceSteps = [];
  let lastResponseContext = null;

  for (const request of requests) {
    const context = {
      ...baseContext,
      ...executionContext,
      steps: executionContext.steps,
      extracted: executionContext.extracted
    };

    if (!evaluatePreconditions(request.preconditions, context)) {
      return {
        matched: false,
        evidence: { rule_id: rule.word, target, steps: evidenceSteps, skipped_at: request.id }
      };
    }

    const method = String(request.method || 'GET').toUpperCase();
    if (settings.safeMode && !SAFE_METHODS.has(method)) {
      return { matched: false };
    }

    const renderedPath = renderTemplate(request.path, context);
    const url = joinUrl(target, renderedPath);
    const headers = renderTemplate(request.headers || {}, context);
    const body = request.body == null ? undefined : renderTemplate(request.body, context);

    try {
      const response = await fetchWithTimeout(
        url,
        {
          method,
          headers: {
            'User-Agent': settings.userAgent,
            ...headers
          },
          body: body == null ? undefined : String(body)
        },
        Number(request.timeout_ms || settings.timeout)
      );

      const responseBody = await response.text();
      const responseHeaders = {};
      response.headers.forEach((value, key) => {
        responseHeaders[String(key).toLowerCase()] = value;
      });
      const responseContext = {
        status: response.status,
        body: responseBody,
        headers: responseHeaders,
        title: parseTitle(responseBody),
        json: tryParseJson(responseBody),
        url
      };

      const extracted = applyExtractors(request.extractors, responseContext);
      executionContext.steps[request.id] = responseContext;
      executionContext.extracted = { ...executionContext.extracted, ...extracted };
      lastResponseContext = responseContext;

      const stepMatchers = asArray(request.matchers);
      const stepMatched = stepMatchers.length === 0
        ? true
        : matchAll(responseContext, stepMatchers, request.operator || 'and');

      evidenceSteps.push({
        id: request.id,
        method,
        url,
        status: response.status,
        matched: stepMatched,
        extracted
      });

      if (!stepMatched) {
        return {
          matched: false,
          evidence: { rule_id: rule.word, target, steps: evidenceSteps, extracted: executionContext.extracted }
        };
      }
    } catch (error) {
      evidenceSteps.push({
        id: request.id,
        method,
        url,
        matched: false,
        extracted: {}
      });
      return {
        matched: false,
        evidence: {
          rule_id: rule.word,
          target,
          steps: evidenceSteps,
          error: error instanceof Error ? error.message : String(error)
        }
      };
    }
  }

  const finalMatchers = asArray(metadata.matchers);
  const finalMatched = finalMatchers.length === 0
    ? true
    : lastResponseContext != null && matchAll(lastResponseContext, finalMatchers, metadata.operator || 'and');

  const evidence = {
    rule_id: rule.word,
    target,
    steps: evidenceSteps,
    final_matchers: finalMatchers,
    extracted: executionContext.extracted
  };

  if (!finalMatched || !lastResponseContext) {
    return { matched: false, evidence };
  }

  return {
    matched: true,
    finding: {
      title: metadata.name || rule.word,
      description: metadata.description || `Risk verification rule ${rule.word} matched.`,
      finding_type: metadata.finding_type || rule.category || 'risk_verification',
      severity: metadata.severity || 'medium',
      confidence: metadata.confidence || 'high',
      url: lastResponseContext.url,
      evidence: lastResponseContext.body.slice(0, 500),
      cwe: metadata.cwe || '',
      remediation: metadata.remediation || 'Restrict access and harden the affected component.',
      impact: metadata.impact || ''
    },
    evidence
  };
}

export function get_input_schema() {
  return {
    type: 'object',
    properties: {
      url: { type: 'string' },
      base_url: { type: 'string' },
      targets: { type: 'array', items: { type: 'string' } },
      technologies: { type: 'array', items: { type: ['string', 'object'] } },
      fingerprints: { type: 'array', items: { type: 'string' } },
      dictionary_id: { type: 'string' },
      dictionaryEntries: { type: 'array' },
      dictionary_entries: { type: 'array' },
      variables: { type: 'object' },
      safe_mode: { type: 'boolean', default: true },
      max_rules: { type: 'number', default: 20 },
      timeout_ms: { type: 'number', default: 8000 },
      user_agent: { type: 'string' },
      stop_on_first_hit: { type: 'boolean', default: false }
    }
  };
}

export async function execute(input) {
  const safeMode = getInputBoolean(input, 'safeMode', 'safe_mode', true);
  const maxRules = getInputNumber(input, 'maxRules', 'max_rules', 20);
  const timeout = getInputNumber(input, 'timeout', 'timeout_ms', 8000);
  const userAgent = getInputString(input, 'userAgent', 'user_agent', 'Sentinel-Risk-Scanner/2.0');
  const stopOnFirstHit = getInputBoolean(input, 'stopOnFirstHit', 'stop_on_first_hit', false);
  const normalizedInput = {
    ...input,
    safe_mode: safeMode,
    timeout,
    userAgent,
    stopOnFirstHit,
    dictionaryId: getInputString(input, 'dictionaryId', 'dictionary_id')
  };
  const candidates = [];
  if (typeof input.url === 'string') candidates.push(input.url);
  if (typeof input.base_url === 'string') candidates.push(input.base_url);
  if (Array.isArray(input.targets)) candidates.push(...input.targets);
  const targets = [...new Set(candidates.map(normalizeBaseUrl).filter(Boolean))];
  const allRules = await loadRules(normalizedInput);
  const rules = allRules
    .filter(rule => {
      const metadata = parseMetadata(rule.metadata);
      if (metadata.enabled === false) return false;
      if (safeMode && metadata.safe_mode === false) return false;
      return true;
    })
    .slice(0, maxRules);
  const findings = [];
  const evidence = [];
  const surfaceFindings = [];
  let executedRules = 0;

  outer:
  for (const target of targets) {
    for (const rule of rules) {
      executedRules += 1;
      const result = await executeRule(target, rule, normalizedInput, { safeMode, timeout, userAgent });
      if (result.evidence) evidence.push(result.evidence);
      if (!result.matched || !result.finding) continue;
      findings.push(result.finding);
      surfaceFindings.push({
        title: result.finding.title,
        severity: result.finding.severity,
        target: result.finding.url,
        vulnerability_type: result.finding.finding_type || 'risk_verification',
        description: result.finding.description,
        source: 'risk_scanner'
      });
      if (stopOnFirstHit) break outer;
    }
  }

  return {
    success: true,
    findings,
    findings_count: findings.length,
    executed_rules: executedRules,
    evidence,
    surface_artifacts: {
      findings: surfaceFindings,
      evidence
    }
  };
}
"#;

pub async fn initialize_builtin_bounty_resources(db_service: &Arc<DatabaseService>) -> Result<()> {
    let pool = db_service.get_runtime_pool()?;
    let dictionary_service = DictionaryService::new(pool);
    dictionary_service.initialize_builtin_dictionaries().await?;

    ensure_default_dictionary(db_service, "subdomain", "builtin_subdomain_common").await?;
    ensure_default_dictionary(db_service, "sensitive_file", "builtin_sensitive_files_web").await?;
    ensure_default_dictionary(db_service, "fingerprint_rule", "builtin_web_fingerprint_rules")
        .await?;
    ensure_default_dictionary(db_service, "poc_rule", "builtin_safe_poc_rules").await?;

    upsert_builtin_plugin(
        db_service,
        TrafficPluginMetadata {
            id: "tech_fingerprinter".to_string(),
            name: "Technology Fingerprinter".to_string(),
            version: "2.0.0".to_string(),
            author: Some("Sentinel AI".to_string()),
            main_category: "agent".to_string(),
            category: "recon".to_string(),
            description: Some("Dictionary-driven web fingerprint scanner with structured matchers and version extraction".to_string()),
            default_severity: "info".to_string(),
            tags: vec!["fingerprint".to_string(), "web".to_string(), "recon".to_string()],
        },
        TECH_FINGERPRINTER_CODE,
    )
    .await?;

    upsert_builtin_plugin(
        db_service,
        TrafficPluginMetadata {
            id: "sensitive_file_scanner".to_string(),
            name: "Sensitive File Scanner".to_string(),
            version: "1.0.0".to_string(),
            author: Some("Sentinel AI".to_string()),
            main_category: "agent".to_string(),
            category: "risk".to_string(),
            description: Some("Scan web assets for exposed sensitive files using dictionary rules".to_string()),
            default_severity: "medium".to_string(),
            tags: vec!["risk".to_string(), "file".to_string(), "exposure".to_string()],
        },
        SENSITIVE_FILE_SCANNER_CODE,
    )
    .await?;

    upsert_builtin_plugin(
        db_service,
        TrafficPluginMetadata {
            id: "risk_scanner".to_string(),
            name: "Risk Scanner".to_string(),
            version: "2.0.0".to_string(),
            author: Some("Sentinel AI".to_string()),
            main_category: "agent".to_string(),
            category: "risk".to_string(),
            description: Some("Run safe dictionary-driven risk verification rules with variables, preconditions, and chained requests".to_string()),
            default_severity: "medium".to_string(),
            tags: vec!["risk".to_string(), "poc".to_string(), "verification".to_string()],
        },
        RISK_SCANNER_CODE,
    )
    .await?;

    Ok(())
}

async fn ensure_default_dictionary(
    db_service: &Arc<DatabaseService>,
    dict_type: &str,
    dictionary_id: &str,
) -> Result<()> {
    let current = db_service.get_config("dictionary_default", dict_type).await?;
    if current.as_deref().unwrap_or_default().trim().is_empty() {
        db_service
            .set_config(
                "dictionary_default",
                dict_type,
                dictionary_id,
                Some("Builtin default dictionary"),
            )
            .await?;
    }
    Ok(())
}

async fn upsert_builtin_plugin(
    db_service: &Arc<DatabaseService>,
    metadata: TrafficPluginMetadata,
    plugin_code: &str,
) -> Result<()> {
    db_service
        .register_traffic_plugin_with_code(&metadata, plugin_code)
        .await?;
    db_service
        .update_traffic_plugin_enabled(&metadata.id, true)
        .await?;
    Ok(())
}
