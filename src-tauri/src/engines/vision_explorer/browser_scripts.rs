//! 浏览器注入脚本
//!
//! 用于注入到页面的 JavaScript 脚本，实现路由监听、DOM 变化检测等功能

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
    format!(r#"
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
"#, index)
}

/// 元素增强属性提取脚本（用于文本模式）
pub const ENHANCED_ELEMENT_ATTRIBUTES_SCRIPT: &str = r#"
(function() {
    // 获取所有可交互元素的增强属性
    const elements = [];
    const interactive = 'a, button, input, select, textarea, [role="button"], [role="link"], [role="menuitem"], [onclick], [tabindex]';
    
    document.querySelectorAll(interactive).forEach((el, index) => {
        // 跳过不可见元素
        if (el.offsetParent === null && el.tagName !== 'INPUT') return;
        
        elements.push({
            index: index,
            tag: el.tagName,
            type: el.getAttribute('type') || el.getAttribute('role') || 'element',
            text: (el.textContent || '').trim().substring(0, 50),
            
            // 语义属性
            ariaLabel: el.getAttribute('aria-label'),
            ariaDescribedby: el.getAttribute('aria-describedby'),
            title: el.getAttribute('title'),
            alt: el.getAttribute('alt'),
            placeholder: el.getAttribute('placeholder'),
            role: el.getAttribute('role'),
            
            // 状态属性
            disabled: el.disabled || el.getAttribute('aria-disabled') === 'true',
            ariaExpanded: el.getAttribute('aria-expanded'),
            ariaHaspopup: el.getAttribute('aria-haspopup'),
            ariaHidden: el.getAttribute('aria-hidden') === 'true',
            
            // 链接
            href: el.getAttribute('href'),
            
            // 表单
            name: el.getAttribute('name'),
            value: el.value || el.getAttribute('value'),
            
            // 类名（用于推断功能）
            className: (el.className || '').substring(0, 100)
        });
    });
    
    return elements;
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
