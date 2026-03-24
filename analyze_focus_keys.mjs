#!/usr/bin/env node

import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';
import { glob } from 'glob';

const __dirname = path.dirname(fileURLToPath(import.meta.url));

// 提取所有 t('...') 调用的键
function extractI18nKeys(content) {
  const keySet = new Set();
  const regex = /t\(['"]([^'"]+)['"]\)/g;
  let match;
  while ((match = regex.exec(content)) !== null) {
    keySet.add(match[1]);
  }
  return keySet;
}

// 从嵌套对象中递归获取所有键路径
function getAllKeyPaths(obj, prefix = '') {
  const keys = new Set();
  for (const [key, value] of Object.entries(obj)) {
    const path = prefix ? `${prefix}.${key}` : key;
    keys.add(path);
    if (typeof value === 'object' && value !== null && !Array.isArray(value)) {
      const nestedKeys = getAllKeyPaths(value, path);
      nestedKeys.forEach(k => keys.add(k));
    }
  }
  return keys;
}

async function main() {
  // 获取所有 Vue 和 TS 文件
  const files = await glob('src/**/*.{vue,ts,tsx}', { ignore: 'node_modules/**' });
  
  const allCodeKeys = new Set();
  for (const file of files) {
    const content = fs.readFileSync(file, 'utf-8');
    const keys = extractI18nKeys(content);
    keys.forEach(k => allCodeKeys.add(k));
  }

  // 动态导入现有的翻译文件
  const zhModule = await import(path.resolve('src/i18n/locales/zh.ts'), { assert: { type: 'module' } });
  const enModule = await import(path.resolve('src/i18n/locales/en.ts'), { assert: { type: 'module' } });

  const zhLocale = zhModule.default || zhModule;
  const enLocale = enModule.default || enModule;

  const zhKeys = getAllKeyPaths(zhLocale);
  const enKeys = getAllKeyPaths(enLocale);

  // 只关注 trafficAnalysis 和 bugBounty
  const focusNamespaces = ['trafficAnalysis', 'bugBounty'];

  const missingByNamespace = {};
  for (const ns of focusNamespaces) {
    const nsKeys = [...allCodeKeys].filter(k => k.startsWith(ns + '.'));
    const missing = nsKeys.filter(k => !zhKeys.has(k) && !enKeys.has(k));
    if (missing.length > 0) {
      missingByNamespace[ns] = missing.sort();
    }
  }

  console.log('📊 Missing keys in focus namespaces:\n');
  for (const [ns, keys] of Object.entries(missingByNamespace)) {
    console.log(`${ns}: ${keys.length} missing`);
    keys.slice(0, 10).forEach(k => console.log(`  - ${k}`));
    if (keys.length > 10) {
      console.log(`  ... and ${keys.length - 10} more`);
    }
  }

  console.log(`\n✅ Analy complete!`);
}

main().catch(err => {
  console.error('Error:', err.message);
  process.exit(1);
});
