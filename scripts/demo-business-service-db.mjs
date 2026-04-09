import { execFileSync } from 'node:child_process'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const __dirname = path.dirname(fileURLToPath(import.meta.url))

export const demoDbPath =
  process.env.DEMO_BIZ_DB || path.join(__dirname, '.demo-business-service.sqlite')

const sqlite = (args) => {
  try {
    return execFileSync('sqlite3', args, { encoding: 'utf8' })
  } catch (error) {
    const message =
      error instanceof Error && 'stderr' in error && typeof error.stderr === 'string'
        ? error.stderr.trim()
        : error instanceof Error
          ? error.message
          : String(error)
    throw new Error(`sqlite3 command failed: ${message}`)
  }
}

const sqlText = value => `'${String(value).replaceAll("'", "''")}'`
const sqlNullableText = value => (value == null ? 'NULL' : sqlText(value))
const sqlNumber = value => {
  const numeric = Number(value)
  return Number.isFinite(numeric) ? String(numeric) : '0'
}

const queryAll = sql => {
  const raw = sqlite(['-json', demoDbPath, `PRAGMA foreign_keys = ON; ${sql}`]).trim()
  return raw ? JSON.parse(raw) : []
}

const queryOne = sql => queryAll(sql)[0] || null

const execute = sql => {
  sqlite([demoDbPath, `PRAGMA foreign_keys = ON; ${sql}`])
}

const schemaSql = `
CREATE TABLE IF NOT EXISTS demo_users (
  id TEXT PRIMARY KEY,
  username TEXT NOT NULL UNIQUE,
  display_name TEXT NOT NULL,
  role TEXT NOT NULL,
  tenant_id TEXT NOT NULL,
  department TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS expense_claims (
  id TEXT PRIMARY KEY,
  claim_no TEXT NOT NULL UNIQUE,
  tenant_id TEXT NOT NULL,
  applicant_id TEXT NOT NULL,
  title TEXT NOT NULL,
  claim_type TEXT NOT NULL,
  amount REAL NOT NULL,
  cost_center TEXT NOT NULL,
  policy_code TEXT NOT NULL,
  vendor_code TEXT NOT NULL,
  business_line TEXT NOT NULL,
  status TEXT NOT NULL,
  workflow_stage TEXT NOT NULL,
  payout_ref TEXT NOT NULL,
  created_at TEXT NOT NULL,
  submitted_at TEXT,
  approved_at TEXT,
  approved_by TEXT,
  paid_at TEXT,
  paid_by TEXT,
  FOREIGN KEY (applicant_id) REFERENCES demo_users(id)
);

CREATE TABLE IF NOT EXISTS expense_claim_events (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  claim_id TEXT NOT NULL,
  action TEXT NOT NULL,
  actor_id TEXT NOT NULL,
  actor_role TEXT NOT NULL,
  channel TEXT NOT NULL,
  note TEXT,
  created_at TEXT NOT NULL,
  FOREIGN KEY (claim_id) REFERENCES expense_claims(id) ON DELETE CASCADE,
  FOREIGN KEY (actor_id) REFERENCES demo_users(id)
);
`

const seedUsers = [
  ['u-alice', 'alice', 'Alice Chen', 'employee', 'tenant-a', 'sales'],
  ['u-carol', 'carol', 'Carol Lin', 'employee', 'tenant-a', 'sales'],
  ['u-bob', 'bob', 'Bob Wang', 'employee', 'tenant-b', 'ops'],
  ['u-manager', 'manager', 'Grace Manager', 'manager', 'tenant-a', 'sales'],
  ['u-finance', 'finance', 'Frank Finance', 'finance', 'tenant-a', 'finance'],
  ['u-admin', 'admin', 'Admin User', 'admin', 'tenant-a', 'security']
]

