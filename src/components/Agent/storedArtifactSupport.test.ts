import { describe, expect, it } from 'vitest'

import { buildStoredArtifactViews, extractStoredArtifacts } from './storedArtifactSupport'

describe('storedArtifactSupport', () => {
  it('extracts stored artifacts from structured tool result JSON', () => {
    const artifacts = extractStoredArtifacts(
      JSON.stringify({
        stored_artifacts: [
          {
            slot: 'body',
            path: '/tmp/http_response.txt',
            storage_backend: 'host',
            size: 4096,
            lines: 380,
          },
        ],
      }),
    )

    expect(artifacts).toEqual([
      {
        slot: 'body',
        path: '/tmp/http_response.txt',
        storage_backend: 'host',
        size: 4096,
        lines: 380,
      },
    ])
  })

  it('builds initial readback commands for host and container artifacts', () => {
    const views = buildStoredArtifactViews({
      stored_artifacts: [
        {
          slot: 'body',
          path: '/tmp/http_response.txt',
          storage_backend: 'host',
          size: 4096,
          lines: 380,
        },
        {
          slot: 'stdout',
          path: '/workspace/context/shell_stdout.txt',
          storage_backend: 'container',
          size: 2048,
          lines: 120,
        },
      ],
    })

    expect(views[0]?.nextCommand).toContain('"offset": 1')
    expect(views[0]?.nextCommand).toContain('"limit": 200')
    expect(views[0]?.progressPercent).toBe(0)
    expect(views[1]?.nextCommand).toBe("sed -n '1,120p' /workspace/context/shell_stdout.txt")
    expect(views[1]?.progressPercent).toBe(0)
  })

  it('reflects tracked readback progress and next chunk boundaries', () => {
    const views = buildStoredArtifactViews(
      {
        stored_artifacts: [
          {
            slot: 'body',
            path: '/tmp/http_response.txt',
            storage_backend: 'host',
            size: 4096,
            lines: 380,
          },
          {
            slot: 'stdout',
            path: '/workspace/context/shell_stdout.txt',
            storage_backend: 'container',
            size: 2048,
            lines: 120,
          },
        ],
      },
      [
        {
          artifact_id: '/tmp/http_response.txt',
          artifact_kind: 'tool_output',
          storage_backend: 'host',
          source_tool: 'http_request',
          total_lines: 380,
          read_ranges: [{ start_line: 1, end_line: 200 }],
          contiguous_read_through_line: 200,
          fully_read: false,
          created_at_ms: 1,
          updated_at_ms: 2,
        },
        {
          artifact_id: '/workspace/context/shell_stdout.txt',
          artifact_kind: 'tool_output',
          storage_backend: 'container',
          source_tool: 'shell',
          total_lines: 120,
          read_ranges: [{ start_line: 1, end_line: 120 }],
          contiguous_read_through_line: 120,
          fully_read: true,
          created_at_ms: 1,
          updated_at_ms: 2,
        },
      ],
    )

    expect(views[0]?.progressLabel).toBe('200 / 380 lines covered')
    expect(views[0]?.progressPercent).toBe(53)
    expect(views[0]?.nextCommand).toContain('"offset": 201')
    expect(views[0]?.nextCommand).toContain('"limit": 180')
    expect(views[1]?.progressLabel).toBe('Sequential readback complete')
    expect(views[1]?.progressPercent).toBe(100)
    expect(views[1]?.nextLabel).toBe('Readback complete')
    expect(views[1]?.nextCommand).toBe('No further readback required')
  })
})
