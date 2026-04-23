import type { HelpCenterContent } from './helpCenterContent'

export interface HelpCenterNavNode {
  id: string
  label: string
  children: HelpCenterNavNode[]
}

export function buildHelpCenterNavigation(content: HelpCenterContent): HelpCenterNavNode[] {
  return [
    {
      id: 'overview',
      label: content.overviewTitle,
      children: [],
    },
    {
      id: 'workflow',
      label: content.workflowTitle,
      children: content.workflowSteps.map(step => ({
        id: step.id,
        label: step.title,
        children: [],
      })),
    },
    ...content.catalogSections.map(section => ({
      id: section.id,
      label: section.title,
      children: section.entries.map(entry => ({
        id: entry.id,
        label: entry.title,
        children: [],
      })),
    })),
    {
      id: 'faq',
      label: content.faqTitle,
      children: content.faqs.map(faq => ({
        id: faq.id,
        label: faq.question,
        children: [],
      })),
    },
  ]
}

export function flattenHelpCenterNavigation(tree: HelpCenterNavNode[]): string[] {
  return tree.flatMap(node => [node.id, ...flattenHelpCenterNavigation(node.children)])
}