const seedClaims = [
  {
    id: 'ec-a-1001',
    claimNo: 'EXP-2026-1001',
    tenantId: 'tenant-a',
    applicantId: 'u-alice',
    title: '华东客户巡检差旅',
    claimType: 'travel',
    amount: 1280,
    costCenter: 'CC-SALES-01',
    policyCode: 'POL-TRAVEL-2026',
    vendorCode: 'vendor-air-cn',
    businessLine: 'regional-sales',
    status: 'draft',
    workflowStage: 'draft',
    payoutRef: 'pay-ref-1001',
    createdAt: '2026-04-08T09:10:00.000Z',
    submittedAt: null,
    approvedAt: null,
    approvedBy: null,
    paidAt: null,
    paidBy: null
  },
  {
    id: 'ec-a-1002',
    claimNo: 'EXP-2026-1002',
    tenantId: 'tenant-a',
    applicantId: 'u-carol',
    title: '渠道招待费报销',
    claimType: 'entertainment',
    amount: 1860,
    costCenter: 'CC-SALES-02',
    policyCode: 'POL-ENT-2026',
    vendorCode: 'vendor-hotel-hz',
    businessLine: 'channel-growth',
    status: 'submitted',
    workflowStage: 'awaiting_manager_approval',
    payoutRef: 'pay-ref-1002',
    createdAt: '2026-04-08T10:15:00.000Z',
    submittedAt: '2026-04-08T10:35:00.000Z',
    approvedAt: null,
    approvedBy: null,
    paidAt: null,
    paidBy: null
  },
  {
    id: 'ec-a-1003',
    claimNo: 'EXP-2026-1003',
    tenantId: 'tenant-a',
    applicantId: 'u-alice',
    title: '解决方案顾问出差住宿',
    claimType: 'travel',
    amount: 2460,
    costCenter: 'CC-SALES-01',
    policyCode: 'POL-TRAVEL-2026',
    vendorCode: 'vendor-hotel-sh',
    businessLine: 'solution-sales',
    status: 'approved',
    workflowStage: 'awaiting_finance_disbursement',
    payoutRef: 'pay-ref-1003',
    createdAt: '2026-04-07T07:00:00.000Z',
    submittedAt: '2026-04-07T07:25:00.000Z',
    approvedAt: '2026-04-07T08:00:00.000Z',
    approvedBy: 'u-manager',
    paidAt: null,
    paidBy: null
  },
  {
    id: 'ec-b-2001',
    claimNo: 'EXP-2026-2001',
    tenantId: 'tenant-b',
    applicantId: 'u-bob',
    title: '欧洲售后备件采购差旅',
    claimType: 'travel',
    amount: 980,
    costCenter: 'CC-OPS-01',
    policyCode: 'POL-TRAVEL-2026',
    vendorCode: 'vendor-train-eu',
    businessLine: 'global-ops',
    status: 'submitted',
    workflowStage: 'awaiting_manager_approval',
    payoutRef: 'pay-ref-2001',
    createdAt: '2026-04-07T11:00:00.000Z',
    submittedAt: '2026-04-07T11:20:00.000Z',
    approvedAt: null,
    approvedBy: null,
    paidAt: null,
    paidBy: null
  }
]

const seedEvents = [
  ['ec-a-1002', 'submit', 'u-carol', 'employee', 'portal', '员工正常提交', '2026-04-08T10:35:00.000Z'],
  ['ec-a-1003', 'submit', 'u-alice', 'employee', 'portal', '员工正常提交', '2026-04-07T07:25:00.000Z'],
  ['ec-a-1003', 'approve', 'u-manager', 'manager', 'workflow', '经理审批通过', '2026-04-07T08:00:00.000Z'],
  ['ec-b-2001', 'submit', 'u-bob', 'employee', 'portal', '员工正常提交', '2026-04-07T11:20:00.000Z']
]

const claimSelectSql = `
SELECT
  c.id,
  c.claim_no AS claimNo,
  c.tenant_id AS tenantId,
  c.applicant_id AS applicantId,
  u.username AS applicantUsername,
  u.display_name AS applicantDisplayName,
  c.title,
  c.claim_type AS claimType,
  c.amount,
  c.cost_center AS costCenter,
  c.policy_code AS policyCode,
  c.vendor_code AS vendorCode,
  c.business_line AS businessLine,
  c.status,
  c.workflow_stage AS workflowStage,
  c.payout_ref AS payoutRef,
  c.created_at AS createdAt,
  c.submitted_at AS submittedAt,
  c.approved_at AS approvedAt,
  c.approved_by AS approvedBy,
  c.paid_at AS paidAt,
  c.paid_by AS paidBy
FROM expense_claims c
JOIN demo_users u ON u.id = c.applicant_id
`

