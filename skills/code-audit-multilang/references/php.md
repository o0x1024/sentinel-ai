# PHP Audit Focus

## Frameworks To Detect

- Laravel, Symfony, ThinkPHP, WordPress, custom MVC.

## High-Risk Checks

- SQL injection in raw query builders (`DB::raw`, string concat SQL).
- Unsafe unserialize and PHAR deserialization.
- File inclusion (`include`, `require`) with user-controlled paths.
- Command execution (`exec`, `shell_exec`, backticks).
- Upload validation bypass and webroot write.
- Weak session config, missing `HttpOnly/Secure/SameSite`.
- JWT validation mistakes (alg confusion, no exp/aud checks).
- Template injection in Twig/Blade custom rendering.

## Quick Grep Targets

- `unserialize(`
- `eval(`
- `shell_exec(`, `exec(`, `system(`
- `DB::raw(`, `whereRaw(`
- `move_uploaded_file(`
- `include $_`, `require $_`

## Safe Patterns To Prefer

- Parameterized ORM/query builder.
- Strict allowlist for file extensions and MIME validation.
- Signed URLs and storage outside webroot.
