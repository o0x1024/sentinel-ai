//! 浏览器注入脚本
//!
//! 用于注入到页面的 JavaScript 脚本，实现路由监听、DOM 变化检测等功能

// ============================================================================
// 元素快照系统 - 解决 Index 漂移问题
// ============================================================================

/// 创建元素快照脚本
/// 调用 playwright_annotate 后执行，将元素引用锁定到带唯一 ID 的快照中
/// 返回 snapshot_id，后续操作必须携带此 ID
pub const CREATE_ELEMENT_SNAPSHOT_SCRIPT: &str = r#"
(function() {
    // 生成唯一的快照 ID (时间戳 + 随机数)
    const snapshotId = Date.now().toString(36) + Math.random().toString(36).substring(2, 8);
    
    // 获取当前标注的元素
    const source = window.__elements_map__ || window.__playwrightAnnotatedElements;
    if (!source) {
        return { success: false, error: 'No annotated elements found', snapshotId: null };
    }
    
    const elements = Array.isArray(source) ? source : Object.values(source);
    
    // 为每个元素生成稳定的指纹 (用于漂移检测)
    const snapshot = {
        id: snapshotId,
        createdAt: Date.now(),
        elementCount: elements.length,
        elements: {}
    };
    
    elements.forEach((el, idx) => {
        if (!el) return;
        
        // 计算元素指纹 (用于后续验证)
        const rect = el.getBoundingClientRect ? el.getBoundingClientRect() : {};
        const fingerprint = {
            tagName: el.tagName || '',
            text: (el.textContent || '').substring(0, 50).trim(),
            id: el.id || '',
            className: (typeof el.className === 'string' ? el.className : '').substring(0, 100),
            href: el.getAttribute ? (el.getAttribute('href') || '') : '',
            type: el.getAttribute ? (el.getAttribute('type') || '') : '',
            name: el.getAttribute ? (el.getAttribute('name') || '') : '',
            // 位置粗略哈希 (允许小幅偏移)
            posHash: Math.floor(rect.x / 10) + '_' + Math.floor(rect.y / 10) + '_' + 
                     Math.floor(rect.width / 10) + '_' + Math.floor(rect.height / 10),
            // 精确坐标 (用于回退)
            centerX: rect.x + rect.width / 2,
            centerY: rect.y + rect.height / 2
        };
        
        snapshot.elements[idx] = {
            ref: el,              // 保存 DOM 引用
            fingerprint: fingerprint,
            index: idx
        };
    });
    
    // 存储快照
    window.__element_snapshot__ = snapshot;
    
    return {
        success: true,
        snapshotId: snapshotId,
        elementCount: elements.length,
        createdAt: snapshot.createdAt
    };
})()
"#;

/// 验证快照有效性脚本生成器
/// 检查指定的 snapshot_id 是否仍然有效，以及目标 index 的元素是否已漂移
pub fn validate_snapshot_script(snapshot_id: &str, index: u32) -> String {
    format!(
        r#"
(function() {{
    const targetSnapshotId = "{}";
    const targetIndex = {};
    
    const snapshot = window.__element_snapshot__;
    
    // 检查快照是否存在
    if (!snapshot) {{
        return {{ 
            valid: false, 
            error: 'NO_SNAPSHOT',
            message: 'No element snapshot exists, page may have been reloaded'
        }};
    }}
    
    // 检查快照 ID 是否匹配
    if (snapshot.id !== targetSnapshotId) {{
        return {{ 
            valid: false, 
            error: 'SNAPSHOT_MISMATCH',
            message: 'Snapshot ID mismatch: expected ' + targetSnapshotId + ', got ' + snapshot.id,
            currentSnapshotId: snapshot.id
        }};
    }}
    
    // 检查快照是否过期 (超过 30 秒视为过期)
    const age = Date.now() - snapshot.createdAt;
    if (age > 30000) {{
        return {{ 
            valid: false, 
            error: 'SNAPSHOT_EXPIRED',
            message: 'Snapshot expired after ' + Math.round(age / 1000) + ' seconds',
            age: age
        }};
    }}
    
    // 检查目标索引是否存在
    const entry = snapshot.elements[targetIndex];
    if (!entry) {{
        return {{ 
            valid: false, 
            error: 'INDEX_NOT_FOUND',
            message: 'Element index ' + targetIndex + ' not found in snapshot (max: ' + (snapshot.elementCount - 1) + ')',
            maxIndex: snapshot.elementCount - 1
        }};
    }}
    
    // 检查元素是否仍然连接到 DOM
    const el = entry.ref;
    if (!el || !el.isConnected) {{
        return {{ 
            valid: false, 
            error: 'ELEMENT_DETACHED',
            message: 'Element at index ' + targetIndex + ' is no longer in DOM',
            fingerprint: entry.fingerprint
        }};
    }}
    
    // 检查元素是否仍然可见
    const rect = el.getBoundingClientRect();
    if (rect.width === 0 || rect.height === 0) {{
        return {{ 
            valid: false, 
            error: 'ELEMENT_HIDDEN',
            message: 'Element at index ' + targetIndex + ' is now hidden (size 0)',
            fingerprint: entry.fingerprint
        }};
    }}
    
    // 计算当前指纹
    const currentFingerprint = {{
        tagName: el.tagName || '',
        text: (el.textContent || '').substring(0, 50).trim(),
        id: el.id || '',
        className: (typeof el.className === 'string' ? el.className : '').substring(0, 100),
        posHash: Math.floor(rect.x / 10) + '_' + Math.floor(rect.y / 10) + '_' + 
                 Math.floor(rect.width / 10) + '_' + Math.floor(rect.height / 10)
    }};
    
    // 比较核心指纹 (允许位置小幅偏移)
    const orig = entry.fingerprint;
    const fingerprintMatch = (
        currentFingerprint.tagName === orig.tagName &&
        currentFingerprint.id === orig.id
        // 不强制比较 text 和 className，因为这些可能因 hover/active 状态变化
    );
    
    if (!fingerprintMatch) {{
        return {{ 
            valid: false, 
            error: 'ELEMENT_CHANGED',
            message: 'Element at index ' + targetIndex + ' has changed significantly',
            originalFingerprint: orig,
            currentFingerprint: currentFingerprint
        }};
    }}
    
    // 一切正常
    return {{ 
        valid: true, 
        snapshotId: snapshot.id,
        index: targetIndex,
        element: {{
            tagName: el.tagName,
            text: (el.textContent || '').substring(0, 30).trim(),
            centerX: rect.x + rect.width / 2,
            centerY: rect.y + rect.height / 2
        }}
    }};
}})()
"#,
        snapshot_id, index
    )
}

