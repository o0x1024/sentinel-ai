import { defineConfig } from 'vitepress'

export default defineConfig({
  title: 'Sentinel AI',
  description: 'AI驱动的漏洞挖掘平台 - 完整文档',
  base: '/docs/',
  
  themeConfig: {
    logo: '/logo.svg',
    nav: [
      { text: '首页', link: '/' },
      { text: '用户指南', link: '/guide/introduction' },
      { text: '开发文档', link: '/development/architecture' },
      { text: 'API文档', link: '/api/overview' },
      { text: '部署指南', link: '/deployment/production' },
    ],

    sidebar: {
      '/guide/': [
        {
          text: '用户指南',
          items: [
            { text: '简介', link: '/guide/introduction' },
            { text: '快速开始', link: '/guide/getting-started' },
            { text: '安装配置', link: '/guide/installation' },
            { text: '基本使用', link: '/guide/basic-usage' },
            { text: '扫描任务', link: '/guide/scan-tasks' },
            { text: '漏洞管理', link: '/guide/vulnerability-management' },
            { text: 'AI助手', link: '/guide/ai-assistant' },
            { text: '常见问题', link: '/guide/faq' },
          ]
        }
      ],
      '/development/': [
        {
          text: '开发文档',
          items: [
            { text: '技术架构', link: '/development/architecture' },
            { text: '开发环境', link: '/development/development-setup' },
            { text: '前端开发', link: '/development/frontend' },
            { text: '后端开发', link: '/development/backend' },
            { text: '数据库设计', link: '/development/database' },
            { text: 'MCP协议', link: '/development/mcp-protocol' },
            { text: 'AI集成', link: '/development/ai-integration' },
            { text: '测试指南', link: '/development/testing' },
            { text: '贡献指南', link: '/development/contributing' },
          ]
        }
      ],
      '/api/': [
        {
          text: 'API文档',
          items: [
            { text: 'API概览', link: '/api/overview' },
            { text: '扫描API', link: '/api/scanning' },
            { text: '漏洞API', link: '/api/vulnerabilities' },
            { text: 'AI服务API', link: '/api/ai-services' },
            { text: 'MCP工具API', link: '/api/mcp-tools' },
            { text: '数据库API', link: '/api/database' },
            { text: '性能API', link: '/api/performance' },
          ]
        }
      ],
      '/deployment/': [
        {
          text: '部署指南',
          items: [
            { text: '生产部署', link: '/deployment/production' },
            { text: 'Docker部署', link: '/deployment/docker' },
            { text: '系统要求', link: '/deployment/requirements' },
            { text: '配置管理', link: '/deployment/configuration' },
            { text: '监控运维', link: '/deployment/monitoring' },
            { text: '故障排除', link: '/deployment/troubleshooting' },
          ]
        }
      ]
    },

    socialLinks: [
      { icon: 'github', link: 'https://github.com/user/sentinel-ai' }
    ],

    editLink: {
      pattern: 'https://github.com/user/sentinel-ai/edit/main/docs/:path',
      text: '在 GitHub 上编辑此页'
    },

    lastUpdated: {
      text: '最后更新',
      formatOptions: {
        dateStyle: 'short',
        timeStyle: 'medium'
      }
    },

    search: {
      provider: 'local',
      options: {
        locales: {
          root: {
            translations: {
              button: {
                buttonText: '搜索文档',
                buttonAriaLabel: '搜索文档'
              },
              modal: {
                noResultsText: '无法找到相关结果',
                resetButtonTitle: '清除查询条件',
                footer: {
                  selectText: '选择',
                  navigateText: '切换'
                }
              }
            }
          }
        }
      }
    },

    footer: {
      message: '基于 MIT 许可证发布',
      copyright: 'Copyright © 2024 Sentinel AI'
    }
  },

  markdown: {
    lineNumbers: true,
    container: {
      tipLabel: '提示',
      warningLabel: '警告',
      dangerLabel: '危险',
      infoLabel: '信息',
      detailsLabel: '详细信息'
    }
  },

  head: [
    ['link', { rel: 'icon', href: '/favicon.ico' }],
    ['meta', { name: 'theme-color', content: '#3eaf7c' }],
    ['meta', { name: 'apple-mobile-web-app-capable', content: 'yes' }],
    ['meta', { name: 'apple-mobile-web-app-status-bar-style', content: 'black' }]
  ]
}) 