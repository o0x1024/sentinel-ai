#!/usr/bin/env node

import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';
import { glob } from 'glob';

const __dirname = path.dirname(fileURLToPath(import.meta.url));

// 提取所有 t('...') 调用的键
function extractI18nKeys(content) {
  const keySet = new Set();
  // 匹配 t('key') 或 t("key") 或 t("key", defaultValue)
  const regex = /t\(['"]([^'"]+)['"](?:[,\s]*['"]([^'"]+)['"])?\)/g;
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

// 生成缺失键的翻译值（中英文版本）
function generateValue(key, language) {
  // 从键生成合理的标签
  const parts = key.split('.');
  const lastPart = parts[parts.length - 1];

  if (language === 'zh') {
    // 常见的中文翻译
    const mappings = {
      'title': '标题',
      'name': '名称',
      'description': '描述',
      'action': '操作',
      'actions': '操作',
      'add': '添加',
      'edit': '编辑',
      'delete': '删除',
      'remove': '删除',
      'save': '保存',
      'cancel': '取消',
      'confirm': '确认',
      'ok': '确定',
      'close': '关闭',
      'open': '打开',
      'search': '搜索',
      'placeholder': '请输入...',
      'loading': '加载中...',
      'error': '错误',
      'success': '成功',
      'warning': '警告',
      'info': '信息',
      'status': '状态',
      'enabled': '已启用',
      'disabled': '已禁用',
      'on': '启用',
      'off': '禁用',
      'yes': '是',
      'no': '否',
      'true': '是',
      'false': '否',
      'button': '按钮',
      'buttons': '按钮',
      'label': '标签',
      'labels': '标签',
      'list': '列表',
      'details': '详情',
      'settings': '设置',
      'options': '选项',
      'select': '选择',
      'selected': '已选择',
      'count': '数量',
      'total': '总数',
      'empty': '暂无数据',
      'failed': '失败',
      'tooltip': '提示',
      'header': '标题',
      'footer': '页脚',
      'content': '内容',
      'message': '消息',
      'value': '值',
      'key': '键',
      'id': 'ID',
      'create': '创建',
      'createdAt': '创建时间',
      'updatedAt': '更新时间',
      'deleteAt': '删除时间',
      'path': '路径',
      'url': 'URL',
      'link': '链接',
      'filter': '过滤',
      'sort': '排序',
      'export': '导出',
      'import': '导入',
      'download': '下载',
      'upload': '上传',
      'refresh': '刷新',
      'submit': '提交',
      'reset': '重置'
    };

    return mappings[lastPart] || `${key}（${lastPart}）`;
  } else {
    // 英文映射
    const mappings = {
      'title': 'Title',
      'name': 'Name',
      'description': 'Description',
      'action': 'Action',
      'actions': 'Actions',
      'add': 'Add',
      'edit': 'Edit',
      'delete': 'Delete',
      'remove': 'Remove',
      'save': 'Save',
      'cancel': 'Cancel',
      'confirm': 'Confirm',
      'ok': 'OK',
      'close': 'Close',
      'open': 'Open',
      'search': 'Search',
      'placeholder': 'Please enter...',
      'loading': 'Loading...',
      'error': 'Error',
      'success': 'Success',
      'warning': 'Warning',
      'info': 'Info',
      'status': 'Status',
      'enabled': 'Enabled',
      'disabled': 'Disabled',
      'on': 'On',
      'off': 'Off',
      'yes': 'Yes',
      'no': 'No',
      'true': 'True',
      'false': 'False',
      'button': 'Button',
      'buttons': 'Buttons',
      'label': 'Label',
      'labels': 'Labels',
      'list': 'List',
      'details': 'Details',
      'settings': 'Settings',
      'options': 'Options',
      'select': 'Select',
      'selected': 'Selected',
      'count': 'Count',
      'total': 'Total',
      'empty': 'No data',
      'failed': 'Failed',
      'tooltip': 'Tooltip',
      'header': 'Header',
      'footer': 'Footer',
      'content': 'Content',
      'message': 'Message',
      'value': 'Value',
      'key': 'Key',
      'id': 'ID',
      'create': 'Create',
      'createdAt': 'Created At',
      'updatedAt': 'Updated At',
      'deletedAt': 'Deleted At',
      'path': 'Path',
      'url': 'URL',
      'link': 'Link',
      'filter': 'Filter',
      'sort': 'Sort',
      'export': 'Export',
      'import': 'Import',
      'download': 'Download',
      'upload': 'Upload',
      'refresh': 'Refresh',
      'submit': 'Submit',
      'reset': 'Reset'
    };

    return mappings[lastPart] || key;
  }
}

// 构建嵌套对象
function setNestedValue(obj, path, value) {
  const parts = path.split('.');
  let current = obj;
  for (let i = 0; i < parts.length - 1; i++) {
    if (!current[parts[i]]) {
      current[parts[i]] = {};
    }
    current = current[parts[i]];
  }
  current[parts[parts.length - 1]] = value;
}

async function main() {
  console.log('🔄 Extracting all i18n keys from code...\n');

  // 获取所有 Vue 和 TS 文件
  const files = await glob('src/**/*.{vue,ts,tsx}', { ignore: 'node_modules/**' });
  
  const allCodeKeys = new Set();
  for (const file of files) {
    const content = fs.readFileSync(file, 'utf-8');
    const keys = extractI18nKeys(content);
    keys.forEach(k => allCodeKeys.add(k));
  }

  console.log(`✓ Found ${allCodeKeys.size} unique translation keys in code\n`);

  // 加载现有的翻译文件
  const zhPath = 'src/i18n/locales/zh.ts';
  const enPath = 'src/i18n/locales/en.ts';

  // 动态导入
  const zhModule = await import(path.resolve(zhPath), { assert: { type: 'module' } });
  const enModule = await import(path.resolve(enPath), { assert: { type: 'module' } });

  const zhLocale = zhModule.default || zhModule;
  const enLocale = enModule.default || enModule;

  const zhKeys = getAllKeyPaths(zhLocale);
  const enKeys = getAllKeyPaths(enLocale);

  const missingInZh = new Set([...allCodeKeys].filter(k => !zhKeys.has(k)));
  const missingInEn = new Set([...allCodeKeys].filter(k => !enKeys.has(k)));

  console.log(`⚠️  Missing in zh.ts: ${missingInZh.size}`);
  console.log(`⚠️  Missing in en.ts: ${missingInEn.size}\n`);

  // 按命名空间分组
  const groupByNamespace = (keys) => {
    const groups = {};
    for (const key of keys) {
      const namespace = key.split('.')[0];
      if (!groups[namespace]) groups[namespace] = [];
      groups[namespace].push(key);
    }
    return groups;
  };

  const missingZhByNs = groupByNamespace(missingInZh);
  const missingEnByNs = groupByNamespace(missingInEn);

  console.log('📊 Missing by namespace (Top 5):');
  const sortedNs = Object.entries(missingZhByNs)
    .sort((a, b) => b[1].length - a[1].length)
    .slice(0, 5);
  
  for (const [ns, keys] of sortedNs) {
    console.log(`  ${ns}: ${keys.length} keys`);
  }

  // 生成修复脚本（仅作为参考，不自动修改）
  console.log('\n✅ Analysis complete!');
  console.log('\nTo auto-fill missing translations, run:');
  console.log('  node complete_i18n.mjs\n');
  console.log('Note: This will add placeholder translations for all missing keys.');
  console.log('Please review and update translations manually for accuracy.\n');
}

main().catch(err => {
  console.error('Error:', err.message);
  process.exit(1);
});
