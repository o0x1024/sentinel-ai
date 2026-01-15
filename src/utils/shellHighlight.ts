import { shell } from '@codemirror/legacy-modes/mode/shell'

// Token type to CSS class mapping (matching CodeMirror's oneDark theme)
const tokenClassMap: Record<string, string> = {
  'keyword': 'cm-keyword',
  'operator': 'cm-operator',
  'string': 'cm-string',
  'string2': 'cm-string-2',
  'comment': 'cm-comment',
  'variable': 'cm-variable',
  'variable-2': 'cm-variable-2',
  'variable-3': 'cm-variable-3',
  'def': 'cm-def',
  'atom': 'cm-atom',
  'number': 'cm-number',
  'property': 'cm-property',
  'qualifier': 'cm-qualifier',
  'type': 'cm-type',
  'builtin': 'cm-builtin',
  'bracket': 'cm-bracket',
  'tag': 'cm-tag',
  'attribute': 'cm-attribute',
  'meta': 'cm-meta',
  'link': 'cm-link',
}

interface Token {
  type: string | null
  text: string
  start: number
  end: number
}

/**
 * Tokenize shell command using CodeMirror's legacy mode
 */
function tokenizeShellCommand(command: string): Token[] {
  const tokens: Token[] = []
  
  // Use legacy mode tokenizer
  const mode = shell
  const state = mode.startState?.(0) || {}
  
  let pos = 0
  const lines = command.split('\n')
  
  for (let lineIdx = 0; lineIdx < lines.length; lineIdx++) {
    const line = lines[lineIdx]
    const linePos = 0
    
    // Create a stream-like object for the mode
    const stream = {
      string: line,
      pos: 0,
      start: 0,
      lineStart: pos,
      tabSize: 4,
      indentUnit: 2,
      lastColumnPos: 0,
      lastColumnValue: 0,
      
      eol() { return this.pos >= this.string.length },
      sol() { return this.pos === 0 },
      peek() { return this.string.charAt(this.pos) || undefined },
      next() {
        if (this.pos < this.string.length) {
          return this.string.charAt(this.pos++)
        }
      },
      eat(match: string | RegExp | ((char: string) => boolean)) {
        const ch = this.string.charAt(this.pos)
        let ok: boolean
        if (typeof match === 'string') {
          ok = ch === match
        } else if (match instanceof RegExp) {
          ok = match.test(ch)
        } else {
          ok = match(ch)
        }
        if (ok) {
          this.pos++
          return ch
        }
      },
      eatWhile(match: string | RegExp | ((char: string) => boolean)) {
        const start = this.pos
        while (this.eat(match)) {}
        return this.pos > start
      },
      eatSpace() {
        const start = this.pos
        while (/\s/.test(this.string.charAt(this.pos))) this.pos++
        return this.pos > start
      },
      skipToEnd() { this.pos = this.string.length },
      skipTo(ch: string) {
        const found = this.string.indexOf(ch, this.pos)
        if (found > -1) {
          this.pos = found
          return true
        }
        return false
      },
      match(pattern: string | RegExp, consume?: boolean, caseInsensitive?: boolean) {
        if (typeof pattern === 'string') {
          const cased = (str: string) => caseInsensitive ? str.toLowerCase() : str
          const substr = this.string.substr(this.pos, pattern.length)
          if (cased(substr) === cased(pattern)) {
            if (consume !== false) this.pos += pattern.length
            return true
          }
        } else {
          const match = this.string.slice(this.pos).match(pattern)
          if (match && match.index === 0) {
            if (consume !== false) this.pos += match[0].length
            return match
          }
        }
        return null
      },
      backUp(n: number) { this.pos -= n },
      column() { return this.pos },
      indentation() {
        const match = this.string.match(/^\s*/)
        return match ? match[0].length : 0
      },
      current() { return this.string.slice(this.start, this.pos) }
    } as any
    
    while (!stream.eol()) {
      stream.start = stream.pos
      const tokenType = mode.token(stream, state)
      const tokenText = stream.current()
      
      if (tokenText) {
        tokens.push({
          type: tokenType,
          text: tokenText,
          start: pos + stream.start,
          end: pos + stream.pos
        })
      }
    }
    
    pos += line.length + 1 // +1 for newline
  }
  
  return tokens
}

/**
 * Highlight shell command and return HTML string with CSS classes
 */
export function highlightShellCommand(command: string): string {
  if (!command) return ''
  
  try {
    const tokens = tokenizeShellCommand(command)
    let html = ''
    let lastEnd = 0
    
    for (const token of tokens) {
      // Add any text between tokens (shouldn't happen, but just in case)
      if (token.start > lastEnd) {
        html += escapeHtml(command.slice(lastEnd, token.start))
      }
      
      // Add token with appropriate class
      const text = escapeHtml(token.text)
      if (token.type) {
        const cssClass = tokenClassMap[token.type] || 'cm-' + token.type
        html += `<span class="${cssClass}">${text}</span>`
      } else {
        html += text
      }
      
      lastEnd = token.end
    }
    
    // Add any remaining text
    if (lastEnd < command.length) {
      html += escapeHtml(command.slice(lastEnd))
    }
    
    return html
  } catch (error) {
    console.error('Error highlighting shell command:', error)
    return escapeHtml(command)
  }
}

/**
 * Escape HTML special characters
 */
function escapeHtml(text: string): string {
  const div = document.createElement('div')
  div.textContent = text
  return div.innerHTML
}

/**
 * Get CSS styles for shell syntax highlighting (matching oneDark theme)
 */
export function getShellHighlightStyles(): string {
  return `
    .cm-keyword { color: #c678dd; } /* Commands and keywords */
    .cm-operator { color: #56b6c2; } /* Operators like |, >, <, & */
    .cm-string { color: #98c379; } /* Single-quoted strings */
    .cm-string-2 { color: #98c379; } /* Double-quoted strings */
    .cm-comment { color: #5c6370; font-style: italic; } /* Comments */
    .cm-variable { color: #e06c75; } /* Variables like $VAR */
    .cm-variable-2 { color: #e5c07b; } /* Special variables */
    .cm-variable-3 { color: #d19a66; } /* Other variables */
    .cm-def { color: #61afef; } /* Function definitions */
    .cm-atom { color: #d19a66; } /* Atoms (true, false, null) */
    .cm-number { color: #d19a66; } /* Numbers */
    .cm-property { color: #61afef; } /* Properties */
    .cm-qualifier { color: #e06c75; } /* Qualifiers */
    .cm-type { color: #e5c07b; } /* Types */
    .cm-builtin { color: #e5c07b; } /* Built-in commands */
    .cm-bracket { color: #abb2bf; } /* Brackets */
    .cm-tag { color: #e06c75; } /* Tags */
    .cm-attribute { color: #d19a66; } /* Attributes */
    .cm-meta { color: #61afef; } /* Meta information */
    .cm-link { color: #61afef; text-decoration: underline; } /* Links */
  `
}
