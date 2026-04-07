#!/usr/bin/env node

import fs from 'node:fs/promises';
import path from 'node:path';

function printHelp() {
  console.log(`system-agent-eval-compare

Usage:
  node scripts/system-agent-eval-compare.mjs --eval eval-report.json --findings findings.json

Description:
  Compare the structured output from system-agent-eval with a findings snapshot
  exported from Sentinel, then classify each scenario as TP/FP/FN/TN.

Accepted findings snapshot shapes:
  - [ ...findings ]
  - { findings: [ ... ] }
  - { data: [ ... ] }
`);
}

function getArgValue(flag) {
  const index = process.argv.indexOf(flag);
  if (index === -1) {
    return '';
  }
  return process.argv[index + 1] || '';
}

function safeParseJson(raw, sourcePath) {
  try {
    return JSON.parse(raw);
  } catch (error) {
    throw new Error(`Failed to parse JSON from ${sourcePath}: ${error.message}`);
  }
}

async function readJson(inputPath) {
  const resolvedPath = path.resolve(process.cwd(), inputPath);
  const raw = await fs.readFile(resolvedPath, 'utf8');
  return safeParseJson(raw, resolvedPath);
}

function normalizeFindings(source) {
  if (Array.isArray(source)) {
    return source;
  }
  if (Array.isArray(source?.findings)) {
    return source.findings;
  }
  if (Array.isArray(source?.data)) {
    return source.data;
  }
  return [];
}

function tryGetPathname(urlLike) {
  if (typeof urlLike !== 'string' || urlLike.trim().length === 0) {
    return '';
  }
  try {
    return new URL(urlLike).pathname || '';
  } catch {
    return urlLike.startsWith('/') ? urlLike : '';
  }
}

function normalizeFinding(finding) {
  return {
    id: String(finding?.id || ''),
    title: String(finding?.title || ''),
    url: String(finding?.url || ''),
    path: tryGetPathname(finding?.url || ''),
    status: String(finding?.status || ''),
    analysisStage: String(finding?.analysisStage || ''),
    severity: String(finding?.severity || ''),
    vulnType: String(finding?.vuln_type || finding?.vulnType || ''),
  };
}

function isFormalStage(stage, status) {
  return (
    stage === 'formal_open' ||
    stage === 'verified' ||
    stage === 'fixed' ||
    status === 'open' ||
    status === 'reviewed' ||
    status === 'fixed'
  );
}

function isCandidateStage(stage, status) {
  return stage === 'hypothesis' || status === 'candidate' || status === 'triaging';
}

function formatFindingBadge(finding) {
  const stage = finding.analysisStage || finding.status || 'unknown';
  const vulnType = finding.vulnType || 'unknown';
  return `${finding.path || finding.url} [${vulnType}/${stage}]`;
}

function classifyScenario(expectation, matchedFindings) {
  const candidateFindings = matchedFindings.filter(finding =>
    isCandidateStage(finding.analysisStage, finding.status),
  );
  const formalFindings = matchedFindings.filter(finding =>
    isFormalStage(finding.analysisStage, finding.status),
  );

  if (expectation === 'negative_control') {
    if (formalFindings.length > 0) {
      return {
        result: 'FP',
        note: '负样本命中了正式漏洞。',
      };
    }
    if (candidateFindings.length > 0) {
      return {
        result: 'FP-CANDIDATE',
        note: '负样本触发了候选结果，需要继续压误报。',
      };
    }
    return {
      result: 'TN',
      note: '负样本未命中候选或正式漏洞。',
    };
  }

  if (formalFindings.length > 0) {
    return {
      result: 'TP-VERIFIED',
      note: '候选场景命中了正式漏洞或已验证结果。',
    };
  }
  if (candidateFindings.length > 0) {
    return {
      result: 'TP-CANDIDATE',
      note: '候选场景至少形成了候选待验证结果。',
    };
  }
  return {
    result: 'FN',
    note: '候选场景没有对应结果，属于漏报。',
  };
}

function toMarkdownTable(rows) {
  const header = [
    '| 场景 | 预期 | 判定 | 说明 | 命中结果 |',
    '| --- | --- | --- | --- | --- |',
  ];
  const body = rows.map(row => {
    const matches = row.matches.length > 0 ? row.matches.join('<br/>') : '-';
    return `| ${row.title} | ${row.expectation} | ${row.result} | ${row.note} | ${matches} |`;
  });
  return [...header, ...body].join('\n');
}

async function main() {
  if (process.argv.includes('--help')) {
    printHelp();
    return;
  }

  const evalPath = getArgValue('--eval');
  const findingsPath = getArgValue('--findings');
  if (!evalPath || !findingsPath) {
    printHelp();
    process.exitCode = 1;
    return;
  }

  const evalReport = await readJson(evalPath);
  const findingsSource = await readJson(findingsPath);
  const findings = normalizeFindings(findingsSource).map(normalizeFinding);

  const rows = (evalReport?.scenarios || []).map(scenario => {
    const scenarioPaths = (scenario.requests || [])
      .map(request => tryGetPathname(request?.path || request?.url || ''))
      .filter(Boolean);
    const matchedFindings = findings.filter(finding => scenarioPaths.includes(finding.path));
    const classification = classifyScenario(scenario.expectation, matchedFindings);
    return {
      id: scenario.id,
      title: scenario.title,
      expectation: scenario.expectation,
      result: classification.result,
      note: classification.note,
      matches: matchedFindings.map(formatFindingBadge),
    };
  });

  const summary = rows.reduce(
    (acc, row) => {
      acc[row.result] = (acc[row.result] || 0) + 1;
      return acc;
    },
    {},
  );

  const report = {
    comparedAt: new Date().toISOString(),
    evalScenarioCount: rows.length,
    findingCount: findings.length,
    summary,
    rows,
    markdownTable: toMarkdownTable(rows),
  };

  console.log(JSON.stringify(report, null, 2));
}

main().catch(error => {
  console.error('system-agent-eval-compare failed', error);
  process.exitCode = 1;
});
