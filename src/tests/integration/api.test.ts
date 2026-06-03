import { describe, it, expect, beforeEach, vi } from 'vitest'
import { invoke } from '@tauri-apps/api/core'

// Mock Tauri invoke
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}))

const mockInvoke = vi.mocked(invoke)

describe('API Integration Tests', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  describe('Database Operations', () => {
    it('should create scan task successfully', async () => {
      const mockTask = {
        id: '1',
        name: 'Test Scan',
        target: 'example.com',
        status: 'pending',
        created_at: new Date().toISOString(),
      }

      mockInvoke.mockResolvedValueOnce(mockTask)

      const result = await invoke('create_scan_task', {
        name: 'Test Scan',
        target: 'example.com',
        scan_type: 'comprehensive',
      })

      expect(result).toEqual(mockTask)
      expect(mockInvoke).toHaveBeenCalledWith('create_scan_task', {
        name: 'Test Scan',
        target: 'example.com',
        scan_type: 'comprehensive',
      })
    })

    it('should fetch vulnerabilities with filters', async () => {
      const mockVulns = [
        {
          id: '1',
          title: 'XSS Vulnerability',
          severity: 'high',
          status: 'open',
        },
        {
          id: '2',
          title: 'SQL Injection',
          severity: 'critical',
          status: 'verified',
        },
      ]

      mockInvoke.mockResolvedValueOnce(mockVulns)

      const result = await invoke('get_vulnerabilities', {
        filters: {
          severity: 'high',
          status: 'open',
        },
        limit: 20,
        offset: 0,
      })

      expect(result).toEqual(mockVulns)
      expect(result).toHaveLength(2)
    })

    it('should handle database errors gracefully', async () => {
      mockInvoke.mockRejectedValueOnce(new Error('Database connection failed'))

      await expect(
        invoke('get_scan_tasks')
      ).rejects.toThrow('Database connection failed')
    })
  })

  describe('MCP Tool Operations', () => {
    it('should execute nuclei scan successfully', async () => {
      const mockResult = {
        tool: 'nuclei',
        output: 'Scan completed successfully',
        vulnerabilities_found: 3,
        execution_time: 45.2,
      }

      mockInvoke.mockResolvedValueOnce(mockResult)

      const result = await invoke('execute_mcp_tool', {
        tool_name: 'nuclei',
        target: 'example.com',
        options: {
          templates: ['cves', 'exposures'],
          rate_limit: 150,
        },
      }) as typeof mockResult

      expect(result).toEqual(mockResult)
      expect(result.vulnerabilities_found).toBe(3)
    })

    it('should handle tool execution timeout', async () => {
      mockInvoke.mockRejectedValueOnce(new Error('Tool execution timeout'))

      await expect(
        invoke('execute_mcp_tool', {
          tool_name: 'nmap',
          target: 'example.com',
          options: { ports: '1-65535' },
        })
      ).rejects.toThrow('Tool execution timeout')
    })
  })

  describe('AI Service Integration', () => {
    it('should analyze vulnerability with AI', async () => {
      const mockAnalysis = {
        severity_score: 8.5,
        classification: 'Cross-Site Scripting (XSS)',
        impact_assessment: 'High risk of user data compromise',
        remediation_steps: [
          'Implement input validation',
          'Use Content Security Policy',
          'Encode output data',
        ],
        confidence: 0.95,
      }

      mockInvoke.mockResolvedValueOnce(mockAnalysis)

      const result = await invoke('analyze_vulnerability_with_ai', {
        vulnerability_data: {
          url: 'https://example.com/search',
          parameter: 'q',
          payload: '<script>alert(1)</script>',
          response: 'Reflected in page content',
        },
        model: 'gpt-4',
      }) as typeof mockAnalysis

      expect(result).toEqual(mockAnalysis)
      expect(result.confidence).toBeGreaterThan(0.9)
    })

    it('should generate scan strategy', async () => {
      const mockStrategy = {
        target_analysis: {
          technology_stack: ['nginx', 'php', 'mysql'],
          attack_surface: ['web', 'dns', 'ssl'],
        },
        recommended_tools: ['nuclei', 'httpx', 'subfinder'],
        scan_phases: [
          { phase: 'reconnaissance', tools: ['subfinder', 'httpx'] },
          { phase: 'vulnerability_scan', tools: ['nuclei'] },
          { phase: 'verification', tools: ['custom_scripts'] },
        ],
        estimated_duration: 30,
      }

      mockInvoke.mockResolvedValueOnce(mockStrategy)

      const result = await invoke('generate_ai_scan_strategy', {
        target: 'example.com',
        scan_type: 'comprehensive',
        time_limit: 60,
      }) as typeof mockStrategy

      expect(result).toEqual(mockStrategy)
      expect(result.scan_phases).toHaveLength(3)
    })
  })

  describe('Performance Monitoring', () => {
    it('should collect performance metrics', async () => {
      const mockMetrics = {
        memory_usage: 156.7,
        cpu_usage: 23.4,
        active_tasks: 5,
        response_times: {
          avg: 125.6,
          p95: 245.8,
          p99: 456.2,
        },
        error_rate: 0.02,
      }

      mockInvoke.mockResolvedValueOnce(mockMetrics)

      const result = await invoke('get_performance_metrics') as typeof mockMetrics

      expect(result).toEqual(mockMetrics)
      expect(result.cpu_usage).toBeLessThan(100)
      expect(result.error_rate).toBeLessThan(0.1)
    })
  })
}) 