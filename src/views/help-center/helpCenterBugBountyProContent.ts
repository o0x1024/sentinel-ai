import type { HelpCenterFeatureEntry } from './helpCenterContent'

export const COMMUNITY_WECHAT_QR_PATH = '/community/wechat-qrcode.png'

export const zhBugBountyProEntry: HelpCenterFeatureEntry = {
  id: 'feature-bug-bounty-pro',
  icon: 'fas fa-trophy',
  title: '漏洞赏金（Pro 专属）',
  route: '/bug-bounty',
  summary:
    'Community 版侧边栏保留灰色「漏洞赏金」入口，但完整 Program、资产、Finding、Submission、监控与知识库能力仅在 Sentinel AI Pro 提供。如需开通或咨询，可扫码添加微信联系。',
  capabilities: [
    'Community 版可进入占位页，查看 Pro 功能说明与联系方式。',
    'Pro 版包含 Program/Scope、资产面发现、Finding 运营、Submission 跟踪、监控调度与工作流编排。',
    'Community 与 Pro 使用不同应用标识，可同时安装。',
  ],
  operations: [
    '在侧边栏点击灰色「漏洞赏金 / PRO」进入占位页。',
    '阅读 Pro 能力说明后，扫描页面中的微信二维码添加联系。',
    '也可在 GitHub README 或本帮助文档中查看同一二维码图片。',
  ],
  detailSections: [
    {
      id: 'bug-bounty-pro-contact',
      title: '微信联系开通 Pro',
      description: '扫描下方二维码添加微信，咨询 Sentinel AI Pro 与漏洞赏金模块。',
      imageSrc: COMMUNITY_WECHAT_QR_PATH,
      imageAlt: '微信二维码',
    },
  ],
}

export const enBugBountyProEntry: HelpCenterFeatureEntry = {
  id: 'feature-bug-bounty-pro',
  icon: 'fas fa-trophy',
  title: 'Bug Bounty (Pro only)',
  route: '/bug-bounty',
  summary:
    'The Community Edition keeps a gray Bug Bounty sidebar entry, but the full program, asset, finding, submission, monitor, and knowledge-base workspace is available in Sentinel AI Pro only. Scan the WeChat QR code to contact us about Pro.',
  capabilities: [
    'Community users can open the placeholder page for Pro feature overview and contact details.',
    'Pro includes program/scope management, attack-surface discovery, finding operations, submissions, monitors, and workflow orchestration.',
    'Community and Pro use different app identifiers and can be installed side by side.',
  ],
  operations: [
    'Open the gray Bug Bounty / PRO item in the sidebar.',
    'Review the Pro capability list and scan the WeChat QR code on that page.',
    'The same QR code is also shown in the GitHub README and this help document.',
  ],
  detailSections: [
    {
      id: 'bug-bounty-pro-contact',
      title: 'Contact via WeChat',
      description: 'Scan the QR code below to add us on WeChat and ask about Sentinel AI Pro.',
      imageSrc: COMMUNITY_WECHAT_QR_PATH,
      imageAlt: 'WeChat QR code',
    },
  ],
}
