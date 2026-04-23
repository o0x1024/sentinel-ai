import type {
  HelpCenterCatalogSection,
  HelpCenterContent,
  HelpCenterFaq,
  HelpCenterFeatureDetailSection,
  HelpCenterFeatureEntry,
  HelpCenterStat,
  HelpCenterWorkflowStep,
} from './helpCenterContent'
import { buildHelpCenterNavigation, type HelpCenterNavNode } from './helpCenterNavigation'

export type HelpCenterDocumentKind =
  | 'overview'
  | 'workflow'
  | 'workflow-step'
  | 'catalog-section'
  | 'feature-entry'
  | 'faq'
  | 'faq-entry'

export interface HelpCenterDocumentLink {
  id: string
  title: string
  description?: string
  icon?: string
  meta?: string
}

export interface HelpCenterDirectoryDocument {
  id: string
  kind: HelpCenterDocumentKind
  title: string
  badge: string
  description: string
  parentId: string | null
  icon?: string
  route?: string
  summary?: string
  stats?: HelpCenterStat[]
  workflowSteps?: HelpCenterWorkflowStep[]
  capabilities?: string[]
  operations?: string[]
  detailSections?: HelpCenterFeatureDetailSection[]
  links?: HelpCenterDocumentLink[]
}

export interface HelpCenterDirectoryData {
  navTree: HelpCenterNavNode[]
  documents: Record<string, HelpCenterDirectoryDocument>
  parentById: Record<string, string | null>
}

export function buildHelpCenterDirectory(content: HelpCenterContent): HelpCenterDirectoryData {
  const navTree = buildHelpCenterNavigation(content)
  const documents: Record<string, HelpCenterDirectoryDocument> = {
    overview: {
      id: 'overview',
      kind: 'overview',
      title: content.overviewTitle,
      badge: content.badge,
      description: content.description,
      parentId: null,
      stats: content.stats,
      links: [
        {
          id: 'workflow',
          title: content.workflowTitle,
          description: content.workflowDescription,
        },
        ...content.catalogSections.map(section => ({
          id: section.id,
          title: section.title,
          description: section.description,
          meta: section.badge,
        })),
        {
          id: 'faq',
          title: content.faqTitle,
          description: content.faqDescription,
        },
      ],
    },
    workflow: {
      id: 'workflow',
      kind: 'workflow',
      title: content.workflowTitle,
      badge: content.workflowTitle,
      description: content.workflowDescription,
      parentId: null,
      workflowSteps: content.workflowSteps,
      links: content.workflowSteps.map(step => ({
        id: step.id,
        title: step.title,
        description: step.description,
      })),
    },
    faq: {
      id: 'faq',
      kind: 'faq',
      title: content.faqTitle,
      badge: content.faqTitle,
      description: content.faqDescription,
      parentId: null,
      links: content.faqs.map(faq => ({
        id: faq.id,
        title: faq.question,
        description: faq.answer,
      })),
    },
  }

  const parentById: Record<string, string | null> = {
    overview: null,
    workflow: null,
    faq: null,
  }

  for (const step of content.workflowSteps) {
    documents[step.id] = createWorkflowStepDocument(step, content.workflowTitle)
    parentById[step.id] = 'workflow'
  }

  for (const section of content.catalogSections) {
    documents[section.id] = createCatalogSectionDocument(section)
    parentById[section.id] = null

    for (const entry of section.entries) {
      documents[entry.id] = createFeatureEntryDocument(entry, section)
      parentById[entry.id] = section.id
    }
  }

  for (const faq of content.faqs) {
    documents[faq.id] = createFaqDocument(faq, content.faqTitle)
    parentById[faq.id] = 'faq'
  }

  return {
    navTree,
    documents,
    parentById,
  }
}

function createWorkflowStepDocument(
  step: HelpCenterWorkflowStep,
  workflowTitle: string
): HelpCenterDirectoryDocument {
  return {
    id: step.id,
    kind: 'workflow-step',
    title: step.title,
    badge: workflowTitle,
    description: step.description,
    parentId: 'workflow',
  }
}

function createCatalogSectionDocument(
  section: HelpCenterCatalogSection
): HelpCenterDirectoryDocument {
  return {
    id: section.id,
    kind: 'catalog-section',
    title: section.title,
    badge: section.badge,
    description: section.description,
    parentId: null,
    links: section.entries.map(entry => ({
      id: entry.id,
      title: entry.title,
      description: entry.summary,
      icon: entry.icon,
      meta: entry.route,
    })),
  }
}

function createFeatureEntryDocument(
  entry: HelpCenterFeatureEntry,
  section: HelpCenterCatalogSection
): HelpCenterDirectoryDocument {
  return {
    id: entry.id,
    kind: 'feature-entry',
    title: entry.title,
    badge: section.badge,
    description: entry.summary,
    parentId: section.id,
    icon: entry.icon,
    route: entry.route,
    capabilities: entry.capabilities,
    operations: entry.operations,
    detailSections: entry.detailSections,
  }
}

function createFaqDocument(faq: HelpCenterFaq, faqTitle: string): HelpCenterDirectoryDocument {
  return {
    id: faq.id,
    kind: 'faq-entry',
    title: faq.question,
    badge: faqTitle,
    description: faq.answer,
    parentId: 'faq',
  }
}
