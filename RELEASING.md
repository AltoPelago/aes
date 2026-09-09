# Releasing

This repository publishes public source and reference material. It does not
currently publish an npm package or a crates.io crate.

## Non-publishing boundary

- the repository-root `package.json` has `private: true`
- the Rust `aes-telex` crate has `publish = false`
- the npm implementation package `@altopelago/aeon-aes` is released from the
  [`AltoPelago/aeon`](https://github.com/AltoPelago/aeon) workspace
- normative specifications and immutable CTS snapshots are released from their
  authority repositories, not from this repository

Do not add an npm or crates.io publish workflow until a separate package-release
decision names the artifact, version line, compatibility promise, and owner.

## Public repository release checklist

1. Ensure the corresponding canonical specifications are on
   `aeonite-org/aeonite-specs` main.
2. Ensure the claimed immutable snapshots are on `aeonite-org/aeonite-cts`
   main.
3. Run `npm run public:check` with `AEONITE_CTS_ROOT` pointing to that CTS
   checkout.
4. Confirm `git status --short` is empty.
5. Merge or fast-forward the reviewed release commit to `main`.
6. Confirm GitHub Actions passes on `main`.
7. Set the GitHub description, homepage, and repository topics.
8. Confirm the intended `main` branch protection and signed-commit policy.
9. Change the GitHub repository visibility from private to public.
10. Enable GitHub private vulnerability reporting before announcing the public
   repository.
11. Verify the README links and Actions status from a clean unauthenticated
    clone.

The repository can become public without creating an npm publication. If a
GitHub Release is later desired, use a signed annotated tag whose message makes
clear that it is a repository/reference release rather than an AES wire or
semantic version.
