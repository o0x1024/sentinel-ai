#!/usr/bin/env node

import fs from 'node:fs'
import http from 'node:http'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { URL } from 'node:url'

import {
  approveExpenseClaim,
  canUseCollabSummary,
  canViewPortalClaim,
  createExpenseClaim,
  demoDbPath,
  disburseExpenseClaim,
  ensureDemoDatabase,
  getClaimDetail,
  getPlatformSnapshot,
  getUserByUsername,
  listClaimsForPortalUser,
  listDemoUsers,
  resetDemoDatabase,
  submitExpenseClaim,
} from './demo-business-service-db.mjs'

const port = Number(process.env.DEMO_BIZ_PORT || 7788)
const host = process.env.DEMO_BIZ_HOST || '127.0.0.1'
const __dirname = path.dirname(fileURLToPath(import.meta.url))
const uiHtmlPath = path.join(__dirname, 'demo-business-service-ui.html')
const uiHtml = fs.readFileSync(uiHtmlPath, 'utf8')

ensureDemoDatabase()

function sendJson(res, statusCode, data) {
  res.writeHead(statusCode, {
    'content-type': 'application/json; charset=utf-8',
    'access-control-allow-origin': '*',
    'access-control-allow-headers': 'content-type, authorization, x-demo-user',
    'access-control-allow-methods': 'GET,POST,OPTIONS',
  })
  res.end(JSON.stringify(data, null, 2))
}

function sendHtml(res, statusCode, html) {
  res.writeHead(statusCode, {
    'content-type': 'text/html; charset=utf-8',
    'access-control-allow-origin': '*',
  })
  res.end(html)
}

function notFound(res, message = 'Not Found') {
  sendJson(res, 404, { success: false, message })
}

function unauthorized(res, message = 'Unauthorized') {
  sendJson(res, 401, { success: false, message })
}

function forbidden(res, message = 'Forbidden') {
  sendJson(res, 403, { success: false, message })
}

function badRequest(res, message = 'Bad Request') {
  sendJson(res, 400, { success: false, message })
}

function parseBody(req) {
  return new Promise((resolve, reject) => {
    let raw = ''
    req.on('data', chunk => {
      raw += chunk
      if (raw.length > 1024 * 1024) {
        reject(new Error('Body too large'))
      }
    })
    req.on('end', () => {
      if (!raw.trim()) {
        resolve({})
        return
      }
      try {
        resolve(JSON.parse(raw))
      } catch {
        reject(new Error('Invalid JSON body'))
      }
    })
    req.on('error', reject)
  })
}

function summarizeUser(user) {
  if (!user) {
    return { authenticated: false }
  }
  return {
    authenticated: true,
    id: user.id,
    username: user.username,
    displayName: user.displayName,
    role: user.role,
    tenantId: user.tenantId,
    department: user.department,
  }
}

function getUserFromRequest(req) {
  const explicitUser = req.headers['x-demo-user']
  if (typeof explicitUser === 'string') {
    return getUserByUsername(explicitUser)
  }

  const authorization = req.headers.authorization
  if (typeof authorization === 'string' && authorization.startsWith('Bearer ')) {
    const token = authorization.slice('Bearer '.length).trim()
    if (token.startsWith('portal-') && token.endsWith('-session')) {
      const username = token.slice('portal-'.length, '-session'.length * -1)
      return getUserByUsername(username)
    }
  }

  return null
}

function logRequest(req, statusCode, user) {
  const line = [
    new Date().toISOString(),
    req.method,
    req.url,
    String(statusCode),
    user ? `${user.username}/${user.role}/${user.tenantId}` : 'anonymous',
  ]
  console.log(line.join(' | '))
}

function respondScenario(res, req, statusCode, user, scenario, data) {
  sendJson(res, statusCode, {
    success: statusCode >= 200 && statusCode < 300,
    scenario,
    actor: summarizeUser(user),
    ...(data || {}),
  })
  logRequest(req, statusCode, user)
}

function getPathId(pathname, regex) {
  const match = pathname.match(regex)
  return match ? match[1] : null
}

