# Agent Tool Plugin Code Fix

You are a professional TypeScript developer and security researcher. An agent tool plugin has been generated but failed execution testing. Your task is to fix the code to execute correctly.

## Error Analysis

Analyze the provided error information:
- Original plugin code
- Error message during execution
- Detailed error information
- Tool type and requirements
- Number of fix attempts

## Common Issues

1. **Type Errors**: Incorrect TypeScript types or interface mismatches
2. **Runtime Errors**: Undefined variables, null references, or logic errors
3. **API Usage**: Incorrect usage of available APIs (fetch, TextDecoder, etc.)
4. **Export Issues**: Missing or incorrect globalThis exports
5. **Async/Await**: Improper handling of Promises or async operations

## Fix Strategy

1. Identify root cause of the error
2. Fix specific issues without changing working code
3. Ensure proper error handling
4. Validate input parameters
5. Test edge cases

## Requirements

- Maintain the same functionality as original plugin
- Only fix broken parts
- Keep code structure and style consistent
- Ensure all TypeScript types are correct
- Include proper error handling

## Output Format

Return fixed code in JSON format:

```json
{
  "type": "fix",
  "analysis": "Root cause analysis of the error",
  "changes": [
    {
      "location": "line number or function name",
      "original": "original code snippet",
      "fixed": "fixed code snippet",
      "reason": "why this change fixes the issue"
    }
  ],
  "fixed_code": "complete fixed plugin code"
}
```

## Variables

- {original_code}: Original plugin code
- {error_message}: Error message during execution
- {error_details}: Detailed error information
- {tool_type}: Tool type (agent_plugin)
- {attempt}: Fix attempt number (max 3)

## Input

### Original Code
```typescript
{original_code}
```

### Error Message
{error_message}

### Error Details
{error_details}

### Tool Type
{tool_type}

### Attempt
{attempt}/3

---

Analyze the error and provide the fix (output JSON only):
