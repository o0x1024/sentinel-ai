#!/usr/bin/env node
/**
 * Extract and compare i18n translation keys
 */

import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

function getAllFiles(dir, extensions = ['.vue', '.ts', '.tsx']) {
  let files = [];
  const items = fs.readdirSync(dir);
  
  for (const item of items) {
    if (['node_modules', '.next', 'dist', '.git', '__pycache__'].includes(item)) continue;
    
    const fullPath = path.join(dir, item);
    const stat = fs.statSync(fullPath);
    
    if (stat.isDirectory()) {
      files = files.concat(getAllFiles(fullPath, extensions));
    } else if (extensions.some(ext => item.endsWith(ext)) && !item.endsWith('.d.ts')) {
      files.push(fullPath);
    }
  }
  
  return files;
}

function extractKeysFromCode() {
  const srcDir = '/Users/like/code/sentinel-ai/src';
  const files = getAllFiles(srcDir);
  const keys = new Set();
  const keyUsage = new Map(); // Track which files use which keys
  
  // Regex to match t('key'), t("key"), t(`key`)
  const regex = /t\(\s*['"`]([^'"`]+)['"`]\s*\)/g;
  
  for (const file of files) {
    try {
      let content = fs.readFileSync(file, 'utf-8');
      let match;
      
      while ((match = regex.exec(content)) !== null) {
        const key = match[1];
        keys.add(key);
        
        if (!keyUsage.has(key)) {
          keyUsage.set(key, []);
        }
        const files_list = keyUsage.get(key);
        const relativePath = file.replace(srcDir, '');
        if (!files_list.includes(relativePath)) {
          files_list.push(relativePath);
        }
      }
    } catch (err) {
      console.error(`Error reading ${file}: ${err.message}`);
    }
  }
  
  return { keys, keyUsage };
}

function getAllLocaleKeys(localeFile) {
  const localeDir = path.dirname(localeFile);
  const allKeys = new Set();
  
  try {
    let content = fs.readFileSync(localeFile, 'utf-8');
    
    // Extract imported modules
    const importRegex = /import\s+(\w+)\s+from\s+['"]\.\/([^'"]+)['"]/g;
    let importMatch;
    
    while ((importMatch = importRegex.exec(content)) !== null) {
      const varName = importMatch[1];
      const importPath = importMatch[2];
      
      // Try to load the module
      let modulePath = path.join(localeDir, importPath);
      if (!modulePath.endsWith('.ts')) {
        modulePath += '.ts';
      }
      
      if (fs.existsSync(modulePath)) {
        try {
          const moduleContent = fs.readFileSync(modulePath, 'utf-8');
          const keys = getKeysFromModule(moduleContent);
          
          for (const key of keys) {
            allKeys.add(`${varName}.${key}`);
          }
        } catch (err) {
          console.error(`Error reading module ${modulePath}: ${err.message}`);
        }
      }
    }
    
    // Extract inline keys directly in the file
    const inlineKeys = getKeysFromModule(content);
    for (const key of inlineKeys) {
      allKeys.add(key);
    }
    
  } catch (err) {
    console.error(`Error reading ${localeFile}: ${err.message}`);
  }
  
  return allKeys;
}

function getKeysFromModule(content) {
  const keys = new Set();
  
  // Extract object keys from export default
  const exportRegex = /export\s+default\s+\{([\s\S]*?)\n\}/;
  const exportMatch = content.match(exportRegex);
  
  if (exportMatch) {
    const objContent = exportMatch[1];
    
    // Get all keys that appear as property names
    const keyRegex = /(\w+)\s*:/g;
    let keyMatch;
    
    while ((keyMatch = keyRegex.exec(objContent)) !== null) {
      const key = keyMatch[1];
      
      // Check if this is a nested object or a simple value
      const afterKey = objContent.substring(keyMatch.index + keyMatch[0].length).trim();
      
      if (afterKey.startsWith('{')) {
        // This is a nested object, try to extract its sub-keys
        const subObjRegex = new RegExp(`${key}\\s*:\\s*\\{([^}]*)\\}`, 's');
        const subMatch = objContent.match(subObjRegex);
        
        if (subMatch) {
          keys.add(key);
          const subContent = subMatch[1];
          const subKeyRegex = /(\w+)\s*:/g;
          let subMatch2;
          
          while ((subMatch2 = subKeyRegex.exec(subContent)) !== null) {
            keys.add(`${key}.${subMatch2[1]}`);
          }
        }
      } else {
        keys.add(key);
      }
    }
  }
  
  return keys;
}

function groupByNamespace(keys) {
  const grouped = new Map();
  
  for (const key of keys) {
    let namespace = '[root]';
    
    if (key.includes('.')) {
      namespace = key.split('.')[0];
    }
    
    if (!grouped.has(namespace)) {
      grouped.set(namespace, []);
    }
    
    grouped.get(namespace).push(key);
  }
  
  return grouped;
}

// Main execution
console.log('Extracting translation keys from code...\n');
const { keys: codeKeys, keyUsage } = extractKeysFromCode();
console.log(`Found ${codeKeys.size} unique keys in code`);

console.log('\nExtracting keys from locale files...\n');
const zhFile = '/Users/like/code/sentinel-ai/src/i18n/locales/zh.ts';
const enFile = '/Users/like/code/sentinel-ai/src/i18n/locales/en.ts';

const zhKeys = getAllLocaleKeys(zhFile);
const enKeys = getAllLocaleKeys(enFile);

console.log(`Found ${zhKeys.size} unique keys in zh.ts`);
console.log(`Found ${enKeys.size} unique keys in en.ts`);

// Find missing keys
const missingInZh = new Set([...codeKeys].filter(k => !zhKeys.has(k)));
const missingInEn = new Set([...codeKeys].filter(k => !enKeys.has(k)));

console.log('\n' + '='.repeat(80));
console.log('MISSING TRANSLATION KEYS');
console.log('='.repeat(80));

if (missingInZh.size > 0) {
  console.log(`\n[${missingInZh.size}] Missing in zh.ts:\n`);
  const sorted = [...missingInZh].sort();
  for (const key of sorted.slice(0, 100)) {
    console.log(`  • ${key}`);
  }
  if (sorted.length > 100) {
    console.log(`  ... and ${sorted.length - 100} more`);
  }
} else {
  console.log('\n✓ All keys present in zh.ts');
}

if (missingInEn.size > 0) {
  console.log(`\n[${missingInEn.size}] Missing in en.ts:\n`);
  const sorted = [...missingInEn].sort();
  for (const key of sorted.slice(0, 100)) {
    console.log(`  • ${key}`);
  }
  if (sorted.length > 100) {
    console.log(`  ... and ${sorted.length - 100} more`);
  }
} else {
  console.log('\n✓ All keys present in en.ts');
}

console.log('\n' + '='.repeat(80));
console.log('GROUPED BY NAMESPACE');
console.log('='.repeat(80));

const allMissing = new Set([...missingInZh, ...missingInEn]);
const grouped = groupByNamespace(allMissing);

for (const [namespace, keys] of [...grouped.entries()].sort()) {
  console.log(`\n### **${namespace}** (${keys.length} missing)`);
  
  const zhMissing = keys.filter(k => missingInZh.has(k));
  const enMissing = keys.filter(k => missingInEn.has(k));
  
  if (zhMissing.length > 0) {
    console.log(`\n  **Missing in zh.ts** (${zhMissing.length}):`);
    for (const key of zhMissing.sort()) {
      console.log(`    - ${key}`);
    }
  }
  
  if (enMissing.length > 0) {
    console.log(`\n  **Missing in en.ts** (${enMissing.length}):`);
    for (const key of enMissing.sort()) {
      console.log(`    - ${key}`);
    }
  }
}

console.log('\n' + '='.repeat(80));
console.log('SUMMARY');
console.log('='.repeat(80));
console.log(`
Total keys in code: ${codeKeys.size}
Total keys in zh.ts: ${zhKeys.size}
Total keys in en.ts: ${enKeys.size}

Missing in zh.ts: ${missingInZh.size}
Missing in en.ts: ${missingInEn.size}
Both missing: ${new Set([...missingInZh].filter(k => missingInEn.has(k))).size}
`);
