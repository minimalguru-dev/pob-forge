# CDN

A Cloudflare Worker in front of an R2 bucket. It serves what the app downloads instead of shipping in
every installer, and the updater manifest. Both fit Cloudflare's free tier.

| Path | What | Where it comes from |
|---|---|---|
| `tree/<sha256>.<ext>` | Passive tree art | `bun run sync`, then `bun run downloads upload` |
| `data/<sha256>` | PoE1 timeless jewel data | same |
| `maps/<game>/latest.json`, `<game>/Art/…?v=<tag>` | Item art | copied from `ORIGIN` on first request |
| `update/latest.json` | Updater manifest | latest GitHub release of `UPDATE_REPO` |

## Set up

1. Create an R2 bucket named `pob-forge` (or change `bucket_name` in `wrangler.toml`).
2. Deploy: `bunx wrangler login`, then `bunx wrangler deploy` in this folder. Note the
   `https://pob-forge-cdn.<account>.workers.dev` address it prints, or add a custom domain.
3. Upload the files `bun run sync` produced. Create an R2 API token with write access, then from the
   repository root:

   ```sh
   R2_ACCOUNT_ID=… R2_ACCESS_KEY_ID=… R2_SECRET_ACCESS_KEY=… R2_BUCKET=pob-forge bun run downloads upload
   POB_FORGE_CDN_URL=https://pob-forge-cdn.<account>.workers.dev bun run downloads check
   ```

4. In the GitHub repository, set the variable `POB_FORGE_CDN_URL` to the Worker's address. Release and
   check builds read it; a local `bun run tauri build` needs it in the environment too.

## Moving files from another server

Set `ORIGIN` in `wrangler.toml` to the server you are moving from and deploy again. Every file the
bucket lacks is then fetched there once, stored, and served from the bucket afterwards. Item-art maps
are refreshed from `ORIGIN` at most once an hour. Clear `ORIGIN` and deploy once the bucket holds
everything you need; from then on nothing is fetched from elsewhere.

## Updates

The Worker forwards `update/latest.json` from the latest GitHub release, cached for five minutes. The
release workflow lists it before the GitHub address in the app's updater endpoints, so either one
working is enough. The manifest and installers are signed with your updater key; see
[CONTRIBUTING.md](../../CONTRIBUTING.md#releasing).
