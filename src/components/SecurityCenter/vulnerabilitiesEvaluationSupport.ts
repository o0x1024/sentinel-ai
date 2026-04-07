import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { readTextFile } from '@tauri-apps/plugin-fs';

export interface EvaluationComparisonRow {
  id: string;
  title: string;
  expectation: string;
  result: string;
  note: string;
  matches: string[];
}

export interface EvaluationComparisonSummary {
  comparedAt: string;
  evalScenarioCount: number;
  findingCount: number;
  summary: Record<string, number>;
  rows: EvaluationComparisonRow[];
}

const EVALUATION_CONFIG_CATEGORY = 'security_center';
const EVALUATION_CONFIG_KEY = 'latest_vulnerability_evaluation_comparison';
const EVALUATION_HISTORY_CONFIG_KEY = 'vulnerability_evaluation_comparison_history';

function normalizeRow(row: any): EvaluationComparisonRow {
  return {
    id: String(row?.id || ''),
    title: String(row?.title || ''),
    expectation: String(row?.expectation || ''),
    result: String(row?.result || ''),
    note: String(row?.note || ''),
    matches: Array.isArray(row?.matches) ? row.matches.map((item: any) => String(item)) : [],
  };
}

function normalizeComparison(parsed: any): EvaluationComparisonSummary {
  return {
    comparedAt: String(parsed?.comparedAt || ''),
    evalScenarioCount: Number(parsed?.evalScenarioCount || 0),
    findingCount: Number(parsed?.findingCount || 0),
    summary: parsed?.summary && typeof parsed.summary === 'object' ? parsed.summary : {},
    rows: Array.isArray(parsed?.rows) ? parsed.rows.map(normalizeRow) : [],
  };
}

export async function loadPersistedEvaluationComparison(): Promise<EvaluationComparisonSummary | null> {
  const items = await invoke<Array<{ key: string; value: string }>>('get_config', {
    request: {
      category: EVALUATION_CONFIG_CATEGORY,
      key: EVALUATION_CONFIG_KEY,
    },
  });
  const value = items?.[0]?.value;
  if (!value) {
    return null;
  }
  return normalizeComparison(JSON.parse(value));
}

export async function persistEvaluationComparison(
  comparison: EvaluationComparisonSummary,
): Promise<void> {
  await invoke('set_config', {
    category: EVALUATION_CONFIG_CATEGORY,
    key: EVALUATION_CONFIG_KEY,
    value: JSON.stringify(comparison),
  });
}

export async function loadPersistedEvaluationComparisonHistory(): Promise<EvaluationComparisonSummary[]> {
  const items = await invoke<Array<{ key: string; value: string }>>('get_config', {
    request: {
      category: EVALUATION_CONFIG_CATEGORY,
      key: EVALUATION_HISTORY_CONFIG_KEY,
    },
  });
  const value = items?.[0]?.value;
  if (!value) {
    return [];
  }
  const parsed = JSON.parse(value);
  return Array.isArray(parsed) ? parsed.map(normalizeComparison) : [];
}

export async function persistEvaluationComparisonHistory(
  history: EvaluationComparisonSummary[],
): Promise<void> {
  await invoke('set_config', {
    category: EVALUATION_CONFIG_CATEGORY,
    key: EVALUATION_HISTORY_CONFIG_KEY,
    value: JSON.stringify(history),
  });
}

export async function clearPersistedEvaluationComparison(): Promise<void> {
  await invoke('delete_config', {
    category: EVALUATION_CONFIG_CATEGORY,
    key: EVALUATION_CONFIG_KEY,
  });
}

export async function clearPersistedEvaluationComparisonHistory(): Promise<void> {
  await invoke('delete_config', {
    category: EVALUATION_CONFIG_CATEGORY,
    key: EVALUATION_HISTORY_CONFIG_KEY,
  });
}

export async function importEvaluationComparison(): Promise<EvaluationComparisonSummary | null> {
  const selected = await open({
    directory: false,
    multiple: false,
    filters: [{ name: 'JSON', extensions: ['json'] }],
    title: '导入评测对照结果',
  });
  if (!selected) {
    return null;
  }

  const content = await readTextFile(selected as string);
  const parsed = JSON.parse(content);
  return normalizeComparison(parsed);
}
