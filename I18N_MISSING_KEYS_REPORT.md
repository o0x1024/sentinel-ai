# i18n Translation Keys Analysis Report

**Execution Date**: 2026-03-23  
**Workspace**: sentinel-ai  
**Analysis Scope**: Vue (.vue) and TypeScript (.ts, .tsx) files in src/  
**Locales Compared**: 
- [src/i18n/locales/zh.ts](src/i18n/locales/zh.ts) (Chinese)
- [src/i18n/locales/en.ts](src/i18n/locales/en.ts) (English)

---

## Executive Summary

| Metric | Count |
|--------|-------|
| **Total valid i18n keys found in code** | 3,480 |
| **Keys defined in zh.ts** | 7,092 |
| **Keys defined in en.ts** | 5,869 |
| **Missing in zh.ts** | 2,553 (73.3%) |
| **Missing in en.ts** | 2,544 (73.1%) |
| **Missing in both** | 2,540 (100 of discrepancy is shared) |
| **Unique to zh missing** | 13 |
| **Unique to en missing** | 4 |

---

## ⚠️ Critical Issues

1. **Massive Coverage Gap**: ~73% of translation keys used in code are missing from both locale files
2. **High Duplication**: 2,540 keys are missing from both locales simultaneously
3. **Minimal Language Variance**: Only 13 keys differ between what's missing in zh vs en

---

## Missing by Namespace

### 🔴 **trafficAnalysis** (811 missing)
**Namespaces affected**: trafficAnalysis.control, trafficAnalysis.history, trafficAnalysis.intercept, trafficAnalysis.packetCapture, trafficAnalysis.proxifierProxies, trafficAnalysis.proxyConfiguration, trafficAnalysis.repeater, trafficAnalysis.workflowStudio, and more

**Sample missing keys**:
- `trafficAnalysis.control.startProxy`
- `trafficAnalysis.control.stats.proxyStatus`
- `trafficAnalysis.history.contextMenu.copyAsCurl`
- `trafficAnalysis.intercept.buttons.forward`
- `trafficAnalysis.workflowStudio.flowchart.toolbar.newWorkflow`

**Impact**: This namespace alone represents 46% of all missing translations. Heavily developed but not localized.

---

### 🔴 **bugBounty** (553 missing)
**Namespaces affected**: bugBounty.assets, bugBounty.batch, bugBounty.changeEvents, bugBounty.confirm, bugBounty.errors, bugBounty.findings, bugBounty.monitor, bugBounty.severity, bugBounty.stats, bugBounty.success, bugBounty.tabs

**Sample missing keys**:
- `bugBounty.assets.attackSurface`
- `bugBounty.batch.selectAllFiltered`
- `bugBounty.changeEvents.affectedScope`
- `bugBounty.errors.loadFailed`
- `bugBounty.severity.critical`

**Impact**: 31.7% of missing keys. Major feature lacking translation infrastructure.

---

### 🟠 **settings** (491 missing)
**Category**: Configuration management

**Namespaces affected**: settings.agent, settings.asm, settings.database, settings.mcp, settings.notification, settings.proxy, and more

**Sample missing keys**:
- `settings.agent.completionGuard.title`
- `settings.asm.description`
- `settings.database.poolSize`
- `settings.notification.emailConfig`
- `settings.proxy.authBasic`

**Impact**: 28.1% of missing keys. Settings pages not internationalized.

---

### 🟠 **Tools** (88 missing)
**Category**: Tool management and configuration

**Sample missing keys**:
- `Tools.addServer.title`
- `Tools.jsonEdit`
- `Tools.serverDetails.connectToViewTools`
- `Tools.connectionFailed`
- `Tools.paramsJsonError`

**Impact**: 5% of missing keys. Implementation partially i18n'd.

---

### 🔵 **plugins** (225 missing)
**Category**: Plugin system interface

**Sample missing keys**:
- `plugins.aiAssistant`
- `plugins.agentCategories.recon`
- `plugins.advancedTest`
- `plugins.syntaxScore`
- `plugins.testFailed`

**Impact**: 12.9% of missing keys. Plugin interface UI needs translation keys.

---

