# Website

The user-facing documentation site, built with [Docusaurus](https://docusaurus.io/).
It lives in `website/` and is deployed to GitHub Pages by
`.github/workflows/gh-pages-deploy.yml`.

## Local Development

```bash
pnpm install
pnpm start
```

`pnpm start` regenerates the changelog page from `CHANGELOG.md` and then starts a
local dev server with live reload.

## Build

```bash
pnpm build
```

The output is written to `website/build/`.

## Deployment

Deployment is automated: pushes to `main` build the site and publish it with
GitHub Actions. No manual deploy step is needed.
