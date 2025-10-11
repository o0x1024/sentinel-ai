# Repository Guidelines

## Project Structure & Module Organization
- Frontend (Vue 3 + Vite): `src/` (components, views, composables, services, types), static assets in `public/`, built files in `dist/`.
- Desktop backend (Tauri + Rust): `src-tauri/` (commands, services, models, engines). Tauri config in `src-tauri/tauri.conf.json`.
- Tests: TypeScript unit/integration/e2e in `src/tests/`; Playwright e2e in `src/tests/e2e/`. Rust tests run inside `src-tauri/`.
- Docs and scripts: `docs/`, `scripts/`. Entry HTML in `index.html`.

## Build, Test, and Development Commands
- Run web app: `npm run dev` (Vite dev server).
- Build web app: `npm run build` (use `build:check` to type-check first).
- Preview build: `npm run preview`.
- Desktop (Tauri): `npm run tauri dev` for dev; `npm run build:release` for production build.
- Unit/integration tests: `npm run test` (Vitest), UI runner `npm run test:ui`.
- Coverage: `npm run test:coverage`.
- E2E: `npm run test:e2e` (Playwright), or `npm run test:e2e:ui`.
- Lint/format: `npm run lint` and `npm run format`. Type check: `npm run type-check`.

## Coding Style & Naming Conventions
- Prettier enforces: 2-space indent, single quotes, no semicolons, width 100. See `.prettierrc`.
- ESLint: Vue 3 essential + TS rules; prefer `const`, no `var`. See `.eslintrc.cjs`.
- Vue components: PascalCase filenames (e.g., `DynamicFlowAdjustment.vue`). Composables: `useXxx.ts`. Avoid multi-word component rule is disabled.
- Rust (Tauri): keep modules small, run `cargo fmt` in `src-tauri/` before committing.

## Testing Guidelines
- Unit tests: place near code or under `src/tests/unit` using `*.test.ts` or `*.spec.ts`.
- E2E: add Playwright specs under `src/tests/e2e`.
- Run all TS tests with `npm run test`; generate coverage with `npm run test:coverage`.
- Rust tests: run from `src-tauri/` with `cargo test`.

## Commit & Pull Request Guidelines
- Commits: concise, imperative subject. Prefer Conventional Commits where possible (e.g., `feat: add performance report`, `fix(scan): handle error states`).
- PRs: include clear description, linked issues, screenshots for UI, and steps to reproduce/verify. Ensure CI passes: build, lint, tests.

## Security & Configuration Tips
- Configure API keys via environment variables (Rust backend loads `*_API_KEY` vars). Never commit secrets.
- Tauri dev host: set `TAURI_DEV_HOST` if needed (see `vite.config.ts`).
- Review `CLAUDE.md` and architecture docs for agent-related context.
