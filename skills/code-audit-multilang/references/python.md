# Python Audit Focus

## Frameworks To Detect

- Django, Flask, FastAPI, Tornado, Celery workers.

## High-Risk Checks

- Broken authz on object-level access.
- SQL injection in raw SQL/ORM escape hatches.
- Unsafe deserialization (`pickle`, `yaml.load` unsafe modes).
- Command injection via `subprocess` shell execution.
- SSTI in Jinja2 with untrusted templates.
- Path traversal in upload/download handlers.
- SSRF in requests/httpx clients.
- Insecure secret management in settings/env handling.

## Quick Grep Targets

- `subprocess.run(`, `shell=True`
- `pickle.loads(`
- `yaml.load(`
- `cursor.execute(f\"`
- `render_template_string(`
- `requests.get(`

## Safe Patterns To Prefer

- Use safe loaders and typed schema validation.
- Enforce object ownership/tenant checks in service layer.
- Avoid dynamic code execution and shell usage.
