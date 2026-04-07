import { invoke } from '@tauri-apps/api/core';
import { save } from '@tauri-apps/plugin-dialog';
import { writeTextFile } from '@tauri-apps/plugin-fs';

export interface FindingSnapshotRecord {
  id: string;
  title: string;
  url: string;
  status: string;
  analysisStage?: string;
  severity: string;
  vulnType: string;
  pluginId: string;
}

export interface ExportFindingSnapshotOptions {
  severityFilter: string | null;
  statusFilter: string | null;
  statusFilters: string[] | null;
  analysisStageFilters: string[] | null;
  lifecycleView: string;
  search: string;
  semanticSourceFilter: string | null;
  hypothesisRiskTypeFilter: string | null;
  hypothesisRiskTypeFilters: string[] | null;
}

function normalizeSnapshotRecord(item: any): FindingSnapshotRecord {
  return {
    id: String(item?.id || ''),
    title: String(item?.title || ''),
    url: String(item?.url || ''),
    status: String(item?.status || ''),
    analysisStage: item?.analysisStage ? String(item.analysisStage) : '',
    severity: String(item?.severity || ''),
    vulnType: String(item?.vuln_type || item?.vulnType || ''),
    pluginId: String(item?.plugin_id || item?.pluginId || ''),
  };
}

async function fetchAllFindings(
  severityFilter: string | null,
  statusFilter: string | null,
  statusFilters: string[] | null,
  analysisStageFilters: string[] | null,
  search: string,
  semanticSourceFilter: string | null,
  hypothesisRiskTypeFilter: string | null,
  hypothesisRiskTypeFilters: string[] | null,
) {
  const countResponse = await invoke<any>('count_findings', {
    severityFilter,
    statusFilter,
    statusFilters,
    analysisStageFilters,
    search,
    semanticSourceFilter,
    hypothesisRiskTypeFilter,
    hypothesisRiskTypeFilters,
  });
  const total = countResponse?.success ? Number(countResponse.data || 0) : 0;
  if (total <= 0) {
    return [];
  }

  const pageSize = 200;
  const pages = Math.ceil(total / pageSize);
  const items: any[] = [];

  for (let page = 0; page < pages; page += 1) {
    // eslint-disable-next-line no-await-in-loop
    const response = await invoke<any>('list_findings', {
      limit: pageSize,
      offset: page * pageSize,
      severityFilter,
      statusFilter,
      statusFilters,
      analysisStageFilters,
      search,
      semanticSourceFilter,
      hypothesisRiskTypeFilter,
      hypothesisRiskTypeFilters,
    });
    if (response?.success && Array.isArray(response.data)) {
      items.push(...response.data);
    }
  }

  return items;
}

export async function exportFindingSnapshot(options: ExportFindingSnapshotOptions) {
  const rawFindings = await fetchAllFindings(
    options.severityFilter,
    options.statusFilter,
    options.statusFilters,
    options.analysisStageFilters,
    options.search,
    options.semanticSourceFilter,
    options.hypothesisRiskTypeFilter,
    options.hypothesisRiskTypeFilters,
  );
  const findings = rawFindings.map(normalizeSnapshotRecord);

  const selected = await save({
    title: '导出评测快照',
    defaultPath: `system-agent-findings-snapshot-${new Date().toISOString().slice(0, 19).replace(/[:T]/g, '-')}.json`,
    filters: [
      {
        name: 'JSON',
        extensions: ['json'],
      },
    ],
  });

  if (!selected) {
    return false;
  }

  const payload = {
    exportedAt: new Date().toISOString(),
    filters: {
      severity: options.severityFilter,
      status: options.statusFilter,
      statusFilters: options.statusFilters,
      analysisStageFilters: options.analysisStageFilters,
      lifecycleView: options.lifecycleView,
      search: options.search,
      semanticSourceFilter: options.semanticSourceFilter,
      hypothesisRiskTypeFilter: options.hypothesisRiskTypeFilter,
      hypothesisRiskTypeFilters: options.hypothesisRiskTypeFilters,
    },
    total: findings.length,
    findings,
  };

  await writeTextFile(selected, JSON.stringify(payload, null, 2));
  return true;
}