/// 通过快照执行点击操作脚本生成器
/// 使用 snapshot_id + index 定位元素，验证后执行点击
pub fn click_by_snapshot_script(snapshot_id: &str, index: u32) -> String {
    format!(
        r#"
(async function() {{
    const targetSnapshotId = "{}";
    const targetIndex = {};
    
    const snapshot = window.__element_snapshot__;
    
    // 验证快照
    if (!snapshot || snapshot.id !== targetSnapshotId) {{
        return {{ 
            success: false, 
            error: 'SNAPSHOT_INVALID',
            message: snapshot ? 'Snapshot ID mismatch' : 'No snapshot exists'
        }};
    }}
    
    const entry = snapshot.elements[targetIndex];
    if (!entry || !entry.ref || !entry.ref.isConnected) {{
        return {{ 
            success: false, 
            error: 'ELEMENT_INVALID',
            message: 'Element not found or disconnected',
            fallbackCoords: entry ? {{ x: entry.fingerprint.centerX, y: entry.fingerprint.centerY }} : null
        }};
    }}
    
    const el = entry.ref;
    const rect = el.getBoundingClientRect();
    
    // 滚动到元素可见
    if (el.scrollIntoViewIfNeeded) {{
        el.scrollIntoViewIfNeeded(true);
    }} else {{
        el.scrollIntoView({{ block: 'center', behavior: 'instant' }});
    }}
    
    // 短暂等待滚动完成
    await new Promise(r => setTimeout(r, 100));
    
    // 尝试点击
    try {{
        // 先聚焦
        if (el.focus) el.focus();
        
        // 使用原生 click()
        el.click();
        
        return {{ 
            success: true, 
            method: 'click',
            element: {{
                tagName: el.tagName,
                text: (el.textContent || '').substring(0, 30).trim()
            }}
        }};
    }} catch (e) {{
        // 回退到坐标点击
        return {{ 
            success: false, 
            error: 'CLICK_FAILED',
            message: e.message,
            fallbackCoords: {{ 
                x: Math.round(rect.x + rect.width / 2), 
                y: Math.round(rect.y + rect.height / 2) 
            }}
        }};
    }}
}})()
"#,
        snapshot_id, index
    )
}

/// 通过快照执行填充操作脚本生成器
pub fn fill_by_snapshot_script(snapshot_id: &str, index: u32, value: &str) -> String {
    // 对 value 进行转义处理
    let escaped_value = value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r");

    format!(
        r#"
(async function() {{
    const targetSnapshotId = "{}";
    const targetIndex = {};
    const fillValue = "{}";
    
    const snapshot = window.__element_snapshot__;
    
    // 验证快照
    if (!snapshot || snapshot.id !== targetSnapshotId) {{
        return {{ 
            success: false, 
            error: 'SNAPSHOT_INVALID',
            message: snapshot ? 'Snapshot ID mismatch' : 'No snapshot exists'
        }};
    }}
    
    const entry = snapshot.elements[targetIndex];
    if (!entry || !entry.ref || !entry.ref.isConnected) {{
        return {{ 
            success: false, 
            error: 'ELEMENT_INVALID',
            message: 'Element not found or disconnected'
        }};
    }}
    
    const el = entry.ref;
    
    // 确保是可填充的元素
    const fillable = ['INPUT', 'TEXTAREA', 'SELECT'];
    if (!fillable.includes(el.tagName)) {{
        return {{ 
            success: false, 
            error: 'NOT_FILLABLE',
            message: 'Element ' + el.tagName + ' is not a fillable element'
        }};
    }}
    
    // 滚动到元素可见
    if (el.scrollIntoViewIfNeeded) {{
        el.scrollIntoViewIfNeeded(true);
    }} else {{
        el.scrollIntoView({{ block: 'center', behavior: 'instant' }});
    }}
    
    await new Promise(r => setTimeout(r, 100));
    
    try {{
        // 聚焦元素
        el.focus();
        
        // 清空并填充
        if (el.tagName === 'SELECT') {{
            // 对于 select，设置 value
            el.value = fillValue;
            el.dispatchEvent(new Event('change', {{ bubbles: true }}));
        }} else {{
            // 对于 input/textarea
            el.value = fillValue;
            // 触发必要的事件
            el.dispatchEvent(new Event('input', {{ bubbles: true }}));
            el.dispatchEvent(new Event('change', {{ bubbles: true }}));
        }}
        
        return {{ 
            success: true, 
            element: {{
                tagName: el.tagName,
                name: el.name || el.id || ''
            }},
            valueSet: fillValue.substring(0, 20) + (fillValue.length > 20 ? '...' : '')
        }};
    }} catch (e) {{
        return {{ 
            success: false, 
            error: 'FILL_FAILED',
            message: e.message
        }};
    }}
}})()
"#,
        snapshot_id, index, escaped_value
    )
}

