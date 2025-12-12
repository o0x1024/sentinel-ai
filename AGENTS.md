# Repository Guidelines

## Project Structure & Module Organization

- `src/`: Vue 3 + TypeScript frontend (SFCs with `<script setup>`). Key areas include `src/components/`, `src/views/`, `src/services/`, `src/composables/`, and `src/i18n/`.
- `src/tests/`: Frontend tests. Subfolders: `unit/`, `integration/`, `e2e/`, plus shared setup in `src/tests/setup.ts`.
- `src-tauri/`: Tauri 2 Rust workspace. App shell is `src-tauri/src/`; internal crates live in `src-tauri/sentinel-*` (e.g., `sentinel-core`, `sentinel-tools`, `sentinel-llm`).
- `packages/`: Extra frontend packages (currently `packages/workflow-studio`).
- `docs/`: VitePress documentation site.
- `public/`: Static assets; `dist/` is build output. `scripts/` contains local utilities.

## Build, Test, and Development Commands

Use `yarn` (lockfile is `yarn.lock`), but `npm run …` works too.

- `yarn dev`: Start Vite frontend dev server.
- `yarn tauri dev`: Run Tauri app with live frontend.
- `yarn build`: Build frontend to `dist/`.
- `yarn build:release`: Frontend build + Tauri release bundle.
- `yarn lint`: ESLint with auto-fix.
- `yarn format`: Prettier on `src/`.
- `yarn test`, `yarn test:watch`: Vitest unit/integration suite.
- `yarn test:coverage`: Vitest with v8 coverage (80% global thresholds).
- `yarn test:e2e`: Playwright e2e tests (expects `tauri dev` webServer).
- `yarn docs:dev`: Run docs locally.

## Coding Style & Naming Conventions

- Indentation: 2 spaces; no tabs. Prettier enforces no semicolons and single quotes.
- Vue components: `PascalCase` filenames and component names; templates use `PascalCase` tags.
- TypeScript: prefer `const`, avoid `var`; keep modules small and focused under `src/services/` and `src/composables/`.
- Run `yarn lint`/`yarn format` before pushing.

## Testing Guidelines

- Unit/integration: Vitest (`vitest.config.ts`) with `jsdom` and globals. Test files live alongside code or under `src/tests/**` and match `*.test.ts` / `*.spec.ts`.
- E2E: Playwright in `src/tests/e2e/`; add stable selectors and avoid timing-based assertions.

## Commit & Pull Request Guidelines

- History uses short, imperative summaries (often Chinese). Keep commits atomic and descriptive; optional scope prefix is welcome, e.g., `core: fix agent tool routing`.
- PRs should include: purpose, key changes, how to test, and any UI screenshots/GIFs when relevant. Link related issues or docs updates in `docs/`.

## Security & Configuration Tips

- Tauri capabilities and app config are in `src-tauri/` (`tauri.conf.json`, `capabilities/`). Review permissions when adding native features.
- Do not commit secrets; use local env/config files and document required keys in `docs/` if added.
