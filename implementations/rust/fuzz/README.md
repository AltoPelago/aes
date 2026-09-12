# Film fuzzing

The `film_decode` target sends arbitrary bytes through both the borrowed
physical decoder and the validated owned decoder. Neither result is assumed to
succeed; any panic, sanitizer finding, or process failure is a defect.

Seed the corpus from every canonical Film byte fixture in the mutable CTS:

```bash
node scripts/export-film-fuzz-corpus.mjs
```

Run the bounded local gate used for implementation milestones:

```bash
npm run check:fuzz:film
npm run fuzz:film
```

For continuing coverage-guided work, omit the libFuzzer run bound:

```bash
cargo +nightly fuzz run --fuzz-dir implementations/rust/fuzz film_decode implementations/rust/fuzz/corpus/film_decode
```

Generated corpus, artifacts, coverage output, and fuzz build products are
ignored. A reproducer that discovers a protocol defect should be minimized and
promoted into the language-neutral Film CTS rather than committed only as an
implementation-specific corpus file.