/// 清除当前快照脚本
pub const CLEAR_SNAPSHOT_SCRIPT: &str = r#"
(function() {
    const hadSnapshot = !!window.__element_snapshot__;
    window.__element_snapshot__ = null;
    return { cleared: hadSnapshot };
})()
"#;

/// 获取当前快照信息脚本
pub const GET_SNAPSHOT_INFO_SCRIPT: &str = r#"
(function() {
    const snapshot = window.__element_snapshot__;
    if (!snapshot) {
        return { exists: false };
    }
    return {
        exists: true,
        id: snapshot.id,
        elementCount: snapshot.elementCount,
        createdAt: snapshot.createdAt,
        age: Date.now() - snapshot.createdAt
    };
})()
"#;

// ============================================================================
// 原有脚本
// ============================================================================

/// 路由变化监听脚本
/// 拦截 History API 和 hashchange 事件，收集所有路由变化
pub const ROUTE_MONITOR_SCRIPT: &str = r#"
(function() {
    // 初始化路由集合
    window.__VISION_ROUTES__ = window.__VISION_ROUTES__ || new Set();
    window.__VISION_ROUTES__.add(location.href);
    
    // 保存原始方法
    const originalPushState = history.pushState;
    const originalReplaceState = history.replaceState;
    
    // 拦截 pushState
    history.pushState = function(...args) {
        originalPushState.apply(this, args);
        window.__VISION_ROUTES__.add(location.href);
        window.dispatchEvent(new CustomEvent('__vision_route_change__', {
            detail: { url: location.href, type: 'pushState' }
        }));
    };
    
    // 拦截 replaceState
    history.replaceState = function(...args) {
        originalReplaceState.apply(this, args);
        window.__VISION_ROUTES__.add(location.href);
        window.dispatchEvent(new CustomEvent('__vision_route_change__', {
            detail: { url: location.href, type: 'replaceState' }
        }));
    };
    
    // 监听 popstate（浏览器前进/后退）
    window.addEventListener('popstate', () => {
        window.__VISION_ROUTES__.add(location.href);
    });
    
    // 监听 hashchange
    window.addEventListener('hashchange', () => {
        window.__VISION_ROUTES__.add(location.href);
    });
    
    console.log('[VisionExplorer] Route monitor injected');
})();
"#;

/// 获取发现的路由脚本
pub const GET_ROUTES_SCRIPT: &str = r#"
Array.from(window.__VISION_ROUTES__ || [])
"#;

/// DOM 变化监听脚本
/// 使用 MutationObserver 监听 DOM 变化
pub const DOM_MUTATION_SCRIPT: &str = r#"
(function() {
    // 初始化变化记录
    window.__VISION_DOM_CHANGES__ = [];
    window.__VISION_NEW_ELEMENTS__ = [];
    
    const observer = new MutationObserver((mutations) => {
        mutations.forEach(mutation => {
            if (mutation.type === 'childList') {
                mutation.addedNodes.forEach(node => {
                    if (node.nodeType === 1) {  // Element node
                        const el = node;
                        
                        // 记录变化
                        window.__VISION_DOM_CHANGES__.push({
                            type: 'added',
                            tag: el.tagName,
                            id: el.id || null,
                            className: el.className || null,
                            role: el.getAttribute('role'),
                            ariaModal: el.getAttribute('aria-modal'),
                            timestamp: Date.now()
                        });
                        
                        // 检测是否是模态框/弹窗
                        const role = el.getAttribute('role');
                        const isModal = el.getAttribute('aria-modal') === 'true';
                        const className = (el.className || '').toLowerCase();
                        
                        if (role === 'dialog' || isModal || 
                            className.includes('modal') || 
                            className.includes('popup') ||
                            className.includes('drawer') ||
                            className.includes('overlay')) {
                            window.__VISION_NEW_ELEMENTS__.push({
                                type: 'modal',
                                selector: el.id ? '#' + el.id : el.tagName.toLowerCase() + '.' + el.className.split(' ')[0],
                                visible: el.offsetParent !== null
                            });
                        }
                        
                        // 检测下拉菜单
                        if (role === 'menu' || role === 'listbox' ||
                            className.includes('dropdown') ||
                            className.includes('menu')) {
                            window.__VISION_NEW_ELEMENTS__.push({
                                type: 'dropdown',
                                selector: el.id ? '#' + el.id : el.tagName.toLowerCase(),
                                visible: el.offsetParent !== null
                            });
                        }
                    }
                });
            }
        });
    });
    
    observer.observe(document.body, {
        childList: true,
        subtree: true
    });
    
    console.log('[VisionExplorer] DOM mutation observer injected');
})();
"#;

/// 获取 DOM 变化脚本
pub const GET_DOM_CHANGES_SCRIPT: &str = r#"
(function() {
    const changes = window.__VISION_DOM_CHANGES__ || [];
    const newElements = window.__VISION_NEW_ELEMENTS__ || [];
    
    // 清空收集的数据
    window.__VISION_DOM_CHANGES__ = [];
    window.__VISION_NEW_ELEMENTS__ = [];
    
    return {
        dom_changes: changes,
        new_elements: newElements
    };
})()
"#;

