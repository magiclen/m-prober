# M Prober Web UI

The web UI that `mprober web` serves. It is built with Vite, React and Mantine, and it reads the
HTTP APIs of the same `mprober` process that serves it.

## Build

```bash
pnpm install
pnpm build
```

The build writes `index.html`, `js/bundle.js` and `css/bundle.css` into `../front-end`, which the
Rust side embeds into the executable. The file names carry no content hash on purpose, because
`src/web/static_files.rs` lists every asset by name. **The build output is committed**, so that
`cargo install mprober` works on a machine without Node.

## Icons

`favicon.png` is the source image. The icons `../front-end` holds are made from it with
[favicon-generator](https://github.com/magiclen/favicon-generator), which is a separate step because
they only change when the logo does:

```bash
favicon-generator web-ui/favicon.png front-end \
    --app-name "M Prober" --app-short-name "M Prober" \
    --theme-color "#ffffff" -y
```

Run it from the root of the repository. It writes `favicon.ico`, `apple-touch-icon.png`,
`icon-192.png`, `icon-512.png`, `icon-mask.png` and `manifest.webmanifest`, and prints the tags for
the `<head>` of `index.html`. Adding or renaming one of them means updating the list in
`src/web/static_files.rs` as well.

## Develop

Start the probe in one terminal and the dev server in another:

```bash
cargo run -- web --addr 127.0.0.1
pnpm dev
```

`pnpm dev` proxies `/api` to `http://127.0.0.1:8000`, so the page talks to the real probe.

## Check

```bash
pnpm check   # oxlint, oxfmt and knip
pnpm test    # vitest
```
