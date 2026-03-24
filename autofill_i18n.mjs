#!/usr/bin/env node

/**
 * Auto-fill missing i18n translation keys
 * 
 * This script scans code for all t('key') calls and adds missing keys to locale files.
 * Missing keys are added with placeholder values (English key name or simple fallback).
 */

import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';
import { glob } from 'glob';

const __dirname = path.dirname(fileURLToPath(import.meta.url));

// Extract all t('...') calls
function extractI18nKeys(content) {
  const keySet = new Set();
  const regex = /t\(['"]([^'"]+)['"]\)/g;
  let match;
  while ((match = regex.exec(content)) !== null) {
    keySet.add(match[1]);
  }
  return keySet;
}

// Get all key paths from nested object
function getAllKeyPaths(obj, prefix = '') {
  const keys = new Set();
  const traverse = (o, p) => {
    for (const [k, v] of Object.entries(o)) {
      const fullPath = p ? `${p}.${k}` : k;
      keys.add(fullPath);
      if (typeof v === 'object' && v !== null && !Array.isArray(v)) {
        traverse(v, fullPath);
      }
    }
  };
  traverse(obj, prefix);
  return keys;
}

// Set nested value in object
function setNestedValue(obj, path, value) {
  const parts = path.split('.');
  let current = obj;
  for (let i = 0; i < parts.length - 1; i++) {
    current[parts[i]] ||= {};
    current = current[parts[i]];
  }
  current[parts[parts.length - 1]] = value;
}

// Generate a reasonable fallback value
function generatePlaceholder(key) {
  const lastPart = key.split('.').pop();
  // Return English key name as fallback
  return lastPart.replace(/([A-Z])/g, ' $1').trim();
}

async function main() {
  console.log('🔍 Extracting all i18n keys from code...\n');

  // Gather all keys from code
  const files = await glob('src/**/*.{vue,ts,tsx}', { ignore: 'node_modules/**' });
  const allCodeKeys = new Set();
  
  for (const file of files) {
    const content = fs.readFileSync(file, 'utf-8');
    const keys = extractI18nKeys(content);
    keys.forEach(k => allCodeKeys.add(k));
  }

  console.log(`✓ Found ${allCodeKeys.size} keys in code\n`);

  // Load  existing locales
  const zhModule = await import(path.resolve('src/i18n/locales/zh.ts'), { assert: { type: 'module' } });
  const enModule = await import(path.resolve('src/i18n/locales/en.ts'), { assert: { type: 'module' } });

  const zhLocale = zhModule.default || zhModule;
  const enLocale = enModule.default || enModule;

  const zhKeys = getAllKeyPaths(zhLocale);
  const enKeys = getAllKeyPaths(enLocale);

  // Find missing keys
  const missingInZh = [...allCodeKeys].filter(k => !zhKeys.has(k)).sort();
  const missingInEn = [...allCodeKeys].filter(k => !enKeys.has(k)).sort();

  console.log(`📊 Summary:`);
  console.log(`  Missing in zh.ts: ${missingInZh.length}`);
  console.log(`  Missing in en.ts: ${missingInEn.length}\n`);

  if (missingInZh.length === 0 && missingInEn.length === 0) {
    console.log('✅ All keys are already defined!');
    return;
  }

  // Add missing keys to locale objects
  const zhCopy = JSON.parse(JSON.stringify(zhLocale));
  const enCopy = JSON.parse(JSON.stringify(enLocale));

  console.log('🔧 Adding missing keys...\n');

  for (const key of missingInZh) {
    setNestedValue(zhCopy, key, `[${key}]`);
  }

  for (const key of missingInEn) {
    setNestedValue(enCopy, key, generatePlaceholder(key));
  }

  // Generate new file content
  const zhContent = `export default ${JSON.stringify(zhCopy, null, 2)}\n`;
  const enContent = `export default ${JSON.stringify(enCopy, null, 2)}\n`;

  // Backup existing files
  const timestamp = new Date().toISOString().slice(0, 10);
  const zhBackup = `src/i18n/locales/zh.ts.backup.${timestamp}`;
  const enBackup = `src/i18n/locales/en.ts.backup.${timestamp}`;

  fs.copyFileSync('src/i18n/locales/zh.ts', zhBackup);
  fs.copyFileSync('src/i18n/locales/en.ts', enBackup);
  console.log(`✓ Backed up to ${zhBackup} and ${enBackup}\n`);

  // Write new files
  fs.writeFileSync('src/i18n/locales/zh.ts', zhContent);
  fs.writeFileSync('src/i18n/locales/en.ts', enContent);

  console.log(`✅ Updated files:`);
  console.log(`  src/i18n/locales/zh.ts (+${missingInZh.length} keys)`);
  console.log(`  src/i18n/locales/en.ts (+${missingInEn.length} keys)\n`);

  console.log(`⚠️  Next steps:`);
  console.log(`  1. Review the changes and update placeholder values`);
  console.log(`  2. Test the app with translations`);
  console.log(`  3. If needed, restore backups from: ${zhBackup} and ${enBackup}\n`);
}

main().catch(err => {
  console.error('❌ Error:', err.message);
  process.exit(1);
});
