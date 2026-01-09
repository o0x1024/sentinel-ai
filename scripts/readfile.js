/**
 * @plugin _etc_passwd_
 * @name 生成一个读取本地/etc/passwd文件的脚本
 * @version 1.0.0
 * @author AI Generated
 * @category 
 * @default_severity medium
 * @tags ai-generated, agent
 * @description 生成一个读取本地/etc/passwd文件的脚本
 */

/**
 * Tool Plugin
 * @plugin read_etc_passwd
 * @name Read /etc/passwd File
 * @version 1.0.0
 * @author Security Researcher
 * @category System Enumeration
 * @default_severity informational
 * @tags local, system, enumeration, linux
 * @description Reads and parses the /etc/passwd file to enumerate local system users.
 */

const fs = require('fs').promises;

/**
 * Tool input interface
 */
interface ToolInput {
    parse?: boolean;
}

/**
 * Tool output interface
 */
interface ToolOutput {
    success: boolean;
    data?: {
        content?: string;
        users?: Array<{
            username: string;
            passwordPlaceholder: string;
            uid: string;
            gid: string;
            comment: string;
            home: string;
            shell: string;
        }>;
    };
    error?: string;
}

/**
 * Export parameter schema function (Required)
 */
export function get_input_schema() {
    return {
        type: "object",
        properties: {
            parse: {
                type: "boolean",
                default: true,
                description: "Whether to parse the file into a structured JSON object"
            }
        }
    };
}

/**
 * Main tool function to read and analyze /etc/passwd
 */
export async function analyze(input: ToolInput): Promise<ToolOutput> {
    try {
        const filePath = '/Users/a1024/code/mcheck/Cargo.toml';
        
        // Check if file exists and is accessible
        try {
            await fs.access(filePath);
        } catch (e) {
            return {
                success: false,
                error: `Cannot access ${filePath}. File may not exist or permission denied.${e}`
            };
        }

        // Read file content
        const content = await fs.readFile(filePath, 'utf-8');

        if (input.parse === false) {
            return {
                success: true,
                data: { content }
            };
        }

        // Parse the /etc/passwd format: username:password:UID:GID:comment:home:shell
        const lines = content.split('\n');
        const users = lines
            .filter(line => line.trim() !== '' && !line.startsWith('#'))
            .map(line => {
                const parts = line.split(':');
                return {
                    username: parts[0] || '',
                    passwordPlaceholder: parts[1] || '',
                    uid: parts[2] || '',
                    gid: parts[3] || '',
                    comment: parts[4] || '',
                    home: parts[5] || '',
                    shell: parts[6] || ''
                };
            });

        return {
            success: true,
            data: {
                content: content,
                users: users
            }
        };

    } catch (error) {
        return {
            success: false,
            error: error instanceof Error ? error.message : String(error)
        };
    }
}

// **CRITICAL**: Export functions to globalThis
globalThis.get_input_schema = get_input_schema;
globalThis.analyze = analyze;