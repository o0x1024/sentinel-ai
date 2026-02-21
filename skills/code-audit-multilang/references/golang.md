# Golang Audit Focus

## Frameworks To Detect

- net/http, Gin, Echo, Fiber, gRPC services.

## High-Risk Checks

- Missing auth middleware on sensitive routes.
- IDOR due to tenant/user ownership checks missing.
- SQL injection via string concatenation with `database/sql`.
- Command injection via `os/exec` with user input.
- SSRF through unrestricted HTTP client destinations.
- Path traversal in file serving/downloading.
- Insecure randomness for tokens (`math/rand` misuse).
- Hardcoded secrets or weak secret loading.

## Quick Grep Targets

- `exec.Command(`
- `fmt.Sprintf(\"SELECT`
- `http.Get(`
- `ioutil.ReadFile(`, `os.ReadFile(`
- `math/rand`
- `gin.Default()`

## Safe Patterns To Prefer

- Context-based authn/authz middleware.
- DSN and query parameters with placeholders only.
- URL allowlist for outbound calls.
- `crypto/rand` for credentials/tokens.