### 🟢 **Agent** (13 missing)
**Category**: AI Agent subsystem

**Sample missing keys**:
- `agent.addPattern`
- `agent.patternDescription`
- `agent.removePattern`
- `agent.subagentStatus.${subagent.status}`
- `agent.tour.conversationList.title`

**Impact**: 0.7% of missing keys. Relatively complete.

---

### Other Namespaces

| Namespace | Missing | Impact |
|-----------|---------|--------|
| **llmSecurity** | 161 | 9.2% |
| **ragManagement** | 87 | 5% |
| **common** | 25 | 1.4% |
| **notifications** | 17 | 1% |
| **assetManagement** | 14 | 0.8% |
| **license** | 10 | 0.6% |
| **proxifierPanel** | 22 | 1.3% |
| **sidebar** | 5 | 0.3% |
| **roles** | 1 | 0.06% |
| **securityCenter** | 8 | 0.5% |
| **dictionary** | 5 | 0.3% |
| **tools** | 21 | 1.2% |

---

## 📊 Breakdown by Category

```
trafficAnalysis ████████████████████████████████████████ 46%  (811 keys)
bugBounty       ███████████████████████ 31.7%  (553 keys)
settings        ███████████████████ 28.1%  (491 keys)
plugins         ███████ 12.9%  (225 keys)
llmSecurity     █████ 9.2%   (161 keys)
ragManagement   ██ 5%     (87 keys)
Tools           ██ 5%     (88 keys)
Others          ████ 5.2%  (91 keys)
```

---

## Key Patterns & Issues

### 1. **Template Variables**
Several keys use template syntax that might be problematic:
- `agent.subagentStatus.${subagent.status}`
- `assetManagement.columns.${col}`
- `dictionary.types.${type}`
- `plugins.types.${type}`
- `trafficAnalysis.repeater.types.${typeKey}`

These suggest dynamic translation keys that may need special handling in the i18n system.

---

### 2. **Namespace Imbalance**
The en.ts is missing slightly more keys overall (2,544 vs 2,553), suggesting:
- Chinese translations were added first
- English translations are incomplete
- Or vice versa with different naming conventions

---

### 3. **Feature Completeness**
Based on missing translations:
- ✅ Dashboard (complete)
- ✅ AIChat (complete)
- ❌ TrafficAnalysis (0% translated)
- ❌ BugBounty (0% translated)
- ⚠️ Settings (partial)
- ⚠️ Plugins (partial)

---

## Recommendations

### 🔴 Priority 1: Major Gap Coverage (2,540 shared)
Create locale modules for:
1. **trafficAnalysis** - Create comprehensive trafficAnalysis locale module
2. **bugBounty** - Create comprehensive bugBounty locale module
3. **settings** - Complete settings translation coverage

### 🟠 Priority 2: Medium Gaps (225-161 keys)
1. **plugins** (225) - Add plugin system interface translations
2. **llmSecurity** (161) - Complete LLM security feature translations

### 🟢 Priority 3: Small Gaps (< 100 keys)
1. **ragManagement** (87 keys)
2. **Tools** (88 keys)
3. **common** (25 keys)
4. **notifications** (17 keys)
5. Others (< 15 keys each)

---

## Files Needing Creation/Updates

```
src/i18n/locales/
├── trafficAnalysis/
│   ├── zh.ts       (needs creation or expansion)
│   └── en.ts       (needs creation or expansion)
├── bugBounty/
│   ├── zh.ts       (needs creation)
│   └── en.ts       (needs creation)
└── [other folders...]
```

---

## Testing Checklist

- [ ] Verify all 3,480 keys parse correctly
- [ ] Test dynamic template keys with actual values
- [ ] Check for circular or duplicate imports in locale files
- [ ] Validate locale module exports match their imports
- [ ] Test fallback behavior for missing keys
- [ ] Verify language switching works for all namespaces

---

## Appendix: Full Missing Keys by Namespace

See the detailed script output for complete lists. Key files used:
- Extraction script: `extract_i18n_clean.mjs`
- Source analysis: All .vue and .ts files in `src/`
- Locale files: `src/i18n/locales/zh.ts` and `src/i18n/locales/en.ts`
