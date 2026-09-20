# Path-alias and package-export corpus pin

Status: source shape and frozen dependency acquisition verified on 2026-09-19. A full build was not run.

- Repository: [`pmndrs/zustand`](https://github.com/pmndrs/zustand)
- Commit: [`b57db4f86ef179285da216eeb291266da82c361c`](https://github.com/pmndrs/zustand/commit/b57db4f86ef179285da216eeb291266da82c361c)
- Shape: TypeScript path mappings paired with conditional package exports
- Lockfile and installer: `pnpm-lock.yaml`, installed with pnpm 11.3.0 and lifecycle scripts disabled
- Observed TypeScript: 6.0.3

The root [`tsconfig.json`](https://github.com/pmndrs/zustand/blob/b57db4f86ef179285da216eeb291266da82c361c/tsconfig.json) maps `zustand` and `zustand/*` directly to source files. [`package.json`](https://github.com/pmndrs/zustand/blob/b57db4f86ef179285da216eeb291266da82c361c/package.json) publishes separate React Native, import, and default targets for the root and wildcard subpaths. The repository was chosen because one compact library forces the resolver to reconcile compiler-time aliases with runtime package exports.

The repository carries an MIT license. The dependency bundle still needs its own redistribution review. Digests and the complete dependency bundle identity are in `tests/corpus/manifest.yaml`.