async function handleRequest(req, res) {
  if (!req.url) {
    notFound(res)
    return
  }

  if (req.method === 'OPTIONS') {
    sendJson(res, 200, { success: true })
    return
  }

  const url = new URL(req.url, `http://${req.headers.host || 'localhost'}`)
  const user = getUserFromRequest(req)

  if (req.method === 'GET' && (url.pathname === '/' || url.pathname === '/index.html')) {
    sendHtml(res, 200, uiHtml)
    logRequest(req, 200, user)
    return
  }

  if (req.method === 'GET' && url.pathname === '/health') {
    sendJson(res, 200, {
      success: true,
      service: 'expense-collaboration-platform',
      dbPath: demoDbPath,
      demoFocus: ['horizontal-idor', 'workflow-bypass'],
      users: listDemoUsers().map(item => ({
        username: item.username,
        role: item.role,
        tenantId: item.tenantId,
      })),
    })
    logRequest(req, 200, user)
    return
  }

  if (req.method === 'POST' && url.pathname === '/api/auth/login') {
    const body = await parseBody(req).catch(error => ({ __error: error.message }))
    if (body.__error) {
      badRequest(res, body.__error)
      logRequest(req, 400, user)
      return
    }

    const username = typeof body.username === 'string' ? body.username : ''
    const targetUser = getUserByUsername(username)
    if (!targetUser) {
      unauthorized(res, 'Unknown account')
      logRequest(req, 401, user)
      return
    }

    sendJson(res, 200, {
      success: true,
      token: `portal-${targetUser.username}-session`,
      user: summarizeUser(targetUser),
    })
    logRequest(req, 200, targetUser)
    return
  }

  if (!user) {
    unauthorized(res, 'Use x-demo-user or Bearer portal-{username}-session')
    logRequest(req, 401, user)
    return
  }

  if (req.method === 'GET' && url.pathname === '/api/platform/state') {
    sendJson(res, 200, {
      success: true,
      ...getPlatformSnapshot(),
    })
    logRequest(req, 200, user)
    return
  }

  if (req.method === 'POST' && url.pathname === '/api/platform/reset') {
    if (user.role !== 'admin') {
      forbidden(res, 'Only admin can reset demo data')
      logRequest(req, 403, user)
      return
    }

    resetDemoDatabase()
    respondScenario(res, req, 200, user, 'platform-reset', {
      reset: true,
      dbPath: demoDbPath,
      claims: getPlatformSnapshot().claims.length,
    })
    return
  }

  if (req.method === 'GET' && url.pathname === '/api/portal/expense-claims') {
    respondScenario(res, req, 200, user, 'portal-claim-list', {
      claims: listClaimsForPortalUser(user),
    })
    return
  }

  const safeDetailId = getPathId(url.pathname, /^\/api\/portal\/expense-claims\/([^/]+)\/detail$/)
  if (req.method === 'GET' && safeDetailId) {
    const claim = getClaimDetail(safeDetailId)
    if (!claim) {
      notFound(res, 'Expense claim not found')
      logRequest(req, 404, user)
      return
    }
    if (!canViewPortalClaim(user, claim)) {
      forbidden(res, 'Claim is outside your normal portal scope')
      logRequest(req, 403, user)
      return
    }
    respondScenario(res, req, 200, user, 'portal-claim-detail', {
      claim,
    })
    return
  }

  const weakSummaryId = getPathId(
    url.pathname,
    /^\/api\/mobile\/collab\/expense-claims\/([^/]+)\/summary$/,
  )
  if (req.method === 'GET' && weakSummaryId) {
    const claim = getClaimDetail(weakSummaryId)
    if (!claim) {
      notFound(res, 'Expense claim not found')
      logRequest(req, 404, user)
      return
    }
    if (!canUseCollabSummary(user, claim)) {
      forbidden(res, 'Claim is outside your tenant scope')
      logRequest(req, 403, user)
      return
    }
    respondScenario(res, req, 200, user, 'mobile-collab-summary', {
      claim,
      accessModel: 'weak-mobile-summary',
      warning: 'This endpoint only checks tenant scope and does not enforce applicant ownership.',
    })
    return
  }

  if (req.method === 'POST' && url.pathname === '/api/portal/expense-claims') {
    const body = await parseBody(req).catch(error => ({ __error: error.message }))
    if (body.__error) {
      badRequest(res, body.__error)
      logRequest(req, 400, user)
      return
    }

    const claim = createExpenseClaim(user, body)
    respondScenario(res, req, 200, user, 'claim-create-draft', {
      claim,
    })
    return
  }

  const submitId = getPathId(url.pathname, /^\/api\/portal\/expense-claims\/([^/]+)\/submit$/)
  if (req.method === 'POST' && submitId) {
    const result = submitExpenseClaim(user, submitId)
    if (result.error === 'not_found') {
      notFound(res, 'Expense claim not found')
      logRequest(req, 404, user)
      return
    }
    if (result.error === 'forbidden_owner') {
      forbidden(res, 'Only applicant can submit this expense claim')
      logRequest(req, 403, user)
      return
    }
    if (result.error === 'invalid_status') {
      badRequest(res, 'Only draft expense claims can be submitted')
      logRequest(req, 400, user)
      return
    }
    respondScenario(res, req, 200, user, 'claim-submit', result)
    return
  }

  const approveId = getPathId(url.pathname, /^\/api\/workflow\/expense-claims\/([^/]+)\/approve$/)
  if (req.method === 'POST' && approveId) {
    const result = approveExpenseClaim(user, approveId, {
      bypass: false,
      channel: 'workflow',
    })
    if (result.error === 'not_found') {
      notFound(res, 'Expense claim not found')
      logRequest(req, 404, user)
      return
    }
    if (result.error === 'forbidden_role') {
      forbidden(res, 'Only manager can approve expense claims')
      logRequest(req, 403, user)
      return
    }
    if (result.error === 'forbidden_tenant') {
      forbidden(res, 'Claim is outside your tenant workflow scope')
      logRequest(req, 403, user)
      return
    }
    if (result.error === 'invalid_status') {
      badRequest(res, 'Expense claim must be submitted before approval')
      logRequest(req, 400, user)
      return
    }
    respondScenario(res, req, 200, user, 'claim-approve-standard', result)
    return
  }

  const quickApproveId = getPathId(
    url.pathname,
    /^\/api\/mobile\/workbench\/expense-claims\/([^/]+)\/quick-approve$/,
  )
  if (req.method === 'POST' && quickApproveId) {
    const result = approveExpenseClaim(user, quickApproveId, {
      bypass: true,
      channel: 'mobile-workbench',
    })
    if (result.error === 'not_found') {
      notFound(res, 'Expense claim not found')
      logRequest(req, 404, user)
      return
    }
    if (result.error === 'forbidden_tenant') {
      forbidden(res, 'Claim is outside your tenant scope')
      logRequest(req, 403, user)
      return
    }
    respondScenario(res, req, 200, user, 'claim-approve-bypass', result)
    return
  }

  const disburseId = getPathId(
    url.pathname,
    /^\/api\/treasury\/expense-claims\/([^/]+)\/disburse$/,
  )
  if (req.method === 'POST' && disburseId) {
    const result = disburseExpenseClaim(user, disburseId, {
      bypass: false,
      channel: 'treasury',
    })
    if (result.error === 'not_found') {
      notFound(res, 'Expense claim not found')
      logRequest(req, 404, user)
      return
    }
    if (result.error === 'forbidden_role') {
      forbidden(res, 'Only finance can disburse expense claims')
      logRequest(req, 403, user)
      return
    }
    if (result.error === 'forbidden_tenant') {
      forbidden(res, 'Claim is outside your tenant treasury scope')
      logRequest(req, 403, user)
      return
    }
    if (result.error === 'invalid_status') {
      badRequest(res, 'Expense claim must be approved before disbursement')
      logRequest(req, 400, user)
      return
    }
    respondScenario(res, req, 200, user, 'claim-disburse-standard', result)
    return
  }

  const releaseId = getPathId(
    url.pathname,
    /^\/api\/integrations\/treasury\/expense-claims\/([^/]+)\/release$/,
  )
  if (req.method === 'POST' && releaseId) {
    const result = disburseExpenseClaim(user, releaseId, {
      bypass: true,
      channel: 'treasury-integration',
    })
    if (result.error === 'not_found') {
      notFound(res, 'Expense claim not found')
      logRequest(req, 404, user)
      return
    }
    if (result.error === 'forbidden_tenant') {
      forbidden(res, 'Claim is outside your tenant integration scope')
      logRequest(req, 403, user)
      return
    }
    respondScenario(res, req, 200, user, 'claim-disburse-bypass', result)
    return
  }

  notFound(res)
  logRequest(req, 404, user)
}

