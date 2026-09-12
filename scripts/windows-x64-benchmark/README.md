# AES native Windows x64 benchmark bundle

This bundle runs the native Rust scalar, document-pipeline, and
within-document Telex pipeline benchmarks without installing Rust, Git, Node,
Docker, or the AES repository.

## Before running

1. Extract the complete artifact ZIP to a local folder.
2. Connect the machine to AC power and select Windows **Best performance** mode.
3. Close development tools, browsers, games, synchronization jobs, and other
   CPU-heavy applications.
4. Let the machine sit idle for a minute so background startup work settles.

Do not run the executable directly from inside the downloaded ZIP.

## Run

Open PowerShell in the extracted folder and run:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\run-benchmarks.ps1
```

The full run uses 15 measured iterations for the within-document matrix. For a
short functional check, use:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\run-benchmarks.ps1 -IncrementalIterations 3
```

The script verifies the executable SHA-256 hashes before running, rejects
non-x64 execution, records the processor and Windows version, and creates both
a results directory and a shareable `results-*.zip` archive.

Run the full benchmark twice. If corresponding medians differ by more than
about 10%, wait for the machine to become idle and run it once more. Return the
most stable two result archives rather than choosing only the fastest run.
