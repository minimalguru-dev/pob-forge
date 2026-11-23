# CDN

A Cloudflare Worker in front of an R2 bucket. It serves what the app downloads instead of shipping in
every installer, and the updater manifest. Both fit Cloudflare's free tier.

| Path | What | Where it comes from |
|---|---|---|
| `tree/<sha256>.<ext>` | Passive tree art | `bun run sync`, then `bun run downloads upload` |
| `data/<sha256>` | PoE1 timeless jewel data | same |
| `maps/<game>/latest.json`, `<game>/Art/…?v=<tag>` | Item art | copied from `ORIGIN` on first request |
| `update/latest.json`, `update/<version>/…` | Updater manifest and signed bundles | publishing a GitHub release (`publish-cdn.yml`), or `bun run publish:update`; without a manifest in the bucket, the latest GitHub release of `UPDATE_REPO` |
| `download/PoE-Forge-setup.exe` | Installer behind the download link | publishing a release, as above |

## Set up

1. Create an R2 bucket named `pob-forge` (or change `bucket_name` in `wrangler.toml`).
2. Deploy: `bun x wrangler login`, then `bun x wrangler deploy` in this folder. The Worker answers on
   the custom domain in `wrangler.toml` (`cdn.netrum.eu`, whose zone must be on the same account);
   the workers.dev address is off.
3. Upload the files `bun run sync` produced. Create an R2 API token with write access, then from the
   repository root:

   ```sh
   R2_ACCOUNT_ID=… R2_ACCESS_KEY_ID=… R2_SECRET_ACCESS_KEY=… R2_BUCKET=pob-forge bun run downloads upload
   POB_FORGE_CDN_URL=https://cdn.netrum.eu bun run downloads check
   ```

4. In the GitHub repository, set the variable `POB_FORGE_CDN_URL` to the Worker's address. Release and
   check builds read it; a local `bun run tauri build` needs it in the environment too.

## Moving files from another server

Deploy with the server you are moving from as `ORIGIN`: `bun x wrangler deploy --var ORIGIN:<url>`
keeps the address out of the repository (setting it in `wrangler.toml` works too). Every file the
bucket lacks is then fetched there once, stored, and served from the bucket afterwards. Item-art maps
are refreshed from `ORIGIN` at most once an hour. A deploy without `--var` clears `ORIGIN`; do that
once the bucket holds everything you need, and from then on nothing is fetched from elsewhere.

## Updates

Installed copies ask `update/latest.json` on this Worker, and nothing else, for a new version.
Publishing a GitHub release runs `publish-cdn.yml` (`bun run publish:update --release <tag>`), which checks each
bundle's updater signature, uploads the bundles to `update/<version>/` and the Windows installer to the download
link, and writes `update/latest.json` last; the manifest is served with `no-cache`, the installers as
immutable. Until the bucket holds a manifest, the Worker forwards the latest GitHub release's
`latest.json` instead. The installers are signed with your updater key and the app checks that
signature before installing; see [CONTRIBUTING.md](../../CONTRIBUTING.md#releasing).

The Worker change that serves the manifest from the bucket needs a deploy (`bun x wrangler deploy`).