const eventSelectSql = `
SELECT
  e.id,
  e.claim_id AS claimId,
  e.action,
  e.actor_id AS actorId,
  actor.username AS actorUsername,
  e.actor_role AS actorRole,
  e.channel,
  e.note,
  e.created_at AS createdAt
FROM expense_claim_events e
JOIN demo_users actor ON actor.id = e.actor_id
`

const nextClaimSequence = () => {
  const row = queryOne(`
    SELECT COALESCE(MAX(CAST(SUBSTR(id, 6) AS INTEGER)), 1003) + 1 AS nextId
    FROM expense_claims
    WHERE id LIKE 'ec-a-%'
  `)
  return Number(row?.nextId || 1004)
}

const normalizeRole = value => String(value || '').trim()

const mapUser = row =>
  row
    ? {
        id: row.id,
        username: row.username,
        displayName: row.display_name || row.displayName,
        role: row.role,
        tenantId: row.tenant_id || row.tenantId,
        department: row.department
      }
    : null

export const ensureDemoDatabase = ({ reset = false } = {}) => {
  execute(schemaSql)
  const userCount = Number(queryOne('SELECT COUNT(*) AS count FROM demo_users')?.count || 0)
  if (reset || userCount === 0) {
    resetDemoDatabase()
  }
}

export const resetDemoDatabase = () => {
  execute(`
    BEGIN;
    DELETE FROM expense_claim_events;
    DELETE FROM expense_claims;
    DELETE FROM demo_users;
    COMMIT;
  `)

  const userSql = seedUsers
    .map(
      ([id, username, displayName, role, tenantId, department]) =>
        `INSERT INTO demo_users (id, username, display_name, role, tenant_id, department)
         VALUES (${sqlText(id)}, ${sqlText(username)}, ${sqlText(displayName)}, ${sqlText(role)}, ${sqlText(tenantId)}, ${sqlText(department)});`
    )
    .join('\n')

  const claimSql = seedClaims
    .map(
      claim => `INSERT INTO expense_claims (
        id, claim_no, tenant_id, applicant_id, title, claim_type, amount, cost_center,
        policy_code, vendor_code, business_line, status, workflow_stage, payout_ref,
        created_at, submitted_at, approved_at, approved_by, paid_at, paid_by
      ) VALUES (
        ${sqlText(claim.id)}, ${sqlText(claim.claimNo)}, ${sqlText(claim.tenantId)}, ${sqlText(claim.applicantId)},
        ${sqlText(claim.title)}, ${sqlText(claim.claimType)}, ${sqlNumber(claim.amount)}, ${sqlText(claim.costCenter)},
        ${sqlText(claim.policyCode)}, ${sqlText(claim.vendorCode)}, ${sqlText(claim.businessLine)},
        ${sqlText(claim.status)}, ${sqlText(claim.workflowStage)}, ${sqlText(claim.payoutRef)},
        ${sqlText(claim.createdAt)}, ${sqlNullableText(claim.submittedAt)}, ${sqlNullableText(claim.approvedAt)},
        ${sqlNullableText(claim.approvedBy)}, ${sqlNullableText(claim.paidAt)}, ${sqlNullableText(claim.paidBy)}
      );`
    )
    .join('\n')

  const eventSql = seedEvents
    .map(
      ([claimId, action, actorId, actorRole, channel, note, createdAt]) =>
        `INSERT INTO expense_claim_events (claim_id, action, actor_id, actor_role, channel, note, created_at)
         VALUES (${sqlText(claimId)}, ${sqlText(action)}, ${sqlText(actorId)}, ${sqlText(actorRole)}, ${sqlText(channel)}, ${sqlText(note)}, ${sqlText(createdAt)});`
    )
    .join('\n')

  execute(`BEGIN; ${userSql} ${claimSql} ${eventSql} COMMIT;`)
}

