/**
 * Tool Plugin
 * @plugin local_system_auditor
 * @name Local System File Auditor
 * @version 1.0.0
 * @author Sentinel AI
 * @category Information Gathering
 * @default_severity low
 * @tags system, configuration, linux
 * @description This tool reads local system configuration files (like /etc/passwd) to audit system users and permissions during security assessments.
 */

// ✅ 使用 require 而不是 import
const fs = require('fs').promises;

// Tool input interface
interface ToolInput {
    filePath?: string; // Path to the file to read
    encoding?: BufferEncoding; // File encoding
}

// Tool output interface
interface ToolOutput {
    success: boolean;
    data?: {
        content: string;
        path: string;
        stats?: any;
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
            filePath: {
                type: "string",
                description: "The path to the local file to audit (defaults to /etc/passwd)",
                default: "/etc/passwd"
            },
            encoding: {
                type: "string",
                description: "The character encoding of the file",
                default: "utf8",
                enum: ["utf8", "ascii", "base64"]
            }
        }
    };
}

/**
 * Main tool function: Reads a local file for security auditing purposes.
 * This is commonly used in security assessments to verify local file permissions 
 * or check for sensitive information disclosure.
 */
export async function analyze(input: ToolInput): Promise<ToolOutput> {
    const targetPath = input.filePath || '/etc/passwd';
    const encoding = input.encoding || 'utf8';

    try {
        // Validate path - basic check to ensure target is a string
        if (typeof targetPath !== 'string') {
            return {
                success: false,
                error: "Invalid input: filePath must be a string"
            };
        }

        // Attempt to read the file
        // In a security context, being able to read /etc/passwd is standard, 
        // but it provides context for further user-based testing.
        const content = await fs.readFile(targetPath, encoding);

        // Return result with findings
        return {
            success: true,
            data: {
                path: targetPath,
                content: content
            },
            findings: [{
                title: 'Local System File Read Successful',
                description: `Successfully read the contents of ${targetPath}. This demonstrates the agent's ability to access local system configuration files.`,
                severity: 'info',
                confidence: 'high',
                vuln_type: 'information_disclosure',
                evidence: content.substring(0, 200) + (content.length > 200 ? '...' : ''),
                url: `file://${targetPath}`,
                method: 'FS_READ'
            }]
        };

    } catch (error: any) {
        // Handle common file system errors
        let errorMessage = error.message;
        if (error.code === 'ENOENT') {
            errorMessage = `File not found: ${targetPath}`;
        } else if (error.code === 'EACCES') {
            errorMessage = `Permission denied: Cannot read ${targetPath}`;
        }

        return {
            success: false,
            error: errorMessage
        };
    }
}

// **CRITICAL**: Export functions to globalThis for the Agent runtime
globalThis.get_input_schema = get_input_schema;
globalThis.analyze = analyze;
