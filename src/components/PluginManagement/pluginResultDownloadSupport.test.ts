import { describe, expect, it } from 'vitest'
import {
  buildStoredZipBlob,
  collectDownloadablePluginFiles,
  makePluginResultArchiveName,
} from './pluginResultDownloadSupport'
import type { AdvancedTestResult } from './types'

function makeResult(output: unknown): AdvancedTestResult {
  return {
    plugin_id: 'webpack_source_downloader',
    success: true,
    total_runs: 1,
    concurrency: 1,
    total_duration_ms: 10,
    avg_duration_ms: 10,
    total_findings: 0,
    unique_findings: 0,
    findings: [],
    runs: [
      {
        run_index: 1,
        duration_ms: 10,
        findings: 0,
        error: null,
        output,
      },
    ],
    outputs: [output],
  }
}

describe('pluginResultDownloadSupport', () => {
  it('collects files from nested plugin outputs and deduplicates archive paths', () => {
    const result = makeResult({
      success: true,
      output: {
        success: true,
        data: {
          files: [
            {
              archivePath: '../src/App.vue',
              sourcePath: 'webpack:///./src/App.vue',
              content: '<template>ok</template>',
              mimeType: 'text/x-vue',
            },
            {
              archivePath: '../src/App.vue',
              content: 'duplicate',
            },
            {
              downloadPath: 'src/main.ts',
              contentBase64: btoa('console.log("ok")'),
            },
          ],
        },
      },
    })

    const files = collectDownloadablePluginFiles(result)

    expect(files).toHaveLength(2)
    expect(files[0].archivePath).toBe('src/App.vue')
    expect(new TextDecoder().decode(files[0].bytes)).toBe('<template>ok</template>')
    expect(files[1].archivePath).toBe('src/main.ts')
    expect(new TextDecoder().decode(files[1].bytes)).toBe('console.log("ok")')
  })

  it('builds a stored zip blob containing central directory records', async () => {
    const result = makeResult({
      data: {
        files: [
          {
            archivePath: 'src/App.vue',
            content: '<template>ok</template>',
          },
        ],
      },
    })

    const files = collectDownloadablePluginFiles(result)
    const zip = buildStoredZipBlob(files)
    expect(zip.type).toBe('application/zip')
    expect(zip.size).toBeGreaterThan(files[0].bytes.byteLength)
  })

  it('creates stable archive names from plugin ids', () => {
    expect(makePluginResultArchiveName('webpack/source downloader')).toMatch(
      /^webpack_source_downloader-files-.*\.zip$/,
    )
  })
})
