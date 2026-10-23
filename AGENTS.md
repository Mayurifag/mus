# Agent Notes

- Always use `make` targets. Do not run `cargo`, `vitest`, `docker compose`, or `npm` directly.
- Install yt-dlp from the official upstream nightly zipapp in all backend images, not distro packages; the app expects
  `yt-dlp --update-to nightly` to work.
- Compose frontend dependencies live in the `app_node_modules` volume, not host `frontend/node_modules`. Use
  `make front-reinstall` to rebuild that generated dependency tree when major upgrades conflict with old installs.
- Keep frontend TypeScript on 6.x until SvelteKit and typescript-eslint support 7.x; E2E can use 7.x independently.
- `docker/docker-compose.override.yml` is tracked with git-crypt. Unlock with the existing GPG key; a `GITCRYPT` header
  means locked, not corrupt. Use the example only if missing.
- Production deployments run the app behind external authentication; do not add app-level auth unless explicitly
  requested.
- App-code releases deploy automatically after successful CI on `main` via registry publishing and Watchtower. Treat
  pushes to `main` as production deployments.
- Before testing production changes, wait for both `Linters pipeline` and `Deploy to Container Registries` to succeed
  for the intended commit; then verify the running production revision and health. A successful Watchtower trigger alone
  does not prove deployment succeeded.
- Infrastructure changes belong in `~/Code/mayurifag.ru`; routine app-code releases do not require Ansible.
- App-owned state (`mus.db`, covers, HLS, transient upload/download/rewrite files) lives under `/app_data/.cache`.
- Production should mount `/app_data/.cache` as the single app-state volume; keep `/app_data/music` as the separate
  OpenCloud music mount.
- Never create staging or replacement files inside mounted music folders.
- Production OpenCloud uses PosixFS and stores metadata in `user.oc.*` xattrs.
- For OpenCloud-mounted music, update existing files in place to preserve inode/xattrs; if renaming too, copy bytes into
  the original path first, then rename that original file.
- If `mus` edits MP3 tags/artwork directly, OpenCloud sync can break with stale `user.oc.blobsize`/checksums or stuck
  `user.oc.nodestatus=processing:*`.
- Keep Markdown docs and roadmap notes current when changes affect project behavior or durable guidance.