/// 检测动态组件脚本（模态框、弹窗等）
pub const DETECT_DYNAMIC_COMPONENTS_SCRIPT: &str = r#"
(function() {
    const components = [];
    
    // 检测模态框
    document.querySelectorAll('[role="dialog"], [aria-modal="true"], .modal, .popup, .drawer, .overlay').forEach(el => {
        if (el.offsetParent !== null) {  // 可见
            components.push({
                type: 'modal',
                selector: el.id ? '#' + el.id : generateSelector(el),
                visible: true,
                tag: el.tagName
            });
        }
    });
    
    // 检测下拉菜单
    document.querySelectorAll('[role="menu"], [role="listbox"], .dropdown-menu, .submenu').forEach(el => {
        components.push({
            type: 'dropdown',
            selector: el.id ? '#' + el.id : generateSelector(el),
            visible: el.offsetParent !== null,
            tag: el.tagName
        });
    });
    
    // 检测 tooltip
    document.querySelectorAll('[role="tooltip"], .tooltip').forEach(el => {
        if (el.offsetParent !== null) {
            components.push({
                type: 'tooltip',
                selector: generateSelector(el),
                visible: true,
                tag: el.tagName
            });
        }
    });
    
    function generateSelector(el) {
        if (el.id) return '#' + el.id;
        let selector = el.tagName.toLowerCase();
        if (el.className) {
            const firstClass = el.className.split(' ')[0];
            if (firstClass) selector += '.' + firstClass;
        }
        return selector;
    }
    
    return components;
})()
"#;

/// 提取页面内部链接脚本
pub const EXTRACT_INTERNAL_LINKS_SCRIPT: &str = r#"
(function() {
    const links = [];
    const origin = location.origin;
    
    document.querySelectorAll('a[href]').forEach(a => {
        let href = a.getAttribute('href');
        if (!href) return;
        
        // 跳过锚点、javascript、mailto 等
        if (href.startsWith('#') || 
            href.startsWith('javascript:') || 
            href.startsWith('mailto:') ||
            href.startsWith('tel:')) {
            return;
        }
        
        // 转换为绝对 URL
        try {
            const url = new URL(href, origin);
            // 只保留同源链接
            if (url.origin === origin) {
                links.push(url.href);
            }
        } catch (e) {
            // 忽略无效 URL
        }
    });
    
    // 去重
    return [...new Set(links)];
})()
"#;

/// 悬停元素并检测变化脚本生成器
pub fn hover_element_script(index: u32) -> String {
    format!(
        r#"
(async function() {{
    // 获取标注元素
    const elements = window.__elements_map__ || {{}};
    const element = elements[{}];
    
    if (!element) {{
        return {{ success: false, error: 'Element not found', new_elements: [] }};
    }}
    
    // 记录悬停前的可见元素计数
    const beforeCount = document.querySelectorAll('[role="menu"], [role="listbox"], .dropdown-menu, .submenu, .tooltip').length;
    
    // 触发悬停
    const rect = element.getBoundingClientRect();
    const centerX = rect.left + rect.width / 2;
    const centerY = rect.top + rect.height / 2;
    
    // 发送 mouseover 和 mouseenter 事件
    const mouseoverEvent = new MouseEvent('mouseover', {{
        bubbles: true,
        cancelable: true,
        view: window,
        clientX: centerX,
        clientY: centerY
    }});
    
    const mouseenterEvent = new MouseEvent('mouseenter', {{
        bubbles: false,
        cancelable: true,
        view: window,
        clientX: centerX,
        clientY: centerY
    }});
    
    element.dispatchEvent(mouseenterEvent);
    element.dispatchEvent(mouseoverEvent);
    
    // 等待动画
    await new Promise(resolve => setTimeout(resolve, 300));
    
    // 检测新出现的元素
    const afterElements = document.querySelectorAll('[role="menu"], [role="listbox"], .dropdown-menu, .submenu, .tooltip');
    const newElements = [];
    
    afterElements.forEach(el => {{
        if (el.offsetParent !== null) {{  // 可见
            newElements.push({{
                type: el.getAttribute('role') || 'dropdown',
                selector: el.id ? '#' + el.id : el.tagName.toLowerCase(),
                visible: true
            }});
        }}
    }});
    
    return {{
        success: true,
        before_count: beforeCount,
        after_count: afterElements.length,
        new_elements: newElements.length > beforeCount ? newElements : []
    }};
}})()
"#,
        index
    )
}

/// DOM 骨架生成脚本（用于文本模式理解页面结构）
pub const DOM_SKELETON_SCRIPT: &str = r#"
(function() {
    function getStructure(el, depth, maxDepth) {
        if (depth > maxDepth) return '';
        const tag = el.tagName.toLowerCase();
        
        // Skip script, style, svg, etc.
        if (['script', 'style', 'svg', 'noscript', 'template'].includes(tag)) {
            return '';
        }
        
        const role = el.getAttribute('role');
        const id = el.id && !el.id.startsWith('__') ? '#' + el.id : '';
        const ariaLabel = el.getAttribute('aria-label');
        const className = el.className && typeof el.className === 'string' 
            ? '.' + el.className.split(' ')[0].substring(0, 20) 
            : '';
        
        // Key structural tags to expand
        const structuralTags = ['nav', 'main', 'aside', 'header', 'footer', 'section', 'article', 'form', 'ul', 'ol', 'table', 'div'];
        const isStructural = structuralTags.includes(tag) || role;
        
        // For non-structural elements at depth > 1, skip
        if (depth > 1 && !isStructural && !id && !role) {
            return '';
        }
        
        const indent = '  '.repeat(depth);
        let attrs = '';
        if (id) attrs += id;
        if (role) attrs += ' role=' + role;
        if (ariaLabel) attrs += ' aria=' + ariaLabel.substring(0, 20);
        if (!id && className) attrs += className;
        
        // Count interactive children
        const interactiveCount = el.querySelectorAll('a, button, input, select, textarea').length;
        const countStr = interactiveCount > 0 ? ` (${interactiveCount} interactive)` : '';
        
        // Get children structures
        const children = Array.from(el.children)
            .map(c => getStructure(c, depth + 1, maxDepth))
            .filter(s => s)
            .join('\n');
        
        if (children) {
            return `${indent}<${tag}${attrs}>${countStr}\n${children}\n${indent}</${tag}>`;
        } else if (isStructural || id || role) {
            return `${indent}<${tag}${attrs}>${countStr}</${tag}>`;
        }
        return '';
    }
    
    return getStructure(document.body, 0, 3);
})()
"#;

