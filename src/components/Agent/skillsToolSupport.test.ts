import { describe, expect, it } from 'vitest'

import {
  extractSkillInvokeMarkdown,
  extractSkillsToolErrorMessage,
  isSkillActivityMessage,
  isSkillsInvokeToolSuccess,
  isSkillsReadFileSuccess,
  isSkillsToolName,
  isSkillsToolSuccess,
  parseSkillFileBlocks,
  parseSkillsToolResult,
  resolveSkillsDisplayFromMessage,
  resolveSkillFileSections,
} from './skillsToolSupport'

describe('skillsToolSupport', () => {
  it('detects skills tool name', () => {
    expect(isSkillsToolName('skills')).toBe(true)
    expect(isSkillsToolName('Skills')).toBe(true)
    expect(isSkillsToolName('shell')).toBe(false)
  })

  it('parses invoke result without referenced files (on-demand loading)', () => {
    const parsed = parseSkillsToolResult({
      action: 'invoke',
      skill: {
        id: 'code-audit',
        name: 'Code Audit',
        description: 'Audit code',
      },
      content: 'Skill loaded: Code Audit\n\n<skill>',
    })

    expect(parsed).toEqual({
      action: 'invoke',
      skillId: 'code-audit',
      skillName: 'Code Audit',
      description: 'Audit code',
      content: 'Skill loaded: Code Audit\n\n<skill>',
      referencedFiles: [],
      warnings: [],
    })
  })

  it('parses invoke result warnings', () => {
    const parsed = parseSkillsToolResult({
      action: 'invoke',
      skill: { id: 'penetration-tester', name: 'penetration-tester' },
      content: 'Skill loaded: penetration-tester\n\n<skill>',
      warnings: ['Referenced file not found: scripts/missing.py'],
    })

    expect(parsed?.warnings).toEqual(['Referenced file not found: scripts/missing.py'])
  })

  it('parses fork result', () => {
    const parsed = parseSkillsToolResult({
      action: 'fork',
      skill: { id: 'review', name: 'Review' },
      content: 'Skill completed in forked execution.',
    })

    expect(parsed?.action).toBe('fork')
    expect(parsed?.skillId).toBe('review')
  })

  it('falls back to tool args when result is not ready', () => {
    const parsed = resolveSkillsDisplayFromMessage({
      toolName: 'skills',
      toolArgs: { skill: 'code-audit' },
      toolResult: undefined,
    })

    expect(parsed).toEqual({
      action: 'unknown',
      skillId: 'code-audit',
      skillName: 'code-audit',
      referencedFiles: [],
      warnings: [],
    })
  })

  it('recognizes skill activity system messages', () => {
    expect(
      isSkillActivityMessage({
        type: 'system',
        metadata: { kind: 'skill_loaded', skill_id: 'x' },
      }),
    ).toBe(true)
    expect(
      isSkillActivityMessage({
        type: 'system',
        metadata: { kind: 'skill_forked', skill_id: 'x' },
      }),
    ).toBe(true)
  })

  it('extracts skills tool error text from rig tool result payload', () => {
    const message = extractSkillsToolErrorMessage(undefined, [
      {
        type: 'text',
        text: 'Toolset error: Tool execution failed: Skills operation failed: Skill not found: .js',
      },
    ])

    expect(message).toBe('Skills operation failed: Skill not found: .js')
  })

  it('treats inline invoke payload as success even with error-like prose', () => {
    const wrapped = [
      {
        type: 'text',
        text: JSON.stringify({
          action: 'invoke',
          content:
            'Skill loaded: penetration-tester\n\n<skill>\n<name>penetration-tester</name>\ntimed out error failed',
        }),
      },
    ]

    expect(isSkillsInvokeToolSuccess(wrapped)).toBe(true)
    expect(isSkillsToolSuccess(wrapped)).toBe(true)
    expect(extractSkillsToolErrorMessage(undefined, wrapped)).toBeUndefined()
  })

  it('treats read_file result as success', () => {
    const result = {
      action: 'read_file',
      skill: { id: 'penetration-tester', name: 'penetration-tester' },
      content: '<file path="references/attack_vectors.md">\n# Attack Vectors\n</file>',
      referenced_files: ['references/attack_vectors.md'],
    }

    expect(isSkillsReadFileSuccess(result)).toBe(true)
    expect(isSkillsToolSuccess(result)).toBe(true)
  })

  it('parses helper file blocks and invoke markdown', () => {
    const fileContent = '<file path="references/guide.md">\n# Guide\n\nHello\n</file>'
    expect(parseSkillFileBlocks(fileContent)).toEqual([
      { path: 'references/guide.md', content: '# Guide\n\nHello' },
    ])

    const invokeContent =
      'Skill loaded: audit\n\n<skill>\n# Audit Skill\n\nRun checks.\n</skill>'
    expect(extractSkillInvokeMarkdown(invokeContent)).toContain('# Audit Skill')
    expect(resolveSkillFileSections(invokeContent, ['SKILL.md'])).toEqual([
      { path: 'SKILL.md', content: expect.stringContaining('# Audit Skill') },
    ])
  })
})
