import DOMPurify from 'dompurify'
import { marked } from 'marked'

const MARKDOWN_OPTIONS = {
  gfm: true,
  breaks: true,
  async: false as const,
}

export function renderBountyKnowledgeMarkdown(content: string) {
  const rawHtml = marked.parse(content || '', MARKDOWN_OPTIONS) as string
  return DOMPurify.sanitize(rawHtml, {
    ADD_DATA_URI_TAGS: ['img'],
    ALLOWED_URI_REGEXP: /^(?:(?:(?:f|ht)tps?|mailto|tel|callto|sms|cid|xmpp):|[^a-z]|[a-z+.\-]+(?:[^a-z+.\-:]|$)|data:image\/(?:png|jpeg|jpg|gif|webp);base64,)/i,
  })
}

export function formatBountyKnowledgeMarkdownSnippet(content: string) {
  const html = renderBountyKnowledgeMarkdown(content)
  const text = html
    .replace(/<[^>]+>/g, ' ')
    .replace(/&nbsp;/g, ' ')
    .replace(/&amp;/g, '&')
    .replace(/&lt;/g, '<')
    .replace(/&gt;/g, '>')
    .replace(/&quot;/g, '"')
    .replace(/&#39;/g, "'")
    .replace(/\s+/g, ' ')
    .trim()

  return text
}