export const listDemoUsers = () =>
  queryAll(`
    SELECT id, username, display_name, role, tenant_id, department
    FROM demo_users
    ORDER BY tenant_id, role, username
  `).map(mapUser)

export const getUserByUsername = username =>
  mapUser(
    queryOne(`
      SELECT id, username, display_name, role, tenant_id, department
      FROM demo_users
      WHERE username = ${sqlText(username)}
      LIMIT 1
    `)
  )

export const getPlatformSnapshot = () => ({
  dbPath: demoDbPath,
  users: listDemoUsers(),
  claims: queryAll(`${claimSelectSql} ORDER BY c.created_at DESC`),
  claimEvents: queryAll(`${eventSelectSql} ORDER BY e.created_at DESC`)
})

const getClaimById = claimId =>
  queryOne(`${claimSelectSql} WHERE c.id = ${sqlText(claimId)} LIMIT 1`)

export const getClaimDetail = claimId => {
  const claim = getClaimById(claimId)
  if (!claim) return null
  return {
    ...claim,
    events: queryAll(`${eventSelectSql} WHERE e.claim_id = ${sqlText(claimId)} ORDER BY e.created_at ASC`)
  }
}

export const listClaimsForPortalUser = user => {
  const role = normalizeRole(user.role)
  let whereClause = `WHERE c.tenant_id = ${sqlText(user.tenantId)}`
  if (role === 'employee') {
    whereClause += ` AND c.applicant_id = ${sqlText(user.id)}`
  }
  return queryAll(`${claimSelectSql} ${whereClause} ORDER BY c.created_at DESC`)
}

export const canViewPortalClaim = (user, claim) => {
  if (!claim) return false
  if (user.role === 'admin') return true
  if (user.role === 'employee') {
    return claim.applicantId === user.id
  }
  return claim.tenantId === user.tenantId
}

export const canUseCollabSummary = (user, claim) => {
  if (!claim) return false
  return claim.tenantId === user.tenantId || user.role === 'admin'
}

const insertClaimEvent = ({ claimId, action, actor, channel, note }) => {
  execute(`
    INSERT INTO expense_claim_events (claim_id, action, actor_id, actor_role, channel, note, created_at)
    VALUES (
      ${sqlText(claimId)},
      ${sqlText(action)},
      ${sqlText(actor.id)},
      ${sqlText(actor.role)},
      ${sqlText(channel)},
      ${sqlNullableText(note || null)},
      ${sqlText(new Date().toISOString())}
    );
  `)
}

export const createExpenseClaim = (actor, body) => {
  const sequence = nextClaimSequence()
  const id = `ec-a-${String(sequence).padStart(4, '0')}`
  const claim = {
    id,
    claimNo: `EXP-2026-${String(sequence).padStart(4, '0')}`,
    tenantId: actor.tenantId,
    applicantId: actor.id,
    title: typeof body.title === 'string' ? body.title : typeof body.item === 'string' ? body.item : '差旅报销',
    claimType: typeof body.claimType === 'string' ? body.claimType : 'travel',
    amount: Number(body.amount || 0),
    costCenter: typeof body.costCenter === 'string' ? body.costCenter : 'CC-SALES-01',
    policyCode: typeof body.policyCode === 'string' ? body.policyCode : 'POL-TRAVEL-2026',
    vendorCode: typeof body.vendorCode === 'string' ? body.vendorCode : 'vendor-default',
    businessLine: typeof body.businessLine === 'string' ? body.businessLine : actor.department,
    status: 'draft',
    workflowStage: 'draft',
    payoutRef: `pay-ref-${String(sequence).padStart(4, '0')}`,
    createdAt: new Date().toISOString()
  }

  execute(`
    INSERT INTO expense_claims (
      id, claim_no, tenant_id, applicant_id, title, claim_type, amount, cost_center,
      policy_code, vendor_code, business_line, status, workflow_stage, payout_ref, created_at
    ) VALUES (
      ${sqlText(claim.id)}, ${sqlText(claim.claimNo)}, ${sqlText(claim.tenantId)}, ${sqlText(claim.applicantId)},
      ${sqlText(claim.title)}, ${sqlText(claim.claimType)}, ${sqlNumber(claim.amount)}, ${sqlText(claim.costCenter)},
      ${sqlText(claim.policyCode)}, ${sqlText(claim.vendorCode)}, ${sqlText(claim.businessLine)},
      'draft', 'draft', ${sqlText(claim.payoutRef)}, ${sqlText(claim.createdAt)}
    );
  `)

  insertClaimEvent({
    claimId: claim.id,
    action: 'create',
    actor,
    channel: 'portal',
    note: '员工创建报销单草稿'
  })

  return getClaimDetail(claim.id)
}

