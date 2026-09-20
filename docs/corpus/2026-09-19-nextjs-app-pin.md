# Next.js application corpus pin

Status: source shape and frozen dependency acquisition verified on 2026-09-19. A full build was not run.

- Repository: [`vercel/nextjs-postgres-auth-starter`](https://github.com/vercel/nextjs-postgres-auth-starter)
- Commit: [`fde8ecf1da9337223081f70cf88b420060039d6e`](https://github.com/vercel/nextjs-postgres-auth-starter/commit/fde8ecf1da9337223081f70cf88b420060039d6e)
- Shape: Next.js application using the App Router
- Lockfile and installer: `pnpm-lock.yaml`, installed with pnpm 8.15.9 and lifecycle scripts disabled
- Observed TypeScript: 5.3.3

This repository was chosen because the shape is small and direct. [`app/page.tsx`](https://github.com/vercel/nextjs-postgres-auth-starter/blob/fde8ecf1da9337223081f70cf88b420060039d6e/app/page.tsx) is a file-system route, `package.json` pins Next 14.0.4, and [`tsconfig.json`](https://github.com/vercel/nextjs-postgres-auth-starter/blob/fde8ecf1da9337223081f70cf88b420060039d6e/tsconfig.json) carries the Next plugin and path mappings. That makes it useful for qualifying route recognition without bringing in a large application.

The repository has no license declaration at this pin. Its metadata is publishable and its source is publicly obtainable, but redistribution of the source or dependency archives needs a separate rights review. Digests and the complete dependency bundle identity are in `tests/corpus/manifest.yaml`.
