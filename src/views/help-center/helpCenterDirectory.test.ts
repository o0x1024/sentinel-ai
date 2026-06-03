import { describe, expect, it } from 'vitest'
import { getHelpCenterContent } from './helpCenterContent'
import { buildHelpCenterDirectory } from './helpCenterDirectory'

describe('helpCenterDirectory', () => {
  it('builds a selectable document map for the help center tree', () => {
    const directory = buildHelpCenterDirectory(getHelpCenterContent('zh-CN'))

    expect(directory.documents.overview.kind).toBe('overview')
    expect(directory.documents.workflow.links?.[0]?.id).toBe('workflow-capture')
    expect(directory.documents['global-controls'].kind).toBe('catalog-section')
    expect(directory.documents['feature-global-search'].route).toBe('/search，或 Ctrl/Cmd + K')
    expect(directory.documents['feature-traffic-oast'].detailSections?.length).toBeGreaterThan(10)
    expect(directory.documents['feature-ai-assistant-sessions'].detailSections?.length).toBe(3)
    expect(directory.documents['feature-security-workbench'].detailSections?.length).toBe(3)
    expect(directory.documents['feature-workflow-execution'].detailSections?.length).toBe(3)
    expect(directory.documents['feature-mcp-servers'].detailSections?.length).toBe(3)
    expect(directory.documents['feature-plugin-lifecycle'].detailSections?.length).toBe(3)
    expect(directory.documents['feature-rag-collections'].detailSections?.length).toBe(3)
    expect(directory.documents['feature-bug-bounty-pro'].detailSections?.[0]?.imageSrc).toBe(
      '/community/wechat-qrcode.png',
    )
    expect(directory.documents['faq-traffic-before-ai'].kind).toBe('faq-entry')
  })

  it('tracks each child document under its parent directory', () => {
    const directory = buildHelpCenterDirectory(getHelpCenterContent('en'))

    expect(directory.parentById['workflow-reproduce']).toBe('workflow')
    expect(directory.parentById['feature-traffic-oast']).toBe('core-features')
    expect(directory.parentById['feature-security-findings']).toBe('core-features')
    expect(directory.parentById['feature-workflow-canvas']).toBe('core-features')
    expect(directory.parentById['feature-bug-bounty-pro']).toBe('core-features')
    expect(directory.parentById['feature-ai-assistant-governance']).toBe('core-features')
    expect(directory.parentById['feature-plugin-store-review']).toBe('tools-and-content')
    expect(directory.parentById['feature-rag-ingest-query']).toBe('tools-and-content')
    expect(directory.parentById['feature-builtin-skills']).toBe('tools-and-content')
    expect(directory.parentById['feature-plugin-management']).toBe('tools-and-content')
    expect(directory.parentById['faq-settings-boundary']).toBe('faq')
  })
})
