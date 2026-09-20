# SOLX Launchpad Program

Open-source Solana program for the SOLX mobile-first token launchpad.

## Devnet Program

- Program ID: `2AVaQ45R5u1DE4JyvtQ76fUDm72CLSXMRfBQ8d3TPHjA`
- Network: Solana Devnet
- Framework: Anchor 1.2.0
- License: MIT

Tokens created by the launchpad are standard SPL Token mints. Bonding-curve
trading, market lifecycle, exact final-fill settlement, and graduation state
are implemented by this shared program.

## Reproducible Build

```bash
cargo build-sbf --arch v0 --manifest-path Cargo.toml
solana-verify get-executable-hash target/deploy/solx_launchpad.so
```

The source commit used for a deployment is submitted to the Solana verified
build registry with `solana-verify verify-from-repo`. The repository-level
manifest is intentional: it gives local and remote verifiers the same build
root while keeping the program modules under `programs/solx_launchpad`.

## Security

Read [SECURITY.md](SECURITY.md) before reporting a vulnerability. The deployed
program embeds the same policy and private-reporting link using
`solana-security-txt`.

Never use the Devnet deployment as production financial software. It has not
been audited.