if (process.argv.includes('--help')) {
  console.log(`expense-collaboration-platform

Usage:
  npm run demo:biz-service
  DEMO_BIZ_PORT=7788 DEMO_BIZ_HOST=127.0.0.1 node scripts/demo-business-service.mjs

SQLite:
  ${demoDbPath}

Accounts:
  alice, carol, bob, manager, finance, admin

Auth:
  Authorization: Bearer portal-{username}-session
  x-demo-user: {username}

Focus:
  1. Horizontal IDOR via /api/mobile/collab/expense-claims/{id}/summary
  2. Workflow bypass via /api/mobile/workbench/.../quick-approve and /api/integrations/.../release
`)
  process.exit(0)
}

const server = http.createServer(async (req, res) => {
  try {
    await handleRequest(req, res)
  } catch (error) {
    console.error('expense-collaboration-platform error', error)
    sendJson(res, 500, {
      success: false,
      message: error instanceof Error ? error.message : 'Internal server error',
    })
  }
})

server.listen(port, host, () => {
  console.log(`expense-collaboration-platform listening on http://${host}:${port}`)
  console.log(`web portal: http://${host}:${port}/`)
  console.log(`sqlite db: ${demoDbPath}`)
  console.log('accounts: alice, carol, bob, manager, finance, admin')
  console.log('focus: horizontal idor + workflow bypass')
})
