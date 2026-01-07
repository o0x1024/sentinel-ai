# Web Search Tool

## Overview

The `web_search` tool provides real-time web search capabilities using the Tavily API. It allows AI agents to search the internet for current information, documentation, CVEs, security advisories, CTF writeups, and more.

## Configuration

### API Key Setup

The tool requires a Tavily API key. You can configure it in two ways:

1. **Environment Variable** (Recommended):
   ```bash
   export TAVILY_API_KEY="your_api_key_here"
   ```

2. **AI Settings**:
   - Navigate to Settings → AI Settings
   - Add `tavily_api_key` in the configuration

### Get Tavily API Key

1. Visit [Tavily API](https://tavily.com/)
2. Sign up for an account
3. Get your API key from the dashboard

## Usage

### Tool Parameters

```typescript
{
  "query": string,          // Search query (required)
  "max_results": number,    // Maximum number of results (default: 5)
  "search_depth": string    // "basic" or "advanced" (default: "basic")
}
```

### Example Usage

#### Basic Search
```json
{
  "query": "CVE-2024 SQL injection vulnerabilities",
  "max_results": 5,
  "search_depth": "basic"
}
```

#### Advanced Search
```json
{
  "query": "CTF writeup UNION SQL injection bypass WAF",
  "max_results": 10,
  "search_depth": "advanced"
}
```

### Response Format

```json
{
  "query": "search query",
  "results": [
    {
      "title": "Result Title",
      "url": "https://example.com",
      "content": "Relevant content snippet..."
    }
  ],
  "total_results": 5,
  "source": "Tavily"
}
```

## Use Cases

### 1. Security Research
Search for latest CVEs, security advisories, and vulnerability information:
```
"query": "CVE-2024-1234 exploitation details"
```

### 2. CTF Challenges
Find writeups, techniques, and solutions:
```
"query": "CTF SQL injection double URL encoding bypass"
```

### 3. Documentation Lookup
Search for library documentation and API references:
```
"query": "Python requests library proxy configuration"
```

### 4. Exploit Research
Find public exploits and PoC code:
```
"query": "Apache Struts RCE exploit GitHub"
```

## Integration with AI Agents

The tool is automatically available to AI agents when:
- Tools are enabled in the agent configuration
- The tool is not in the disabled tools list
- A valid Tavily API key is configured

### Agent Prompt Example

```
User: "Search for recent SQL injection vulnerabilities in WordPress plugins"

Agent: I'll search for that information.
[Calls web_search tool with query: "WordPress plugin SQL injection vulnerability 2024"]

Agent: Based on the search results, here are the recent vulnerabilities...
```

## Features

- **Real-time Search**: Get current information from the web
- **Proxy Support**: Automatically uses configured global proxy
- **Configurable Depth**: Choose between basic and advanced search
- **Rich Results**: Returns titles, URLs, and content snippets
- **Error Handling**: Graceful error messages for API issues

## Limitations

- Requires internet connection
- Rate limited by Tavily API plan
- Search results depend on Tavily's index
- API key required for operation

## Troubleshooting

### "API key not configured" Error
- Ensure `TAVILY_API_KEY` environment variable is set
- Or configure `tavily_api_key` in AI settings

### "Request failed" Error
- Check internet connection
- Verify API key is valid
- Check if proxy settings are correct
- Ensure Tavily API service is available

### No Results Returned
- Try different search queries
- Use more specific keywords
- Increase `max_results` parameter
- Try `search_depth: "advanced"` for broader results

## Best Practices

1. **Be Specific**: Use specific keywords for better results
2. **Limit Results**: Start with fewer results (5-10) to reduce latency
3. **Use Basic Depth**: Use "basic" search depth for faster responses
4. **Cache Results**: Store frequently accessed information locally
5. **Combine with Other Tools**: Use with `http_request` to fetch full content

## Example Workflow

```
1. User asks about a security topic
2. Agent uses web_search to find relevant information
3. Agent uses http_request to fetch detailed content from URLs
4. Agent analyzes and summarizes findings
5. Agent provides comprehensive answer to user
```

## Related Tools

- `http_request`: Fetch full content from search result URLs
- `shell`: Execute commands based on search findings
- `task_planner`: Plan multi-step research tasks
