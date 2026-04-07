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
  alice: { id: 'u-alice', username: 'alice', role: 'employee', tenantId: 'tenant-a', department: 'sales' },
  bob: { id: 'u-bob', username: 'bob', role: 'employee', tenantId: 'tenant-b', department: 'ops' },
  manager: { id: 'u-manager', username: 'manager', role: 'manager', tenantId: 'tenant-a', department: 'sales' },
  finance: { id: 'u-finance', username: 'finance', role: 'finance', tenantId: 'tenant-a', department: 'finance' },
  admin: { id: 'u-admin', username: 'admin', role: 'admin', tenantId: 'tenant-a', department: 'security' },
}

const orders = new Map([
  [
    'ord-a-1001',
    {
      id: 'ord-a-1001',
      orderNo: 'SO-A-2026-1001',
      tenantId: 'tenant-a',
      ownerId: 'u-alice',
      customerId: 'cust-cn-1088',
      serviceDeskCaseId: 'csr-a-3001',
      amount: 199,
      status: 'awaiting_dispatch',
      item: '华东渠道样品包',
      businessUnit: 'east-sales',
    },
  ],
  [
    'ord-b-2001',
    {
      id: 'ord-b-2001',
      orderNo: 'SO-B-2026-2001',
      tenantId: 'tenant-b',
      ownerId: 'u-bob',
      customerId: 'cust-eu-2081',
      serviceDeskCaseId: 'csr-b-9008',
      amount: 88,
      status: 'submitted',
      item: '售后备件键盘',
      businessUnit: 'global-ops',
    },
  ],
])

const reimbursements = new Map()
const transfers = new Map()
const payments = new Map()
const coupons = new Map([
  ['SPRING-2026', { code: 'SPRING-2026', singleUse: true, uses: [] }],
  ['OPS-BONUS', { code: 'OPS-BONUS', singleUse: true, uses: [] }],
])
let reimbursementSeq = 1
let transferSeq = 1
let paymentSeq = 1

function padSequence(value, width = 4) {
  return String(value).padStart(width, '0')
}

function seedPlatformState() {
  reimbursements.clear()
  transfers.clear()
  payments.clear()

  for (const coupon of coupons.values()) {
    coupon.uses = []
  }

  reimbursements.set('ec-a-0901', {
    id: 'ec-a-0901',
    claimNo: 'EXP-2026-0901',
    tenantId: 'tenant-a',
    applicantId: users.alice.id,
    applicant: users.alice.username,
    claimType: 'travel',
    item: '季度客户拜访机酒',
    amount: 2860,
    costCenter: 'CC-SALES-01',
    expensePolicyId: 'POL-TRAVEL-2026',
    vendorId: 'vendor-air-cn',
    businessLine: 'regional-sales',
    approvalTaskId: 'task-rb-0901',
    treasuryDisbursementRef: 'dj-rb-0901',
    settlementBatchId: 'TB-20260407-001',
    payoutChannel: 'cmb-corporate',
    status: 'paid',
    approvalHistory: [{ by: 'manager', at: '2026-04-03T09:20:00.000Z', mode: 'standard' }],
    createdAt: '2026-04-03T08:30:00.000Z',
    submittedAt: '2026-04-03T08:50:00.000Z',
    paidAt: '2026-04-03T10:10:00.000Z',
    paidCount: 1,
  })
  reimbursements.set('ec-a-0902', {
    id: 'ec-a-0902',
    claimNo: 'EXP-2026-0902',
    tenantId: 'tenant-a',
    applicantId: users.alice.id,
    applicant: users.alice.username,
    claimType: 'entertainment',
    item: '渠道伙伴招待费',
    amount: 1680,
    costCenter: 'CC-SALES-02',
    expensePolicyId: 'POL-ENT-2026',
    vendorId: 'vendor-hotel-hz',
    businessLine: 'channel-growth',
    approvalTaskId: 'task-rb-0902',
    treasuryDisbursementRef: 'dj-rb-0902',
    settlementBatchId: null,
    payoutChannel: 'cmb-corporate',
    status: 'submitted',
    approvalHistory: [],
    createdAt: '2026-04-06T07:15:00.000Z',
    submittedAt: '2026-04-06T07:45:00.000Z',
  })

  transfers.set('pr-a-0701', {
    id: 'pr-a-0701',
    paymentRequestNo: 'PAYREQ-2026-0701',
    tenantId: 'tenant-a',
    createdBy: 'manager',
    fromAccount: 'treasury-main',
    toAccount: 'vendor-clearing',
    amount: 62000,
    status: 'reviewed',
    paymentCategory: 'channel-rebate',
    beneficiaryVendorId: 'vendor-ic-301',
    cashPoolId: 'cashpool-east-01',
    settlementChannel: 'cmb-enterprise',
    reviewTaskId: 'task-pr-0701',
    approvals: [{ by: 'manager', at: '2026-04-02T09:10:00.000Z', mode: 'standard' }],
    createdAt: '2026-04-02T08:40:00.000Z',
  })

  payments.set('pay-0901', {
    id: 'pay-0901',
    transferId: null,
    reimbursementId: 'ec-a-0901',
    actor: 'finance',
    amount: 2860,
    treasuryBatchNo: 'TB-20260407-001',
    disbursementReference: 'dj-rb-0901',
    createdAt: '2026-04-03T10:10:00.000Z',
  })

  reimbursementSeq = 903
  transferSeq = 702
  paymentSeq = 902
}

