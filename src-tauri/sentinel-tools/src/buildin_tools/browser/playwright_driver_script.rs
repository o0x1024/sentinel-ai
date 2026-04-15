pub const PLAYWRIGHT_DRIVER_SCRIPT: &str = r#"
import readline from 'node:readline';

const playwrightModule = process.env.SENTINEL_PLAYWRIGHT_MODULE || 'playwright';
const { chromium } = await import(playwrightModule);

const networkLog = [];
let browser = null;
let page = null;

function normalizeText(value, limit = 4000) {
  if (!value) return '';
  return String(value).replace(/\s+/g, ' ').trim().slice(0, limit);
}

async function ensureBrowser(headless) {
  if (browser && page) return;
  browser = await chromium.launch({ headless });
  const context = await browser.newContext({
    ignoreHTTPSErrors: true,
  });
  page = await context.newPage();
  page.on('request', (request) => {
    networkLog.push({
      type: 'request',
      method: request.method(),
      url: request.url(),
      resourceType: request.resourceType(),
      ts: Date.now(),
    });
    if (networkLog.length > 200) networkLog.shift();
  });
  page.on('response', async (response) => {
    networkLog.push({
      type: 'response',
      status: response.status(),
      url: response.url(),
      ts: Date.now(),
    });
    if (networkLog.length > 200) networkLog.shift();
  });
}

async function snapshot() {
  const title = await page.title();
  const url = page.url();
  const text = normalizeText(await page.locator('body').innerText().catch(() => ''), 6000);
  const links = await page.locator('a').evaluateAll((nodes) =>
    nodes.slice(0, 20).map((node) => ({
      text: (node.textContent || '').replace(/\s+/g, ' ').trim().slice(0, 120),
      href: node.getAttribute('href') || '',
    }))
  ).catch(() => []);
  const buttons = await page.locator('button,input[type=submit]').evaluateAll((nodes) =>
    nodes.slice(0, 20).map((node) => ({
      text: (node.textContent || node.getAttribute('value') || '').replace(/\s+/g, ' ').trim().slice(0, 120),
      selector_hint: node.getAttribute('name') || node.getAttribute('id') || node.getAttribute('class') || '',
    }))
  ).catch(() => []);
  const inputs = await page.locator('input,textarea,select').evaluateAll((nodes) =>
    nodes.slice(0, 30).map((node) => ({
      tag: node.tagName.toLowerCase(),
      type: node.getAttribute('type') || '',
      name: node.getAttribute('name') || '',
      id: node.getAttribute('id') || '',
      placeholder: node.getAttribute('placeholder') || '',
    }))
  ).catch(() => []);

  return {
    title,
    url,
    text,
    links,
    buttons,
    inputs,
    network_log_preview: networkLog.slice(-20),
  };
}

async function handleCommand(command) {
  const action = command.action;
  const headless = command.headless !== false;
  if (action !== 'close') {
    await ensureBrowser(headless);
  }

  switch (action) {
    case 'launch':
      return { launched: true };
    case 'goto':
      await page.goto(command.url, {
        waitUntil: command.wait_until || 'load',
        timeout: (command.timeout_secs || 30) * 1000,
      });
      return await snapshot();
    case 'snapshot':
      return await snapshot();
    case 'click':
      await page.click(command.selector, {
        timeout: (command.timeout_secs || 30) * 1000,
      });
      return await snapshot();
    case 'fill':
      await page.fill(command.selector, command.value || '', {
        timeout: (command.timeout_secs || 30) * 1000,
      });
      return await snapshot();
    case 'eval':
      return {
        value: await page.evaluate(command.script),
      };
    case 'cookies':
      return {
        cookies: await page.context().cookies(),
      };
    case 'network_log':
      return {
        entries: networkLog.slice(-(command.network_limit || 30)),
      };
    case 'close':
      if (browser) {
        await browser.close();
        browser = null;
        page = null;
      }
      return { closed: true };
    default:
      throw new Error(`unsupported browser action: ${action}`);
  }
}

const rl = readline.createInterface({
  input: process.stdin,
  output: process.stdout,
  terminal: false,
});

rl.on('line', async (line) => {
  let command = null;
  try {
    command = JSON.parse(line);
    const data = await handleCommand(command);
    process.stdout.write(JSON.stringify({
      ok: true,
      id: command.id,
      data,
    }) + '\n');
    if (command.action === 'close') {
      process.exit(0);
    }
  } catch (error) {
    process.stdout.write(JSON.stringify({
      ok: false,
      id: command ? command.id : null,
      error: String(error && error.message ? error.message : error),
    }) + '\n');
  }
});
"#;
