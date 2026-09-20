# pnpm project-reference corpus pin

Status: source shape and frozen dependency acquisition verified on 2026-09-19. A full build was not run.

- Repository: [`ibezkrovnyi/pnpm-monorepo-typescript-project-references`](https://github.com/ibezkrovnyi/pnpm-monorepo-typescript-project-references)
- Commit: [`dbf42905a1c1c0443ca575c4cb25da0b401e6aba`](https://github.com/ibezkrovnyi/pnpm-monorepo-typescript-project-references/commit/dbf42905a1c1c0443ca575c4cb25da0b401e6aba)
- Shape: pnpm workspace with TypeScript project references
- Lockfile and installer: `pnpm-lock.yaml`, installed with pnpm 7.33.7 and lifecycle scripts disabled
- Observed TypeScript: 4.2.4

[`pnpm-workspace.yaml`](https://github.com/ibezkrovnyi/pnpm-monorepo-typescript-project-references/blob/dbf42905a1c1c0443ca575c4cb25da0b401e6aba/pnpm-workspace.yaml) selects packages and components. The root [`tsconfig.json`](https://github.com/ibezkrovnyi/pnpm-monorepo-typescript-project-references/blob/dbf42905a1c1c0443ca575c4cb25da0b401e6aba/tsconfig.json) references the frontend and backend projects, and `tsconfig-base.json` enables composite builds. The repository was chosen because these relationships fit in a twenty-file tree, so a resolution failure will be easier to diagnose than the same failure in a large monorepo.

The root `LICENSE` contains the Apache License 2.0. The root `package.json` and all four workspace `package.json` files declare ISC. These declarations are recorded without drawing a rights conclusion. The dependency bundle still needs its own redistribution review. Digests and the complete dependency bundle identity are in `tests/corpus/manifest.yaml`.
