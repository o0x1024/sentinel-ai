#!/usr/bin/env node

const baseUrl = process.env.SYSTEM_AGENT_EVAL_BASE_URL || 'http://127.0.0.1:7788';

function headersFor(user, hasBody = false) {
  return {
    authorization: `Bearer portal-${user}-session`,
    'x-demo-user': user,
    ...(hasBody ? { 'content-type': 'application/json' } : {}),
  };
}

async function requestJson(path, { method = 'GET', user = 'admin', body } = {}) {
  const response = await fetch(`${baseUrl}${path}`, {
    method,
    headers: headersFor(user, body != null),
    body: body != null ? JSON.stringify(body) : undefined,
  });
  const text = await response.text();
  let data = null;
  try {
    data = text ? JSON.parse(text) : null;
  } catch {
    data = text;
  }
  return {
    method,
    path,
    user,
    status: response.status,
    ok: response.ok,
    data,
  };
}

async function resetState() {
  return requestJson('/api/platform/reset', {
    method: 'POST',
    user: 'admin',
  });
}

const scenarios = [
  {
    id: 'reimbursement-primary',
    title: '报销协作流程',
    expectation: 'negative_control',
    notes: '正常多角色协作流程，不应被直接判为正式漏洞。',
    run: async () => {
      const create = await requestJson('/api/reimbursements/create', {
        method: 'POST',
        user: 'alice',
        body: { item: '客户拜访差旅', amount: 860 },
      });
      const reimbursementId = create.data?.reimbursement?.id;
      return [
        create,
        await requestJson(`/api/reimbursements/${reimbursementId}/submit`, {
          method: 'POST',
          user: 'alice',
        }),
        await requestJson(`/api/reimbursements/${reimbursementId}/approve`, {
          method: 'POST',
          user: 'manager',
        }),
        await requestJson(`/api/reimbursements/${reimbursementId}/disburse`, {
          method: 'POST',
          user: 'finance',
        }),
      ];
    },
  },
  {
    id: 'reimbursement-secondary',
    title: '报销处理入口',
    expectation: 'logic_candidate',
    notes: '同一单据通过另一套入口推进审批和出款，预期至少形成 workflow/logic 类候选。',
    run: async () => {
      const create = await requestJson('/api/reimbursements/create', {
        method: 'POST',
        user: 'alice',
        body: { item: '商务招待费', amount: 1280 },
      });
      const reimbursementId = create.data?.reimbursement?.id;
      return [
        create,
        await requestJson(`/api/reimbursements/${reimbursementId}/decision`, {
          method: 'POST',
          user: 'alice',
        }),
        await requestJson(`/api/reimbursements/${reimbursementId}/release`, {
          method: 'POST',
          user: 'alice',
        }),
        await requestJson(`/api/reimbursements/${reimbursementId}/release`, {
          method: 'POST',
          user: 'alice',
        }),
      ];
    },
  },
  {
    id: 'transfer-primary',
    title: '转账协作流程',
    expectation: 'negative_control',
    notes: '标准转账创建、审核、确认流程，用来对照辅助入口的状态推进。',
    run: async () => {
      const create = await requestJson('/api/transfers/create', {
        method: 'POST',
        user: 'alice',
        body: { fromAccount: 'alice-wallet', toAccount: 'supplier-main', amount: 5200 },
      });
      const transferId = create.data?.transfer?.id;
      return [
        create,
        await requestJson(`/api/transfers/${transferId}/review`, {
          method: 'POST',
          user: 'manager',
        }),
        await requestJson(`/api/transfers/${transferId}/confirm`, {
          method: 'POST',
          user: 'finance',
        }),
      ];
    },
  },
  {
    id: 'transfer-secondary',
    title: '转账确认入口',
    expectation: 'logic_candidate',
    notes: '同一确认入口被重复触发，预期形成 workflow/race 类候选。',
    run: async () => [
      await requestJson('/api/transfers/confirmation', {
        method: 'POST',
        user: 'alice',
        body: { transferId: 'tr-eval-1', amount: 5200, toAccount: 'supplier-main' },
      }),
      await requestJson('/api/transfers/confirmation', {
        method: 'POST',
        user: 'alice',
        body: { transferId: 'tr-eval-1', amount: 5200, toAccount: 'supplier-main' },
      }),
    ],
  },
  {
    id: 'activity-and-order',
    title: '活动与订单访问',
    expectation: 'mixed_candidate',
    notes: '连续激活活动并访问跨租户订单，预期出现 single-use/ownership 相关候选。',
    run: async () => [
      await requestJson('/api/coupons/SPRING-2026/apply', {
        method: 'POST',
        user: 'alice',
      }),
      await requestJson('/api/campaigns/OPS-BONUS/activate', {
        method: 'POST',
        user: 'alice',
      }),
      await requestJson('/api/campaigns/OPS-BONUS/activate', {
        method: 'POST',
        user: 'alice',
      }),
      await requestJson('/api/orders/ord-a-1001/detail', {
        method: 'GET',
        user: 'alice',
      }),
      await requestJson('/api/orders/ord-b-2001/summary', {
        method: 'GET',
        user: 'alice',
      }),
    ],
  },
];

async function runScenario(scenario) {
  const requests = await scenario.run();
  return {
    id: scenario.id,
    title: scenario.title,
    expectation: scenario.expectation,
    notes: scenario.notes,
    requests,
  };
}

async function main() {
  if (process.argv.includes('--help')) {
    console.log(`system-agent-eval

Usage:
  npm run demo:eval
  SYSTEM_AGENT_EVAL_BASE_URL=http://127.0.0.1:7788 node scripts/system-agent-eval.mjs

Description:
  Reset the local enterprise finance service, run baseline and candidate scenarios,
  and output a structured evaluation fixture for manual comparison with Sentinel findings.
`);
    return;
  }

  const startedAt = new Date().toISOString();
  const reset = await resetState();
  const results = [];

  for (const scenario of scenarios) {
    // 串行执行，保证行为和流量窗口更稳定。
    // eslint-disable-next-line no-await-in-loop
    results.push(await runScenario(scenario));
  }

  const report = {
    startedAt,
    baseUrl,
    reset,
    scenarios: results,
    summary: {
      negativeControls: results.filter(item => item.expectation === 'negative_control').map(item => item.id),
      candidateExpectations: results.filter(item => item.expectation !== 'negative_control').map(item => item.id),
    },
  };

  console.log(JSON.stringify(report, null, 2));
}

main().catch(error => {
  console.error('system-agent-eval failed', error);
  process.exitCode = 1;
});