function resetPlatformState() {
  seedPlatformState()
}

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

function getUserFromRequest(req) {
  const explicitUser = req.headers['x-demo-user']
  if (typeof explicitUser === 'string' && users[explicitUser]) {
    return users[explicitUser]
  }

  const authorization = req.headers.authorization
  if (typeof authorization === 'string' && authorization.startsWith('Bearer ')) {
    const token = authorization.slice('Bearer '.length).trim()
    if (token.startsWith('portal-') && token.endsWith('-session')) {
      const username = token.slice('portal-'.length, '-session'.length * -1)
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
    department: user.department,
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

function createReimbursement(actor, body) {
  const sequence = padSequence(reimbursementSeq++)
  const id = `ec-a-${sequence}`
  const item = typeof body.item === 'string' ? body.item : '差旅报销'
  const amount = Number(body.amount || 0)
  const record = {
    id,
    claimNo: `EXP-2026-${sequence}`,
    tenantId: actor.tenantId,
    applicantId: actor.id,
    applicant: actor.username,
    claimType: typeof body.claimType === 'string' ? body.claimType : 'travel',
    item,
    amount,
    costCenter: typeof body.costCenter === 'string' ? body.costCenter : 'CC-SALES-01',
    expensePolicyId:
      typeof body.expensePolicyId === 'string' ? body.expensePolicyId : 'POL-TRAVEL-2026',
    vendorId: typeof body.vendorId === 'string' ? body.vendorId : 'vendor-air-cn',
    businessLine: typeof body.businessLine === 'string' ? body.businessLine : actor.department,
    approvalTaskId: `task-rb-${sequence}`,
    treasuryDisbursementRef: `dj-rb-${sequence}`,
    settlementBatchId: null,
    payoutChannel: 'cmb-corporate',
    status: 'draft',
    approvalHistory: [],
    createdAt: new Date().toISOString(),
  }
  reimbursements.set(id, record)
  return record
}

function createTransfer(actor, body) {
  const sequence = padSequence(transferSeq++)
  const id = `pr-a-${sequence}`
  const record = {
    id,
    paymentRequestNo: `PAYREQ-2026-${sequence}`,
    tenantId: actor.tenantId,
    createdBy: actor.username,
    fromAccount: body.fromAccount || `${actor.username}-wallet`,
    toAccount: body.toAccount || 'merchant-main',
    amount: Number(body.amount || 0),
    paymentCategory: body.paymentCategory || 'vendor-settlement',
    beneficiaryVendorId: body.beneficiaryVendorId || 'vendor-default',
    cashPoolId: body.cashPoolId || 'cashpool-east-01',
    settlementChannel: body.settlementChannel || 'cmb-enterprise',
    reviewTaskId: `task-pr-${sequence}`,
    status: 'draft',
    approvals: [],
    createdAt: new Date().toISOString(),
  }
  transfers.set(id, record)
  return record
}

function createPayment(body, actor) {
  const sequence = padSequence(paymentSeq++)
  const id = `pay-${sequence}`
  const record = {
    id,
    transferId: body.transferId,
    reimbursementId: body.reimbursementId,
    actor: actor.username,
    amount: Number(body.amount || 0),
    treasuryBatchNo: `TB-20260407-${sequence}`,
    disbursementReference: body.disbursementReference || `dj-${sequence}`,
    createdAt: new Date().toISOString(),
  }
  payments.set(id, record)
  return record
}

function getPathId(pathname, regex) {
  const match = pathname.match(regex)
  return match ? match[1] : null
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
      service: 'enterprise-finance-service',
      operationModes: ['portal', 'mobile-workbench', 'partner-integration', 'customer-service'],
      businessDomains: ['reimbursement', 'transfer', 'coupon', 'order'],
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
    const targetUser = users[username]
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
      users: Object.values(users),
      orders: Array.from(orders.values()),
      reimbursements: Array.from(reimbursements.values()),
      transfers: Array.from(transfers.values()),
      payments: Array.from(payments.values()),
      coupons: Array.from(coupons.values()),
    })
    logRequest(req, 200, user)
    return
  }

  if (req.method === 'POST' && url.pathname === '/api/platform/reset') {
    if (user.role !== 'admin') {
      forbidden(res, 'Only admin can reset platform state')
      logRequest(req, 403, user)
      return
    }

    resetPlatformState()
    respondScenario(res, req, 200, user, 'platform-reset', {
      reset: true,
      service: 'enterprise-finance-service',
    })
    return
  }

  if (req.method === 'POST' && url.pathname === '/api/expense/claims/create') {
    const body = await parseBody(req).catch(error => ({ __error: error.message }))
    if (body.__error) {
      badRequest(res, body.__error)
      logRequest(req, 400, user)
      return
    }
    const record = createReimbursement(user, body)
    respondScenario(res, req, 200, user, 'reimbursement-create-draft', {
      reimbursement: record,
    })
    return
  }

  const reimbursementSubmitId = getPathId(url.pathname, /^\/api\/expense\/claims\/([^/]+)\/submit$/)
  if (req.method === 'POST' && reimbursementSubmitId) {
    const record = reimbursements.get(reimbursementSubmitId)
    if (!record) {
      notFound(res, 'Reimbursement not found')
      logRequest(req, 404, user)
      return
    }
    if (record.applicantId !== user.id) {
      forbidden(res, 'Only applicant can submit reimbursement')
      logRequest(req, 403, user)
      return
    }
    if (record.status !== 'draft') {
      badRequest(res, 'Only draft reimbursement can be submitted')
      logRequest(req, 400, user)
      return
    }
    record.status = 'submitted'
    record.submittedAt = new Date().toISOString()
    respondScenario(res, req, 200, user, 'reimbursement-submit', {
      reimbursement: record,
    })
    return
  }

  const reimbursementApproveId = getPathId(url.pathname, /^\/api\/workflow\/expense-claims\/([^/]+)\/approve$/)
  if (req.method === 'POST' && reimbursementApproveId) {
    const record = reimbursements.get(reimbursementApproveId)
    if (!record) {
      notFound(res, 'Reimbursement not found')
      logRequest(req, 404, user)
      return
    }
    if (user.role !== 'manager') {
      forbidden(res, 'Only manager can approve expense claim')
      logRequest(req, 403, user)
      return
    }
    if (record.tenantId !== user.tenantId || record.status !== 'submitted') {
      badRequest(res, 'Expense claim is not ready for approval')
      logRequest(req, 400, user)
      return
    }
    record.status = 'approved'
    record.approvalHistory.push({ by: user.username, at: new Date().toISOString(), mode: 'standard' })
    respondScenario(res, req, 200, user, 'reimbursement-approve', {
      reimbursement: record,
    })
    return
  }

  const reimbursementDecisionId = getPathId(
    url.pathname,
    /^\/api\/mobile\/workbench\/expense-claims\/([^/]+)\/task-complete$/,
  )
  if (req.method === 'POST' && reimbursementDecisionId) {
    const record = reimbursements.get(reimbursementDecisionId)
    if (!record) {
      notFound(res, 'Reimbursement not found')
      logRequest(req, 404, user)
      return
    }
    record.status = 'approved'
    record.approvalHistory.push({ by: user.username, at: new Date().toISOString(), mode: 'alternate' })
    record.mobileTaskCompletedBy = user.username
    respondScenario(res, req, 200, user, 'mobile-expense-task-complete', {
      reimbursement: record,
    })
    return
  }

  const reimbursementDisburseId = getPathId(
    url.pathname,
    /^\/api\/treasury\/expense-claims\/([^/]+)\/disburse$/,
  )
  if (req.method === 'POST' && reimbursementDisburseId) {
    const record = reimbursements.get(reimbursementDisburseId)
    if (!record) {
      notFound(res, 'Reimbursement not found')
      logRequest(req, 404, user)
      return
    }
    if (user.role !== 'finance') {
      forbidden(res, 'Only finance can pay expense claim')
      logRequest(req, 403, user)
      return
    }
    if (record.tenantId !== user.tenantId || record.status !== 'approved') {
      badRequest(res, 'Expense claim must be approved before payment')
      logRequest(req, 400, user)
      return
    }
    if (record.paidAt) {
      badRequest(res, 'Reimbursement already paid')
      logRequest(req, 400, user)
      return
    }
    record.status = 'paid'
    record.paidAt = new Date().toISOString()
    record.settlementBatchId = `TB-20260407-${padSequence(paymentSeq)}`
    respondScenario(res, req, 200, user, 'reimbursement-disburse', {
      reimbursement: record,
      payment: createPayment(
        {
          reimbursementId: record.id,
          amount: record.amount,
          disbursementReference: record.treasuryDisbursementRef,
        },
        user,
      ),
    })
    return
  }

  const reimbursementReleaseId = getPathId(
    url.pathname,
    /^\/api\/integrations\/treasury\/expense-claims\/([^/]+)\/execute$/,
  )
  if (req.method === 'POST' && reimbursementReleaseId) {
    const record = reimbursements.get(reimbursementReleaseId)
    if (!record) {
      notFound(res, 'Reimbursement not found')
      logRequest(req, 404, user)
      return
    }
    record.status = 'paid'
    record.paidCount = (record.paidCount || 0) + 1
    record.lastPaidAt = new Date().toISOString()
    record.integrationChannel = 'legacy-treasury-adapter'
    respondScenario(res, req, 200, user, 'treasury-disbursement-execute', {
      reimbursement: record,
      payment: createPayment(
        {
          reimbursementId: record.id,
          amount: record.amount,
          disbursementReference: record.treasuryDisbursementRef,
        },
        user,
      ),
    })
    return
  }

  if (req.method === 'POST' && url.pathname === '/api/treasury/payment-requests/create') {
    const body = await parseBody(req).catch(error => ({ __error: error.message }))
    if (body.__error) {
      badRequest(res, body.__error)
      logRequest(req, 400, user)
      return
    }
    const transfer = createTransfer(user, body)
    respondScenario(res, req, 200, user, 'transfer-create-draft', {
      transfer,
    })
    return
  }

  const transferReviewId = getPathId(
    url.pathname,
    /^\/api\/treasury\/payment-requests\/([^/]+)\/review$/,
  )
  if (req.method === 'POST' && transferReviewId) {
    const transfer = transfers.get(transferReviewId)
    if (!transfer) {
      notFound(res, 'Transfer not found')
      logRequest(req, 404, user)
      return
    }
    if (user.role !== 'manager') {
      forbidden(res, 'Only manager can review payment request')
      logRequest(req, 403, user)
      return
    }
    if (transfer.tenantId !== user.tenantId || transfer.status !== 'draft') {
      badRequest(res, 'Payment request is not ready for review')
      logRequest(req, 400, user)
      return
    }
    transfer.status = 'reviewed'
    transfer.approvals.push({ by: user.username, at: new Date().toISOString(), mode: 'standard' })
    respondScenario(res, req, 200, user, 'transfer-review', {
      transfer,
    })
    return
  }

  const transferConfirmId = getPathId(
    url.pathname,
    /^\/api\/treasury\/payment-requests\/([^/]+)\/confirm$/,
  )
  if (req.method === 'POST' && transferConfirmId) {
    const transfer = transfers.get(transferConfirmId)
    if (!transfer) {
      notFound(res, 'Transfer not found')
      logRequest(req, 404, user)
      return
    }
    if (user.role !== 'finance') {
      forbidden(res, 'Only finance can confirm payment request')
      logRequest(req, 403, user)
      return
    }
    if (transfer.tenantId !== user.tenantId || transfer.status !== 'reviewed') {
      badRequest(res, 'Payment request must be reviewed before confirmation')
      logRequest(req, 400, user)
      return
    }
    if (transfer.confirmedAt) {
      badRequest(res, 'Transfer already confirmed')
      logRequest(req, 400, user)
      return
    }
    transfer.status = 'confirmed'
    transfer.confirmedAt = new Date().toISOString()
    respondScenario(res, req, 200, user, 'transfer-confirm', {
      transfer,
    })
    return
  }

  if (req.method === 'POST' && url.pathname === '/api/mobile/treasury/payment-confirmations') {
    const body = await parseBody(req).catch(error => ({ __error: error.message }))
    if (body.__error) {
      badRequest(res, body.__error)
      logRequest(req, 400, user)
      return
    }
    const transferId =
      typeof body.transferId === 'string' ? body.transferId : `pr-inline-${Date.now()}`
    const transfer = transfers.get(transferId) || {
      id: transferId,
      paymentRequestNo: `PAYREQ-2026-${Date.now()}`,
      tenantId: user.tenantId,
      createdBy: user.username,
      fromAccount: body.fromAccount || `${user.username}-wallet`,
      toAccount: body.toAccount || 'merchant-main',
      amount: Number(body.amount || 0),
      paymentCategory: body.paymentCategory || 'vendor-settlement',
      beneficiaryVendorId: body.beneficiaryVendorId || 'vendor-default',
      cashPoolId: body.cashPoolId || 'cashpool-east-01',
      settlementChannel: body.settlementChannel || 'cmb-enterprise',
      reviewTaskId: null,
      status: 'draft',
      approvals: [],
    }
    transfer.status = 'confirmed'
    transfer.confirmedAt = new Date().toISOString()
    transfer.confirmedBy = user.username
    transfers.set(transferId, transfer)
    transfer.mobileConfirmed = true
    respondScenario(res, req, 200, user, 'mobile-payment-confirmation', {
      transfer,
    })
    return
  }

  const couponApplyCode = getPathId(url.pathname, /^\/api\/marketing\/coupons\/([^/]+)\/claim$/)
  if (req.method === 'POST' && couponApplyCode) {
    const coupon = coupons.get(couponApplyCode)
    if (!coupon) {
      notFound(res, 'Coupon not found')
      logRequest(req, 404, user)
      return
    }
    const alreadyUsed = coupon.uses.some(item => item.userId === user.id)
    if (alreadyUsed) {
      badRequest(res, 'Coupon already redeemed by this user')
      logRequest(req, 400, user)
      return
    }
    const use = { userId: user.id, username: user.username, redeemedAt: new Date().toISOString(), mode: 'standard' }
    coupon.uses.push(use)
    respondScenario(res, req, 200, user, 'coupon-apply', {
      couponCode: coupon.code,
      redemption: use,
    })
    return
  }

  const couponActivateCode = getPathId(
    url.pathname,
    /^\/api\/channel-partner\/campaigns\/([^/]+)\/activate$/,
  )
  if (req.method === 'POST' && couponActivateCode) {
    const coupon = coupons.get(couponActivateCode)
    if (!coupon) {
      notFound(res, 'Coupon not found')
      logRequest(req, 404, user)
      return
    }
    const use = { userId: user.id, username: user.username, redeemedAt: new Date().toISOString(), mode: 'alternate' }
    coupon.uses.push(use)
    respondScenario(res, req, 200, user, 'campaign-activate', {
      couponCode: coupon.code,
      redemption: use,
      redemptionCountForUser: coupon.uses.filter(item => item.userId === user.id).length,
    })
    return
  }

  const orderDetailId = getPathId(url.pathname, /^\/api\/customer\/orders\/([^/]+)\/detail$/)
  if (req.method === 'GET' && orderDetailId) {
    const order = orders.get(orderDetailId)
    if (!order) {
      notFound(res, 'Order not found')
      logRequest(req, 404, user)
      return
    }
    const isAllowed = order.ownerId === user.id || order.tenantId === user.tenantId || user.role === 'admin'
    if (!isAllowed) {
      forbidden(res, 'Order is outside your scope')
      logRequest(req, 403, user)
      return
    }
    respondScenario(res, req, 200, user, 'order-detail', {
      order,
    })
    return
  }

  const orderSummaryId = getPathId(
    url.pathname,
    /^\/api\/customer-service\/orders\/([^/]+)\/snapshot$/,
  )
  if (req.method === 'GET' && orderSummaryId) {
    const order = orders.get(orderSummaryId)
    if (!order) {
      notFound(res, 'Order not found')
      logRequest(req, 404, user)
      return
    }
    respondScenario(res, req, 200, user, 'order-snapshot', {
      order,
    })
    return
  }

  notFound(res)
  logRequest(req, 404, user)
}

