# Node.js Audit Focus

## Frameworks To Detect

- Express, Koa, NestJS, Fastify, Next.js API routes.

## High-Risk Checks

- Missing authorization checks beyond authentication.
- Prototype pollution entry points.
- NoSQL injection (Mongo operators from user input).
- SQL injection with raw query strings.
- Command injection (`child_process` execution).
- Path traversal and unsafe static file serving.
- SSRF with user-controlled URLs.
- JWT verification bypass or weak secret handling.
- Unsafe template rendering/XSS in SSR flows.

## Quick Grep Targets

- `child_process.exec(`
- `jwt.verify(`
- `$where`, `$gt`, `$ne`
- `res.send(`
- `fs.readFile(`
- `axios.get(`, `fetch(`

## Safe Patterns To Prefer

- Schema validation (zod/joi/class-validator) at boundaries.
- Strict object sanitization for query/filter payloads.
- Escaping/encoding by output context.
