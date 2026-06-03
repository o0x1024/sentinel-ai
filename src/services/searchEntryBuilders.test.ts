import { describe, expect, it } from 'vitest'
import {
  createKnowledgeDocumentSearchEntries,
  createPluginSearchEntries,
  createScanTaskSearchEntries,
  createWorkflowDefinitionSearchEntries,
  createWorkflowRunSearchEntries,
} from '@/services/searchEntryBuilders'

describe('searchEntryBuilders', () => {
  it('builds scan task search entries', () => {
    const [entry] = createScanTaskSearchEntries([{
      id: 'task-1',
      name: 'Nightly Scan',
      target: 'https://example.com',
      status: 'running',
    }])

    expect(entry.category).toBe('task')
    expect(entry.query).toEqual({ taskId: 'task-1' })
    expect(entry.keywords).toContain('running')
  })

  it('builds workflow entries for definitions and runs', () => {
    const [definition] = createWorkflowDefinitionSearchEntries([{
      id: 'wf-1',
      name: 'HTTP Recon',
      description: 'Collect HTTP assets',
      tags: ['recon'],
    }])
    const [run] = createWorkflowRunSearchEntries([{
      id: 'run-1',
      workflow_name: 'HTTP Recon',
      status: 'completed',
    }])

    expect(definition.category).toBe('workflow')
    expect(definition.query).toEqual({ workflowId: 'wf-1' })
    expect(run.query).toEqual({ execution_id: 'run-1' })
  })

  it('builds knowledge and plugin search entries', () => {
    const [document] = createKnowledgeDocumentSearchEntries([{
      id: 'doc-1',
      file_name: 'incident-playbook.md',
      collection_name: 'Playbooks',
      chunk_count: 12,
    }])
    const [plugin] = createPluginSearchEntries([{
      plugin_id: 'plugin.http.recon',
      title: 'HTTP Recon Plugin',
      status: 'PendingReview',
      author: 'Sentinel',
    }])

    expect(document.category).toBe('document')
    expect(document.keywords).toContain('Playbooks')
    expect(plugin.category).toBe('plugin')
    expect(plugin.keywords).toContain('PendingReview')
  })

})
