# CommonJS library corpus pin

Status: source shape and frozen dependency acquisition verified on 2026-09-19. A full build was not run.

- Repository: [`zaki-yama/typescript-npm-package-template`](https://github.com/zaki-yama/typescript-npm-package-template)
- Commit: [`92b6f49414a5200daab958b174598ca7dff43a5a`](https://github.com/zaki-yama/typescript-npm-package-template/commit/92b6f49414a5200daab958b174598ca7dff43a5a)
- Shape: TypeScript library with a CommonJS output
- Lockfile and installer: `package-lock.json`, installed with npm 11.19.0 and lifecycle scripts disabled
- Observed TypeScript: 3.6.2

[`tsconfig.cjs.json`](https://github.com/zaki-yama/typescript-npm-package-template/blob/92b6f49414a5200daab958b174598ca7dff43a5a/tsconfig.cjs.json) emits CommonJS into `lib`, and `package.json` makes `lib/index.js` the primary entry. The same source also emits ESM and UMD. It was chosen because the CommonJS resolution surface is explicit and the repository is small; the dual outputs keep Warrant from assuming that one repository has only one module format.

The package declares MIT in `package.json` but has no license file at this pin. The dependency lock is old and npm reports deprecated packages during acquisition. The bundle is inert because install scripts are disabled, but it is test input, not trusted executable code. Redistribution needs a separate rights review. Digests and the complete dependency bundle identity are in `tests/corpus/manifest.yaml`.
