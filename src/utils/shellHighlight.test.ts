import { describe, it, expect } from 'vitest'
import { highlightShellCommand } from './shellHighlight'

describe('shellHighlight', () => {
  it('should highlight basic commands', () => {
    const command = 'ls -la /home'
    const result = highlightShellCommand(command)
    expect(result).toBeTruthy()
    expect(result).toContain('ls')
  })

  it('should highlight strings', () => {
    const command = 'echo "hello world"'
    const result = highlightShellCommand(command)
    expect(result).toBeTruthy()
    expect(result).toContain('cm-string')
  })

  it('should highlight pipes', () => {
    const command = 'cat file.txt | grep pattern'
    const result = highlightShellCommand(command)
    expect(result).toBeTruthy()
    expect(result).toContain('|')
  })

  it('should highlight variables', () => {
    const command = 'echo $HOME'
    const result = highlightShellCommand(command)
    expect(result).toBeTruthy()
    expect(result).toContain('$HOME')
  })

  it('should escape HTML', () => {
    const command = 'echo "<script>alert(1)</script>"'
    const result = highlightShellCommand(command)
    expect(result).not.toContain('<script>')
    expect(result).toContain('&lt;script&gt;')
  })

  it('should handle empty command', () => {
    const result = highlightShellCommand('')
    expect(result).toBe('')
  })

  it('should handle complex CTF command', () => {
    const command = `python3 -c "import base64; print(base64.b64decode('dGVzdA=='))"`
    const result = highlightShellCommand(command)
    expect(result).toBeTruthy()
    expect(result).toContain('python3')
    expect(result).toContain('cm-string')
  })
})
