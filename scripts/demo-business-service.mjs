#!/usr/bin/env node

import fs from 'node:fs'
import http from 'node:http'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { URL } from 'node:url'

const port = Number(process.env.DEMO_BIZ_PORT || 7788)
const host = process.env.DEMO_BIZ_HOST || '127.0.0.1'
const __dirname = path.dirname(fileURLToPath(import.meta.url))
const uiHtmlPath = path.join(__dirname, 'demo-business-service-ui.html')
const uiHtml = fs.readFileSync(uiHtmlPath, 'utf8')

const users = {
  alice: { id: 'u-alice', username: 'alice', role: 'user', tenantId: 'tenant-a' },
  bob: { id: 'u-bob', username: 'bob', role: 'user', tenantId: 'tenant-b' },
  admin: { id: 'u-admin', username: 'admin', role: 'admin', tenantId: 'tenant-a' },
  finance: { id: 'u-finance', username: 'finance', role: 'finance', tenantId: 'tenant-a' },
}

const orders = new Map([
  ['ord-1001', { id: 'ord-1001', tenantId: 'tenant-a', ownerId: 'u-alice', amount: 199, status: 'draft', item: 'Laptop Bag' }],
  ['ord-1002', { id: 'ord-1002', tenantId: 'tenant-a', ownerId: 'u-admin', amount: 599, status: 'submitted', item: 'Monitor' }],
  ['ord-2001', { id: 'ord-2001', tenantId: 'tenant-b', ownerId: 'u-bob', amount: 88, status: 'submitted', item: 'Keyboard' }],
])

const projects = new Map([
  ['proj-100', { id: 'proj-100', tenantId: 'tenant-a', ownerId: 'u-alice', name: 'Apollo', members: ['alice', 'admin'] }],
  ['proj-200', { id: 'proj-200', tenantId: 'tenant-b', ownerId: 'u-bob', name: 'Borealis', members: ['bob'] }],
])

const invoices = new Map([
  ['inv-100', { id: 'inv-100', tenantId: 'tenant-a', ownerId: 'u-alice', amount: 320, paidCount: 0 }],
  ['inv-200', { id: 'inv-200', tenantId: 'tenant-b', ownerId: 'u-bob', amount: 450, paidCount: 0 }],
])

const couponRedemptions = []
const transfers = new Map()
let couponUseSequence = 0
let transferSequence = 0

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

function getUserFromRequest(req) {
  const explicitUser = req.headers['x-demo-user']
  if (typeof explicitUser === 'string' && users[explicitUser]) {
    return users[explicitUser]
  }

  const authorization = req.headers.authorization
  if (typeof authorization === 'string' && authorization.startsWith('Bearer ')) {
    const token = authorization.slice('Bearer '.length).trim()
    if (token.startsWith('demo-')) {
      const username = token.slice('demo-'.length)
      if (users[username]) {
        return users[username]
      }
    }
  }

  return null
}

