/**
 * @plugin _etc_passwd_
 * @name 
 * @version 1.0.0
 * @author AI Generated
 * @category 
 * @default_severity medium
 * @tags ai-generated, agent
 * @description 生成一个读取本地/etc/passwd文件的脚本
 */


interface ToolInput {
    path?: string; // Path to the file, defaults to /etc/passwd
}

interface UserEntry {
    username: string;
    passwordIndicator: string;
    uid: number;
    gid: number;
    comment: string;
    home: string;
    shell: string;
}

interface ToolOutput {
    success: boolean;
    data?: {
        path: string;
        raw_content: string;
        parsed_users: UserEntry[];
        total_users: number;
        sensitive_shells: string[]; // List of users with interactive shells
    };
    error?: string;
}

/**
 * Export parameter schema function (Required)
 * Defines the input structure for the plugin engine.
 */
export function get_input_schema() {
    return {
        type: "object",
        properties: {
            path: {
                type: "string",
                default: "/etc/passwd",
                description: "The absolute path to the password file to read (standard is /etc/passwd)"
            }
        }
    };
}

/**
 * Main tool function to read and parse system user information.
 */
export async function analyze(input: ToolInput): Promise<ToolOutput> {
    const filePath = input.path || "/etc/passwd";

    try {
        // Log the start of the operation
        Deno.core.ops.op_plugin_log('info', `Attempting to read system file: ${filePath}`);

        // Check if file exists and get info
        const fileInfo = await Deno.stat(filePath);
        if (!fileInfo.isFile) {
            return {
                success: false,
                error: `Target path ${filePath} is not a regular file.`
            };
        }

        // Read the file content
        const content = await Deno.readTextFile(filePath);
        
        // Parse the /etc/passwd format
        // Format: name:password:UID:GID:comment:home:shell
        const lines = content.split('\n');
        const users: UserEntry[] = [];
        const interactiveShells = ['/bin/bash', '/bin/sh', '/bin/zsh', '/usr/bin/bash', '/usr/bin/zsh', '/bin/dash'];
        const sensitiveShells: string[] = [];

        for (const line of lines) {
            const trimmedLine = line.trim();
            if (!trimmedLine || trimmedLine.startsWith('#')) continue;

            const parts = trimmedLine.split(':');
            if (parts.length >= 7) {
                const user: UserEntry = {
                    username: parts[0],
                    passwordIndicator: parts[1],
                    uid: parseInt(parts[2], 10),
                    gid: parseInt(parts[3], 10),
                    comment: parts[4],
                    home: parts[5],
                    shell: parts[6]
                };
                
                users.push(user);

                // Identify users with potentially interactive shells (auditing purpose)
                if (interactiveShells.some(s => user.shell.includes(s))) {
                    sensitiveShells.push(user.username);
                }
            }
        }

        return {
            success: true,
            data: {
                path: filePath,
                raw_content: content,
                parsed_users: users,
                total_users: users.length,
                sensitive_shells: sensitiveShells
            }
        };

    } catch (error) {
        Deno.core.ops.op_plugin_log('error', `Failed to read ${filePath}: ${String(error)}`);
        return {
            success: false,
            error: error instanceof Error ? error.message : String(error)
        };
    }
}

// **CRITICAL**: Export functions to globalThis for the plugin engine
globalThis.get_input_schema = get_input_schema;
globalThis.analyze = analyze;