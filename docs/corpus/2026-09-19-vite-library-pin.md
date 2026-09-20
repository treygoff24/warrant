# Vite library corpus pin

Status: source shape and frozen dependency acquisition verified on 2026-09-19. A full build was not run.

- Repository: [`jasonsturges/vite-typescript-npm-package`](https://github.com/jasonsturges/vite-typescript-npm-package)
- Commit: [`e182cf8e135fb072c4ddf261f5ac34423fd067ea`](https://github.com/jasonsturges/vite-typescript-npm-package/commit/e182cf8e135fb072c4ddf261f5ac34423fd067ea)
- Shape: Vite library mode with TypeScript declarations
- Lockfile and installer: `package-lock.json`, installed with npm 11.19.0 and lifecycle scripts disabled
- Observed TypeScript: 5.9.3

[`vite.config.ts`](https://github.com/jasonsturges/vite-typescript-npm-package/blob/e182cf8e135fb072c4ddf261f5ac34423fd067ea/vite.config.ts) has an explicit `build.lib` entry and emits ES, CommonJS, and IIFE formats. [`package.json`](https://github.com/jasonsturges/vite-typescript-npm-package/blob/e182cf8e135fb072c4ddf261f5ac34423fd067ea/package.json) publishes conditional import and require entries. The repository was chosen because the library-mode boundary is visible in twelve tracked files.

The package declares ISC in `package.json` but has no license file at this pin. Redistribution needs a separate rights review. Digests and the complete dependency bundle identity are in `tests/corpus/manifest.yaml`.