if (process.argv.includes('--help')) {
  console.log(`enterprise-finance-service

Usage:
  npm run demo:biz-service
  DEMO_BIZ_PORT=7788 DEMO_BIZ_HOST=127.0.0.1 node scripts/demo-business-service.mjs

Accounts:
  alice, bob, manager, finance, admin

Auth:
  Authorization: Bearer portal-{username}-session
  x-demo-user: {username}

Primary routes:
  POST /api/expense/claims/create
  POST /api/workflow/expense-claims/{id}/approve
  POST /api/treasury/payment-requests/{id}/confirm
  GET  /api/customer/orders/{id}/detail

Alternate routes:
  POST /api/mobile/workbench/expense-claims/{id}/task-complete
  POST /api/integrations/treasury/expense-claims/{id}/execute
  POST /api/mobile/treasury/payment-confirmations
  GET  /api/customer-service/orders/{id}/snapshot
`)
  process.exit(0)
}

seedPlatformState()

const server = http.createServer(async (req, res) => {
  try {
    await handleRequest(req, res)
  } catch (error) {
    console.error('enterprise-finance-service error', error)
    sendJson(res, 500, {
      success: false,
      message: error instanceof Error ? error.message : 'Internal server error',
    })
  }
})

server.listen(port, host, () => {
  console.log(`enterprise-finance-service listening on http://${host}:${port}`)
  console.log(`web portal: http://${host}:${port}/`)
  console.log('accounts: alice, bob, manager, finance, admin')
  console.log('use Authorization: Bearer portal-alice-session or x-demo-user: alice')
  console.log('business domains: reimbursement, transfer, coupon, order')
})
