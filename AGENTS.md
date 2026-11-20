# AGENTS.md

Read and follow [CONTRIBUTING.md](CONTRIBUTING.md). It is the source of truth for development setup, verification commands, API and protocol conventions, live-test safety, changelog and release processes, and licensing. Keep shared contributor guidance there rather than duplicating it in this file.

## Coding-agent workflow

- Inspect the relevant code and official schemas before editing, and make the smallest focused change that satisfies the request.
- Run the applicable offline checks from `CONTRIBUTING.md` before finishing, and do not skip or weaken failing checks.
- Do not run production operations unless the user explicitly requests and separately charge-caps them.
- Never commit production credentials, `.env` files, personal report content, or other secrets.
- Do not publish releases or rewrite Git history. Commit only when explicitly requested.
- Update `CONTRIBUTING.md` when shared development guidance changes, and keep this file consistent with it.
