import { describe, expect, it } from 'vitest'
import { getHelpCenterContent } from './helpCenterContent'
import { buildHelpCenterNavigation, flattenHelpCenterNavigation } from './helpCenterNavigation'

describe('helpCenterNavigation', () => {
  it('builds a collapsed tree from localized help content', () => {
    const tree = buildHelpCenterNavigation(getHelpCenterContent('zh-CN'))

    expect(tree.map(node => node.id)).toEqual([
      'overview',
      'workflow',
      'global-controls',
      'core-features',
      'tools-and-content',
      'system-and-collaboration',
      'faq',
    ])
    expect(tree[2]?.children.map(node => node.id)).toEqual([
      'feature-global-search',
      'feature-help-guide',
      'feature-immersive-drill',
      'feature-notification-inbox',
      'feature-language-theme',
    ])
    expect(tree[3]?.children.map(node => node.id)).toContain('feature-traffic-analysis')
    expect(tree[3]?.children.map(node => node.id)).toContain('feature-traffic-oast')
    expect(tree[3]?.children.map(node => node.id)).toContain('feature-security-workbench')
    expect(tree[3]?.children.map(node => node.id)).toContain('feature-workflow-execution')
    expect(tree[3]?.children.map(node => node.id)).toContain('feature-ai-assistant-sessions')
    expect(tree[4]?.children.map(node => node.id)).toContain('feature-plugin-lifecycle')
    expect(tree[4]?.children.map(node => node.id)).toContain('feature-rag-ingest-query')
    expect(tree[4]?.children.map(node => node.id)).toContain('feature-mcp-servers')
    expect(tree[6]?.children[0]?.id).toBe('faq-traffic-before-ai')
  })

  it('flattens every section id for scroll tracking', () => {
    const ids = flattenHelpCenterNavigation(buildHelpCenterNavigation(getHelpCenterContent('en')))

    expect(ids).toContain('overview')
    expect(ids).toContain('feature-traffic-oast')
    expect(ids).toContain('feature-security-findings')
    expect(ids).toContain('feature-workflow-canvas')
    expect(ids).toContain('feature-ai-assistant-governance')
    expect(ids).toContain('feature-plugin-store-review')
    expect(ids).toContain('feature-rag-collections')
    expect(ids).toContain('feature-builtin-skills')
    expect(ids).toContain('feature-workflow-studio')
    expect(ids).toContain('feature-plugin-management')
    expect(ids).toContain('feature-performance-monitor')
  })
})
