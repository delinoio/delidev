# AI Agent Rules

When working in a specific directory, apply the rules from that directory and all parent directories up to the root.

## While working on `.`

*Source: `AGENTS.md`*

### Instructions

- Use the `@docs/` directory as the source of truth. You should list the files in the docs directory before starting any task, and update the documents as required. The `@docs/` directory should always be up-to-date.
- Write all comments in English.
- Prefer enum types over strings when all variants are known at the moment of writing the code.
- If you modified Rust code, run `cargo test` from the root directory before finishing your task.
- If you modified frontend code, run `pnpm test` from the frontend directory before finishing your task.
- Commit your work as frequent as possible using git. Do NOT use `--no-verify` flag.
- Do not guess; rather search for the web.
- Debug by logging. You should write enough logging code.

---

## While working on `apps/webapp`

*Source: `apps/webapp/AGENTS.md`*

### webapp

- This app is a next.js app, deployed to Vercel.
- This app exists primarily for marketing, but this app should also work as an proxy for AI calls, so that the end-users does not need to configure credentials for AI endpoints.
- The checkout page is available at `https://deli.dev/checkout`. This endpoint handles Polar.sh checkout redirects for license purchases.