/// 元素增强属性提取脚本（用于文本模式）
pub const ENHANCED_ELEMENT_ATTRIBUTES_SCRIPT: &str = r#"
(function() {
    // 1. 获取已标注元素源
    // MCP tool Usually creates window.__playwrightAnnotatedElements (array or object)
    const source = window.__elements_map__ || window.__playwrightAnnotatedElements;
    
    if (!source) {
        return []; 
    }

    const results = [];
    const elements = Array.isArray(source) ? source : Object.values(source);
    
    // 辅助函数：颜色转换为语义
    function getColorSemantic(r, g, b, a) {
        if (a < 0.1) return 'transparent';
        // 简单判断红色 (Danger)
        if (r > 200 && g < 100 && b < 100) return 'red';
        // 简单判断绿色 (Success)
        if (g > 180 && r < 150 && b < 150) return 'green';
        // 简单判断蓝色 (Action)
        if (b > 200 && r < 100 && g < 150) return 'blue';
        // 简单判断黄色 (Warning)
        if (r > 200 && g > 200 && b < 100) return 'yellow';
        // 简单判断灰色 (Disabled/Neutral)
        if (Math.abs(r-g) < 20 && Math.abs(g-b) < 20) return 'grey';
        return null;
    }

    // 辅助函数：推断隐式标签
    function inferLabel(el) {
        // 1. Check common icon classes
        const className = (el.className || '').toLowerCase();
        if (className.includes('trash') || className.includes('delete') || className.includes('remove')) return 'delete';
        if (className.includes('edit') || className.includes('pen') || className.includes('modify')) return 'edit';
        if (className.includes('search') || className.includes('magnify')) return 'search';
        if (className.includes('close') || className.includes('times')) return 'close';
        if (className.includes('menu') || className.includes('bars') || className.includes('hamburger')) return 'menu';
        if (className.includes('setting') || className.includes('gear') || className.includes('config')) return 'settings';
        if (className.includes('user') || className.includes('profile') || className.includes('account')) return 'profile';
        if (className.includes('add') || className.includes('plus') || className.includes('create')) return 'add';
        
        // 2. Check SVG title/desc
        const svg = el.querySelector('svg');
        if (svg) {
            const title = svg.querySelector('title');
            if (title && title.textContent) return title.textContent;
            if (svg.getAttribute('aria-label')) return svg.getAttribute('aria-label');
        }
        
        // 3. Check tooltip attributes
        if (el.getAttribute('title')) return el.getAttribute('title');
        if (el.getAttribute('data-tooltip')) return el.getAttribute('data-tooltip');
        if (el.getAttribute('data-original-title')) return el.getAttribute('data-original-title'); // Bootstrap
        
        // 4. Check pseudo-elements (::before/::after) for potential icon content
        // Note: content is usually quoted like "\f007", hard to decode but indicates "icon" presence
        const beforeContent = window.getComputedStyle(el, '::before').getPropertyValue('content');
        if (beforeContent && beforeContent !== 'none' && beforeContent !== '""') return 'icon-pseudo';
        
        const afterContent = window.getComputedStyle(el, '::after').getPropertyValue('content');
        if (afterContent && afterContent !== 'none' && afterContent !== '""') return 'icon-pseudo';

        return null;
    }

    elements.forEach((el, index) => {
        if (!el || !el.getBoundingClientRect) return;
        
        const rect = el.getBoundingClientRect();
        const style = window.getComputedStyle(el);
        
        // --- 1. 颜色与状态 ---
        const bgColor = style.backgroundColor; // "rgba(r, g, b, a)"
        const color = style.color;
        const cursor = style.cursor;
        const opacity = parseFloat(style.opacity);
        const textDecoration = style.textDecorationLine || style.textDecoration;
        
        // 解析颜色
        let colorSemantic = null;
        const bgMatch = bgColor.match(/rgba?\((\d+),\s*(\d+),\s*(\d+)/);
        if (bgMatch) {
            colorSemantic = getColorSemantic(parseInt(bgMatch[1]), parseInt(bgMatch[2]), parseInt(bgMatch[3]), 1.0);
        }
        
        // 状态推断
        let derivedState = [];
        if (cursor === 'not-allowed' || cursor === 'no-drop') {
            derivedState.push('disabled');
        }
        if (opacity < 0.5 && opacity > 0) {
            derivedState.push('dimmed');
        }
        if (textDecoration && textDecoration.includes('line-through')) {
            derivedState.push('strikethrough');
        }
        if (style.visibility === 'hidden' || style.display === 'none') {
            derivedState.push('hidden');
        }
        
        // ARIA state inference
        if (el.getAttribute('aria-disabled') === 'true') derivedState.push('disabled');
        if (el.getAttribute('aria-hidden') === 'true') derivedState.push('hidden');
        if (el.getAttribute('aria-expanded') === 'true') derivedState.push('expanded');
        if (el.getAttribute('aria-selected') === 'true') derivedState.push('selected');
        if (el.getAttribute('aria-checked') === 'true') derivedState.push('checked');

        // --- 2. 遮挡检测 ---
        const centerX = rect.x + rect.width / 2;
        const centerY = rect.y + rect.height / 2;
        let isOccluded = false;
        
        // 只有当元素在视口内时才检测
        if (centerX >= 0 && centerY >= 0 && 
            centerX <= window.innerWidth && centerY <= window.innerHeight) {
            
            const topEl = document.elementFromPoint(centerX, centerY);
            if (topEl && topEl !== el && !el.contains(topEl) && !topEl.contains(el)) {
                // 如果顶层元素不是我自己，也不是我的祖先或后代，那我可能被遮挡了
                // 特例：如果是透明遮罩覆盖在上面，也算遮挡，导致无法点击
                isOccluded = true;
                
                // 再次检查 Label 这种特殊情况，点击 Label 等于点击 Input
                if (el.tagName === 'INPUT' && topEl.tagName === 'LABEL' && topEl.getAttribute('for') === el.id) {
                    isOccluded = false;
                }
            }
        }
        
        // --- 3. 隐式语义挖掘 ---
        const inferredLabel = inferLabel(el);

        // --- 4. 模态框检测 ---
        const isInModal = isElementInModal(el);

        results.push({
            index: index, // Assuming the array order matches the annotated index
            
            // 空间布局
            rect: {
                x: Math.round(rect.x),
                y: Math.round(rect.y),
                width: Math.round(rect.width),
                height: Math.round(rect.height)
            },
            
            // 视觉属性
            computedStyles: {
                colorSemantic: colorSemantic,
                cursor: cursor,
                opacity: opacity,
                isBold: style.fontWeight === 'bold' || parseInt(style.fontWeight) >= 600
            },
            
            // 推断状态
            derivedState: derivedState,
            isOccluded: isOccluded,
            
            // 推断文本
            inferredLabel: inferredLabel,

            // 是否在模态框内
            isInModal: isInModal
        });
    });
    
    return results;

    // 辅助函数：检测元素是否在模态框/弹窗内部
    function isElementInModal(el) {
        let parent = el;
        while (parent && parent !== document.body) {
            // 检查常见模态框标识
            if (parent.getAttribute('role') === 'dialog' || 
                parent.getAttribute('role') === 'alertdialog' ||
                parent.getAttribute('aria-modal') === 'true') {
                return true;
            }
            
            // 检查常见类名
            const className = (parent.className || '').toLowerCase();
            if (className.includes('modal') && !className.includes('modal-backdrop') ||
                className.includes('dialog') ||
                className.includes('drawer') ||
                className.includes('popover') ||
                className.includes('popup')) {
                return true;
            }
            
            // 检查 z-index (模态框通常有很高的 z-index)
            const style = window.getComputedStyle(parent);
            const zIndex = parseInt(style.zIndex);
            if (!isNaN(zIndex) && zIndex >= 1000 && style.position === 'fixed') {
                return true;
            }
            
            parent = parent.parentElement;
        }
        return false;
    }
})()
"#;

/// 获取页面可见文本和区域信息脚本（用于文本模式理解页面内容）
pub const VISIBLE_TEXT_AND_REGIONS_SCRIPT: &str = r#"
(function() {
    const result = {
        header_text: '',
        nav_items: [],
        main_headings: [],
        main_content_preview: '',
        sidebar_items: [],
        footer_text: '',
        visible_tables: [],
        visible_lists: [],
        form_labels: [],
        alerts_and_messages: [],
        page_state: {}
    };
    
    // Extract header text
    const header = document.querySelector('header, [role="banner"], .header, .navbar');
    if (header) {
        result.header_text = header.textContent.replace(/\s+/g, ' ').trim().substring(0, 150);
    }
    
    // Extract navigation items
    const navs = document.querySelectorAll('nav, [role="navigation"], .nav, .menu, .sidebar-menu');
    navs.forEach(nav => {
        nav.querySelectorAll('a, [role="menuitem"]').forEach(item => {
            const text = item.textContent.trim();
            if (text && text.length < 50 && result.nav_items.length < 20) {
                result.nav_items.push(text);
            }
        });
    });
    
    // Extract main headings (h1-h3)
    const main = document.querySelector('main, [role="main"], .main-content, .content, #content') || document.body;
    main.querySelectorAll('h1, h2, h3').forEach(h => {
        const text = h.textContent.trim();
        if (text && text.length < 100 && result.main_headings.length < 10) {
            result.main_headings.push(h.tagName + ': ' + text);
        }
    });
    
    // Extract main content preview (first visible paragraphs or list items)
    const contentParts = [];
    main.querySelectorAll('p, li').forEach(el => {
        if (el.offsetParent !== null && contentParts.join(' ').length < 300) {
            const text = el.textContent.trim();
            if (text && text.length > 10 && text.length < 200) {
                contentParts.push(text);
            }
        }
    });
    result.main_content_preview = contentParts.slice(0, 3).join(' | ');
    
    // Extract sidebar items if exists
    const sidebar = document.querySelector('aside, [role="complementary"], .sidebar, .side-panel');
    if (sidebar) {
        sidebar.querySelectorAll('a, button, h4, h5').forEach(item => {
            const text = item.textContent.trim();
            if (text && text.length < 40 && result.sidebar_items.length < 10) {
                result.sidebar_items.push(text);
            }
        });
    }
    
    // Extract footer text
    const footer = document.querySelector('footer, [role="contentinfo"], .footer');
    if (footer) {
        result.footer_text = footer.textContent.replace(/\s+/g, ' ').trim().substring(0, 100);
    }
    
    // Detect visible tables
    document.querySelectorAll('table').forEach((table, idx) => {
        if (table.offsetParent !== null && result.visible_tables.length < 3) {
            const headers = Array.from(table.querySelectorAll('th')).map(th => th.textContent.trim()).filter(t => t).slice(0, 6);
            const rowCount = table.querySelectorAll('tbody tr').length;
            result.visible_tables.push({
                headers: headers.join(', '),
                rows: rowCount
            });
        }
    });
    
    // Detect visible lists
    document.querySelectorAll('ul.list, ol, .card-list, [role="list"]').forEach((list, idx) => {
        if (list.offsetParent !== null && result.visible_lists.length < 3) {
            const itemCount = list.querySelectorAll('li, .card, [role="listitem"]').length;
            const firstItems = Array.from(list.querySelectorAll('li, .card')).slice(0, 3).map(li => li.textContent.trim().substring(0, 40));
            result.visible_lists.push({
                count: itemCount,
                preview: firstItems.join(' | ')
            });
        }
    });
    
    // Extract form labels
    document.querySelectorAll('label, .form-label, .field-label').forEach(label => {
        const text = label.textContent.trim();
        if (text && text.length < 50 && result.form_labels.length < 10) {
            result.form_labels.push(text);
        }
    });
    
    // Detect alerts and messages
    document.querySelectorAll('[role="alert"], .alert, .message, .notification, .toast, .error, .success, .warning').forEach(el => {
        if (el.offsetParent !== null && result.alerts_and_messages.length < 5) {
            const text = el.textContent.trim();
            if (text && text.length < 150) {
                result.alerts_and_messages.push(text);
            }
        }
    });
    
    // Page state detection
    result.page_state = {
        has_loading: !!document.querySelector('.loading, .spinner, [aria-busy="true"]'),
        has_empty_state: !!document.querySelector('.empty, .no-data, .no-results'),
        has_pagination: !!document.querySelector('.pagination, [role="navigation"] .page, .pager'),
        scroll_position: window.scrollY,
        scroll_height: document.documentElement.scrollHeight,
        client_height: document.documentElement.clientHeight,
        can_scroll_more: (document.documentElement.scrollHeight - window.scrollY - document.documentElement.clientHeight) > 100
    };
    
    return result;
})()
"#;

/// 获取表单字段当前状态脚本（用于文本模式表单感知）
pub const FORM_FIELDS_STATE_SCRIPT: &str = r#"
(function() {
    const fields = [];
    
    document.querySelectorAll('input, select, textarea').forEach((el, idx) => {
        // Skip hidden
        if (el.type === 'hidden' || el.offsetParent === null) return;
        
        // Find associated label
        let label = '';
        if (el.id) {
            const labelEl = document.querySelector(`label[for="${el.id}"]`);
            if (labelEl) label = labelEl.textContent.trim();
        }
        if (!label && el.parentElement) {
            const parentLabel = el.parentElement.querySelector('label');
            if (parentLabel) label = parentLabel.textContent.trim();
        }
        if (!label) label = el.getAttribute('aria-label') || el.placeholder || el.name || '';
        
        // Check validation state
        let validationMsg = '';
        if (el.validationMessage) {
            validationMsg = el.validationMessage;
        }
        // Check for sibling error message
        const errorSibling = el.parentElement?.querySelector('.error, .invalid-feedback, .error-message');
        if (errorSibling && errorSibling.textContent.trim()) {
            validationMsg = errorSibling.textContent.trim();
        }
        
        fields.push({
            index: idx,
            tag: el.tagName.toLowerCase(),
            type: el.type || 'text',
            name: el.name,
            label: label.substring(0, 50),
            placeholder: el.placeholder,
            value: el.type === 'password' ? (el.value ? '***' : '') : (el.value || '').substring(0, 30),
            required: el.required,
            disabled: el.disabled,
            readonly: el.readOnly,
            validationError: validationMsg.substring(0, 80),
            options: el.tagName === 'SELECT' ? Array.from(el.options).slice(0, 10).map(o => o.text) : null
        });
    });
    
    return fields;
})()
"#;

/// 检测当前是否存在活动的模态框/抽屉/遮罩层
/// 返回 overlay 信息，包括类型、关闭按钮位置等
pub const DETECT_ACTIVE_OVERLAY_SCRIPT: &str = r#"
(function() {
    const result = {
        hasActiveOverlay: false,
        overlayType: null,       // 'modal', 'drawer', 'popover', 'dropdown', 'fullscreen-menu'
        overlaySelector: null,   // 用于关闭或识别的选择器
        closeButtonIndex: null,  // 关闭按钮在标注元素中的索引（如果能找到）
        elementIndicesInOverlay: [],  // 属于这个 overlay 内部的元素索引
        elementIndicesBlocked: [],    // 被 overlay 遮挡的元素索引
        dismissAction: null      // 建议的关闭方式: 'click_close', 'click_outside', 'press_escape', 'none'
    };
    
    // 1. 检测模态框 (Modal/Dialog)
    const modals = document.querySelectorAll(
        '[role="dialog"]:not([aria-hidden="true"]), ' +
        '[aria-modal="true"]:not([aria-hidden="true"]), ' +
        '.modal.show, .modal.open, .modal.active, .modal.visible, ' + 
        '.dialog.show, .dialog.open, .dialog.active, ' +
        '.popup.show, .popup.open, .popup.active, .popup.visible, ' +
        '[data-state="open"][role="dialog"], ' +
        '.ant-modal-wrap:not(.ant-modal-wrap-hidden), ' +
        '.el-dialog__wrapper:not([style*="display: none"]), ' +
        '.arco-modal-wrapper:not([style*="display: none"])'
    );
    
    for (const modal of modals) {
        if (modal.offsetParent !== null || getComputedStyle(modal).display !== 'none') {
            result.hasActiveOverlay = true;
            result.overlayType = 'modal';
            result.overlaySelector = modal.id ? '#' + modal.id : 
                (modal.className ? '.' + modal.className.split(' ')[0] : 'dialog');
            result.dismissAction = 'click_close';
            break;
        }
    }
    
    // 2. 检测抽屉 (Drawer/Sidebar)
    if (!result.hasActiveOverlay) {
        const drawers = document.querySelectorAll(
            '.drawer.show, .drawer.open, .drawer.active, .drawer.visible, ' +
            '.drawer-open, .drawer-visible, ' +
            '.slide-panel.show, .slide-panel.open, ' +
            '.sidebar.show, .sidebar.open, .sidebar.active, ' +
            '.offcanvas.show, .offcanvas.open, ' +
            '[data-state="open"].drawer, ' +
            '.ant-drawer:not(.ant-drawer-hidden), ' +
            '.el-drawer.el-drawer--open, ' +
            '.arco-drawer:not([style*="display: none"])'
        );
        
        for (const drawer of drawers) {
            if (drawer.offsetParent !== null || getComputedStyle(drawer).display !== 'none') {
                result.hasActiveOverlay = true;
                result.overlayType = 'drawer';
                result.overlaySelector = drawer.id ? '#' + drawer.id : 
                    (drawer.className ? '.' + drawer.className.split(' ')[0] : 'drawer');
                result.dismissAction = 'click_close';
                break;
            }
        }
    }
    
    // 3. 检测下拉菜单/Popover (这些通常点击其他地方就关闭)
    if (!result.hasActiveOverlay) {
        const dropdowns = document.querySelectorAll(
            '[role="menu"]:not([aria-hidden="true"]), ' +
            '[role="listbox"]:not([aria-hidden="true"]), ' +
            '.dropdown-menu.show, .dropdown-menu.open, ' +
            '.popover.show, .popover.open, ' +
            '.ant-dropdown:not(.ant-dropdown-hidden), ' +
            '.el-dropdown-menu:not([style*="display: none"]), ' +
            '[data-state="open"][role="menu"]'
        );
        
        for (const dropdown of dropdowns) {
            if (dropdown.offsetParent !== null) {
                result.hasActiveOverlay = true;
                result.overlayType = 'dropdown';
                result.overlaySelector = dropdown.id ? '#' + dropdown.id : 'dropdown';
                result.dismissAction = 'click_outside';
                break;
            }
        }
    }
    
    // 如果存在活动的 overlay，识别内部元素和被遮挡元素
    if (result.hasActiveOverlay) {
        const source = window.__elements_map__ || window.__playwrightAnnotatedElements;
        if (source) {
            const elements = Array.isArray(source) ? source : Object.values(source);
            const overlay = document.querySelector(result.overlaySelector);
            
            if (overlay) {
                // 获取 overlay 的边界
                const overlayRect = overlay.getBoundingClientRect();
                
                elements.forEach((el, index) => {
                    if (!el || !el.getBoundingClientRect) return;
                    
                    // 检查元素是否在 overlay 内部
                    if (overlay.contains(el)) {
                        result.elementIndicesInOverlay.push(index);
                        
                        // 检查是否是关闭按钮
                        const text = (el.textContent || '').toLowerCase();
                        const ariaLabel = (el.getAttribute('aria-label') || '').toLowerCase();
                        const className = (el.className || '').toLowerCase();
                        
                        if (text.includes('close') || text.includes('关闭') || text.includes('取消') ||
                            text === '×' || text === 'x' ||
                            ariaLabel.includes('close') || ariaLabel.includes('dismiss') ||
                            className.includes('close') || className.includes('dismiss') ||
                            el.getAttribute('data-dismiss') || el.getAttribute('data-close')) {
                            result.closeButtonIndex = index;
                        }
                    } else {
                        // 元素不在 overlay 内，检查是否被遮挡
                        const elRect = el.getBoundingClientRect();
                        const centerX = elRect.x + elRect.width / 2;
                        const centerY = elRect.y + elRect.height / 2;
                        
                        // 检查元素中心是否被 overlay 覆盖
                        if (centerX >= overlayRect.x && centerX <= overlayRect.x + overlayRect.width &&
                            centerY >= overlayRect.y && centerY <= overlayRect.y + overlayRect.height) {
                            result.elementIndicesBlocked.push(index);
                        }
                        
                        // 额外检查：是否有全屏遮罩层（backdrop）
                        const topEl = document.elementFromPoint(centerX, centerY);
                        if (topEl) {
                            const topClass = (topEl.className || '').toLowerCase();
                            if (topClass.includes('backdrop') || topClass.includes('overlay') || 
                                topClass.includes('mask') || topEl.getAttribute('aria-hidden') === 'true') {
                                if (!result.elementIndicesBlocked.includes(index)) {
                                    result.elementIndicesBlocked.push(index);
                                }
                            }
                        }
                    }
                });
            }
        }
    }
    
    return result;
})()
"#;

/// 检测页面是否有懒加载或无限滚动脚本
pub const LAZY_LOAD_DETECTION_SCRIPT: &str = r#"
(function() {
    return {
        has_infinite_scroll: !!(
            document.querySelector('[data-infinite-scroll], .infinite-scroll, .load-more') ||
            document.querySelector('[data-page], [data-next-page]') ||
            window.__INFINITE_SCROLL__ ||
            document.querySelector('[x-data*="scroll"]')
        ),
        has_load_more_button: !!(
            document.querySelector('button:not([disabled])') && 
            Array.from(document.querySelectorAll('button')).some(b => 
                /load\s*more|show\s*more|view\s*more|加载更多|查看更多/i.test(b.textContent)
            )
        ),
        has_pagination: !!(
            document.querySelector('.pagination, .pager, nav[aria-label*="page"]') ||
            document.querySelector('a[href*="page="], a[href*="p="]')
        ),
        at_bottom: (document.documentElement.scrollHeight - window.scrollY - document.documentElement.clientHeight) < 50,
        visible_item_count: document.querySelectorAll('tr, .card, .list-item, [role="listitem"], .item').length
    };
})()
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scripts_not_empty() {
        assert!(!ROUTE_MONITOR_SCRIPT.is_empty());
        assert!(!GET_ROUTES_SCRIPT.is_empty());
        assert!(!DOM_MUTATION_SCRIPT.is_empty());
        assert!(!DETECT_DYNAMIC_COMPONENTS_SCRIPT.is_empty());
        assert!(!EXTRACT_INTERNAL_LINKS_SCRIPT.is_empty());
    }

    #[test]
    fn test_hover_script_generation() {
        let script = hover_element_script(5);
        assert!(script.contains("5"));
        assert!(script.contains("mouseover"));
        assert!(script.contains("mouseenter"));
    }
}