export const submitExpenseClaim = (actor, claimId) => {
  const claim = getClaimById(claimId)
  if (!claim) return { error: 'not_found' }
  if (claim.applicantId !== actor.id) return { error: 'forbidden_owner' }
  if (claim.status !== 'draft') return { error: 'invalid_status' }

  const now = new Date().toISOString()
  execute(`
    UPDATE expense_claims
    SET status = 'submitted',
        workflow_stage = 'awaiting_manager_approval',
        submitted_at = ${sqlText(now)}
    WHERE id = ${sqlText(claimId)};
  `)
  insertClaimEvent({
    claimId,
    action: 'submit',
    actor,
    channel: 'portal',
    note: '员工提交报销单'
  })
  return { claim: getClaimDetail(claimId) }
}

export const approveExpenseClaim = (actor, claimId, { bypass = false, channel = 'workflow' } = {}) => {
  const claim = getClaimById(claimId)
  if (!claim) return { error: 'not_found' }

  if (!bypass) {
    if (actor.role !== 'manager') return { error: 'forbidden_role' }
    if (claim.tenantId !== actor.tenantId) return { error: 'forbidden_tenant' }
    if (claim.status !== 'submitted') return { error: 'invalid_status' }
  } else if (claim.tenantId !== actor.tenantId && actor.role !== 'admin') {
    return { error: 'forbidden_tenant' }
  }

  const now = new Date().toISOString()
  execute(`
    UPDATE expense_claims
    SET status = 'approved',
        workflow_stage = 'awaiting_finance_disbursement',
        approved_at = ${sqlText(now)},
        approved_by = ${sqlText(actor.id)}
    WHERE id = ${sqlText(claimId)};
  `)
  insertClaimEvent({
    claimId,
    action: bypass ? 'quick_approve' : 'approve',
    actor,
    channel,
    note: bypass ? '移动工作台越权快速审批' : '经理审批通过'
  })
  return { claim: getClaimDetail(claimId) }
}

export const disburseExpenseClaim = (actor, claimId, { bypass = false, channel = 'treasury' } = {}) => {
  const claim = getClaimById(claimId)
  if (!claim) return { error: 'not_found' }

  if (!bypass) {
    if (actor.role !== 'finance') return { error: 'forbidden_role' }
    if (claim.tenantId !== actor.tenantId) return { error: 'forbidden_tenant' }
    if (claim.status !== 'approved') return { error: 'invalid_status' }
  } else if (claim.tenantId !== actor.tenantId && actor.role !== 'admin') {
    return { error: 'forbidden_tenant' }
  }

  const now = new Date().toISOString()
  execute(`
    UPDATE expense_claims
    SET status = 'paid',
        workflow_stage = 'completed',
        paid_at = ${sqlText(now)},
        paid_by = ${sqlText(actor.id)}
    WHERE id = ${sqlText(claimId)};
  `)
  insertClaimEvent({
    claimId,
    action: bypass ? 'release' : 'disburse',
    actor,
    channel,
    note: bypass ? '集成端直接放款，绕过标准审批' : '财务放款完成'
  })
  return { claim: getClaimDetail(claimId) }
}
