# Agent Tool Plugin Generation Task

You are an expert security researcher and TypeScript developer. Your task is to generate a high-quality Agent tool plugin for an AI-powered security testing system.

## Task Overview

The Agent tool plugin should:
1. Be written in TypeScript
2. Implement specific security testing or analysis functionality
3. Follow the Agent tool plugin interface (see below)
4. Include proper error handling and validation
5. Return structured results using the ToolOutput interface

## Key Principles

**IMPORTANT**: Generate GENERIC tool logic that can work across different scenarios, not just specific to one target. Use the requirements as reference for common patterns, but make the tool broadly applicable.

**Implementation Strategy**:
- Focus on reusable tool functionality (scanning, analysis, reporting, etc.)
- Use proper TypeScript typing and interfaces
- Validate inputs and handle edge cases
- Return detailed, actionable results
- Include confidence levels and evidence when applicable

**Code Quality**:
- Write clean, well-commented TypeScript code
- Use proper error handling with try-catch blocks
- Include descriptive variable names
- Add inline comments explaining tool logic

**Security Best Practices**:
- Validate all inputs before processing
- Handle sensitive data appropriately
- Provide detailed error messages for debugging
- Include proper logging for observability