function summarizeUser(user) {
  if (!user) {
    return { authenticated: false }
  }
  return {
    authenticated: true,
    id: user.id,
    username: user.username,
    role: user.role,
    tenantId: user.tenantId,
  }
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
      service: 'demo-business-service',
      scenarios: ['idor', 'approval-bypass', 'double-redeem', 'skip-step', 'double-pay'],
    })
    logRequest(req, 200, user)
    return
  }

  if (req.method === 'POST' && url.pathname === '/api/auth/login') {
    const body = await parseBody(req).catch(error => ({ __error: error.message }))
    if (body.__error) {
      sendJson(res, 400, { success: false, message: body.__error })
      logRequest(req, 400, user)
      return
    }

    const username = typeof body.username === 'string' ? body.username : ''
    const targetUser = users[username]
    if (!targetUser) {
      unauthorized(res, 'Unknown demo user')
      logRequest(req, 401, user)
      return
    }

    sendJson(res, 200, {
      success: true,
      token: `demo-${targetUser.username}`,
      user: summarizeUser(targetUser),
    })
    logRequest(req, 200, targetUser)
    return
  }

  if (!user) {
    unauthorized(res, 'Use x-demo-user or Bearer demo-{username}')
    logRequest(req, 401, user)
    return
  }

  const orderMatch = url.pathname.match(/^\/api\/orders\/([^/]+)$/)
  if (req.method === 'GET' && orderMatch) {
    const order = orders.get(orderMatch[1])
    if (!order) {
      notFound(res, 'Order not found')
      logRequest(req, 404, user)
      return
    }

    // Intentionally vulnerable: no owner or tenant authorization check.
    sendJson(res, 200, {
      success: true,
      scenario: 'idor',
      order,
      actor: summarizeUser(user),
      riskHint: 'Order lookup does not validate tenantId/ownerId before returning data.',
    })
    logRequest(req, 200, user)
    return
  }

  const approveMatch = url.pathname.match(/^\/api\/orders\/([^/]+)\/approve$/)
  if (req.method === 'POST' && approveMatch) {
    const order = orders.get(approveMatch[1])
    if (!order) {
      notFound(res, 'Order not found')
      logRequest(req, 404, user)
      return
    }

    // Intentionally vulnerable: any authenticated user can approve any order.
    order.status = 'approved'
    order.approvedBy = user.username
    sendJson(res, 200, {
      success: true,
      scenario: 'approval-bypass',
      order,
      actor: summarizeUser(user),
      riskHint: 'Approval endpoint does not check role or tenant boundary.',
    })
    logRequest(req, 200, user)
    return
  }

  const projectMembersMatch = url.pathname.match(/^\/api\/projects\/([^/]+)\/members$/)
  if (req.method === 'GET' && projectMembersMatch) {
    const project = projects.get(projectMembersMatch[1])
    if (!project) {
      notFound(res, 'Project not found')
      logRequest(req, 404, user)
      return
    }

    // Intentionally vulnerable: cross-tenant membership details are returned.
    sendJson(res, 200, {
      success: true,
      scenario: 'idor',
      projectId: project.id,
      tenantId: project.tenantId,
      members: project.members,
      actor: summarizeUser(user),
      riskHint: 'Project member list is exposed without project membership validation.',
    })
    logRequest(req, 200, user)
    return
  }

  if (req.method === 'POST' && url.pathname === '/api/coupons/redeem') {
    const body = await parseBody(req).catch(error => ({ __error: error.message }))
    if (body.__error) {
      sendJson(res, 400, { success: false, message: body.__error })
      logRequest(req, 400, user)
      return
    }

    const couponCode = typeof body.couponCode === 'string' ? body.couponCode : 'UNKNOWN'
    const orderId = typeof body.orderId === 'string' ? body.orderId : null
    couponUseSequence += 1
    const record = {
      redemptionId: `red-${couponUseSequence}`,
      couponCode,
      orderId,
      actor: user.username,
      tenantId: user.tenantId,
      grantedDiscount: 50,
      redeemedAt: new Date().toISOString(),
    }
    couponRedemptions.push(record)

    // Intentionally vulnerable: coupon can be redeemed repeatedly without single-use control.
    sendJson(res, 200, {
      success: true,
      scenario: 'double-redeem',
      redemption: record,
      redemptionCountForCoupon: couponRedemptions.filter(item => item.couponCode === couponCode).length,
      riskHint: 'Coupon redemption lacks replay / single-use validation.',
    })
    logRequest(req, 200, user)
    return
  }

  if (req.method === 'POST' && url.pathname === '/api/transfers/prepare') {
    const body = await parseBody(req).catch(error => ({ __error: error.message }))
    if (body.__error) {
      sendJson(res, 400, { success: false, message: body.__error })
      logRequest(req, 400, user)
      return
    }

    transferSequence += 1
    const transferId = `tr-${transferSequence}`
    const transfer = {
      id: transferId,
      createdBy: user.username,
      tenantId: user.tenantId,
      fromAccount: body.fromAccount || `${user.username}-wallet`,
      toAccount: body.toAccount || 'merchant-main',
      amount: Number(body.amount || 0),
      prepared: true,
      preparedAt: new Date().toISOString(),
    }
    transfers.set(transferId, transfer)
    sendJson(res, 200, {
      success: true,
      scenario: 'workflow-step',
      transfer,
      riskHint: 'Use /confirm to test whether the server enforces the prepared state.',
    })
    logRequest(req, 200, user)
    return
  }

  if (req.method === 'POST' && url.pathname === '/api/transfers/confirm') {
    const body = await parseBody(req).catch(error => ({ __error: error.message }))
    if (body.__error) {
      sendJson(res, 400, { success: false, message: body.__error })
      logRequest(req, 400, user)
      return
    }

    const transferId = typeof body.transferId === 'string' ? body.transferId : null
    const existing = transferId ? transfers.get(transferId) : null
    const transfer = existing || {
      id: transferId || 'tr-missing',
      createdBy: user.username,
      tenantId: user.tenantId,
      fromAccount: body.fromAccount || `${user.username}-wallet`,
      toAccount: body.toAccount || 'merchant-main',
      amount: Number(body.amount || 0),
      prepared: false,
    }

    // Intentionally vulnerable: confirm succeeds even when no prepared step exists.
    transfer.confirmed = true
    transfer.confirmedBy = user.username
    transfer.confirmedAt = new Date().toISOString()
    transfers.set(transfer.id, transfer)
    sendJson(res, 200, {
      success: true,
      scenario: 'skip-step',
      transfer,
      riskHint: 'Transfer confirmation does not require a valid prepared state.',
    })
    logRequest(req, 200, user)
    return
  }

  const invoicePayMatch = url.pathname.match(/^\/api\/invoices\/([^/]+)\/pay$/)
  if (req.method === 'POST' && invoicePayMatch) {
    const invoice = invoices.get(invoicePayMatch[1])
    if (!invoice) {
      notFound(res, 'Invoice not found')
      logRequest(req, 404, user)
      return
    }

    // Intentionally vulnerable: repeated payment is allowed.
    invoice.paidCount += 1
    sendJson(res, 200, {
      success: true,
      scenario: 'double-pay',
      invoice,
      actor: summarizeUser(user),
      riskHint: 'Invoice payment does not enforce idempotency or paid status.',
    })
    logRequest(req, 200, user)
    return
  }

  if (req.method === 'GET' && url.pathname === '/api/demo/state') {
    sendJson(res, 200, {
      success: true,
      users: Object.values(users),
      orders: Array.from(orders.values()),
      projects: Array.from(projects.values()),
      invoices: Array.from(invoices.values()),
      couponRedemptions,
      transfers: Array.from(transfers.values()),
    })
    logRequest(req, 200, user)
    return
  }

  notFound(res)
  logRequest(req, 404, user)
}

if (process.argv.includes('--help')) {
  console.log(`demo-business-service

Usage:
  npm run demo:biz-service
  DEMO_BIZ_PORT=7788 DEMO_BIZ_HOST=127.0.0.1 node scripts/demo-business-service.mjs

Demo users:
  alice, bob, admin, finance

Auth:
  Authorization: Bearer demo-{username}
  x-demo-user: {username}
`)
  process.exit(0)
}

const server = http.createServer(async (req, res) => {
  try {
    await handleRequest(req, res)
  } catch (error) {
    console.error('demo-business-service error', error)
    sendJson(res, 500, {
      success: false,
      message: error instanceof Error ? error.message : 'Internal server error',
    })
  }
})

server.listen(port, host, () => {
  console.log(`demo-business-service listening on http://${host}:${port}`)
  console.log(`demo ui: http://${host}:${port}/`)
  console.log('demo users: alice, bob, admin, finance')
  console.log('use Authorization: Bearer demo-alice or x-demo-user: alice')
  console.log('scenarios: idor, approval-bypass, double-redeem, skip-step, double-pay')
})
