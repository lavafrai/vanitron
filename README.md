vanitron
=======
vanitron — vanity address generator for TRON
vanitron generates BIP‑39 mnemonics and searches for TRON addresses that match a given regex pattern. It works offline.

> **Important:**
> - Always verify that the generated mnemonic actually corresponds to the shown address by importing it into your wallet of choice.
> - Use at your own risk. Like any software, this project may contain bugs.

## Usage
```
vanitron [flags] <pattern>

 -j, --threads <N>        Number of worker threads (default: all CPU cores minus one)
 -c, --case-sensitive     Enable case‑sensitive matching (default: off)
 -m, --mnemonic-size <N>  Mnemonic size in words (12/15/18/21/24; default: 24)
 -p, --passphrase <STR>   BIP‑39 passphrase (optional)
     --backend <MODE>      Compute backend: cpu, gpu, or hybrid (default: hybrid)
     --adapter <GPU>       GPU adapter: auto, list index, or unique name (default: auto)
     --gpu-batch-size <N>  Mnemonics per GPU batch (default: 16384)
     --list-adapters       List GPU adapters and exit; no pattern is required

<pattern> — a regular expression applied to the address string
```

## Pattern format
- Full Rust regex. Without anchors ^/$ the match behaves like a substring search.
- Case‑insensitive by default; add `-c` to make it case‑sensitive.
- Examples:
```
$ vanitron "tron"            # address contains "tron" (case‑insensitive)
$ vanitron -c "^T[R0O]N"      # address starts with T followed by R/0/O (case‑sensitive)
$ vanitron "t[o0]n"          # address contains "ton" or "t0n"
```

## Safety
- Algorithms: BIP‑39, BIP‑32/44, secp256k1, Keccak256/SHA‑256, TRON Base58Check (prefix 0x41).
- No network requests or dynamic plugins. The app runs fully offline.

## “Are you cracking passwords?”
No. A 24‑word mnemonic from a 2048‑word list yields 2048^24 (~2.9×10^79) combinations. The chance of “guessing” someone else’s wallet is astronomically small.

## Speed
Vanity search is brute force. The longer and stricter the pattern, the slower it gets.
- Use more threads (`-j`), but not more than you have CPU cores.
- The normal build includes the cross-platform GPU PBKDF2 backend. Use `--no-default-features` for a smaller CPU-only binary.
- `--backend cpu` runs only independent CPU brute-force workers.
- `--backend gpu` uses GPU PBKDF2 and CPU post-processing, without a separate CPU brute-force lane.
- `--backend hybrid` runs the GPU PBKDF2 pipeline and independent CPU brute force simultaneously. It assigns roughly one quarter of the `--threads` budget to GPU post-processing and the rest to the independent CPU lane. GPU derivation of batch N+1 overlaps CPU checking of batch N, keeping both stages busy.
- `--adapter auto` benchmarks compatible GPUs and selects the fastest adapter.
- Use `--list-adapters` and `--adapter "GPU name"` (or its displayed index) to select one GPU manually. A manual selection still falls back to CPU if initialization or execution fails.
- On Apple M4 Pro, controlled local release/LTO runs with 24-word mnemonics and an impossible `^Z` pattern produced median end-to-end rates of ~14.2k addr/s on `--backend cpu`, ~28.5k addr/s on `--backend gpu`, and ~37.0k addr/s on `--backend hybrid`.
- With the default 13-thread budget, `--backend hybrid` keeps an independent CPU brute-force lane and the GPU PBKDF2 lane active at the same time. The displayed current rate uses a short rolling window, so completion of a 16,384-item GPU batch no longer appears as an artificial one-tick spike.
- GPU-produced matches are verified again through the CPU seed derivation path before they are printed. If GPU initialization, execution, post-processing, or verification fails, the affected batch is replayed on CPU and the run continues without using GPU.
- GPU acceleration currently supports an empty BIP‑39 passphrase. A non-empty `--passphrase` automatically uses CPU.
- Simplify your pattern / reduce the required prefix length / try to use [leet](https://en.wikipedia.org/wiki/Leet) to simplify the search.
- Avoid limiting yourself to a single case.

### Estimated time
For end‑anchored patterns like `...XYZ$` (matching the last k characters), the probability to match an exact k‑char suffix is `(1/58)^k` (case‑sensitive). For non‑anchored patterns (substring anywhere), we approximate the probability as `(L - k + 1) * (1/58)^k` where `L` is the address length. TRON Base58Check addresses are typically `L = 34` chars.

Case‑insensitive (NC) matching increases the chance for alphabetic characters: if your suffix consists only of letters, probability per letter doubles, i.e. `(2/58)^k = (1/29)^k`. If the suffix includes digits, adjust per position accordingly (digits unaffected).

Formulas (expected tries):
- Anchored, case‑sensitive (CS): `58^k`
- Non‑anchored, CS: `58^k / (L - k + 1)`
- Anchored, NC (letters‑only): `29^k`
- Non‑anchored, NC (letters‑only): `29^k / (L - k + 1)`

Time ≈ tries / your measured addr/s rate.

Examples (avg @ 39k addr/s; L=34):

| k | Anchored (CS) | Non‑anchored (CS) | Anchored (NC letters) | Non‑anchored (NC letters) |
|---|---|---|---|---|
| 1 | ~0.0015 s | ~0.000044 s | ~0.00074 s | ~0.000022 s |
| 2 | ~0.086 s | ~0.0026 s | ~0.022 s | ~0.00065 s |
| 3 | ~5.0 s | ~0.16 s | ~0.63 s | ~0.020 s |
| 4 | ~4m 50s | ~9.4 s | ~18 s | ~0.59 s |
| 5 | ~4h 40m | ~9m 21s | ~8m 46s | ~18 s |

Notes:
- NC columns assume all‑letter suffix; mix of letters/digits will be between CS and NC.
- Non‑anchored estimates use a rare‑event approximation; for small k the union bound may slightly over/under‑estimate.
- Real results vary with randomness and machine performance.

## TRON wallet details
- Coin type: 195 (SLIP‑44), derivation path: `m/44'/195'/0'/0/i`.
- Current build derives only index `i=0` during search.
- Mnemonic language: English (BIP‑39). A passphrase changes the seed and addresses — keep it consistent when importing.

## Importing the mnemonic
Import the mnemonic into a TRON‑compatible wallet (e.g., TronLink) using the TRON BIP‑44 path (HD-Wallet).

## Build
- Requires Rust (stable).
- Run: `cargo run --release -- "PATTERN"`
- Build: `cargo build --release`
- GPU-enabled build: `cargo build --release` (default)
- Hybrid run: `vanitron --backend hybrid "PATTERN"`
- CPU-only build without wgpu: `cargo build --release --no-default-features`

The GPU build uses Metal on macOS, Vulkan on Linux, and DX12 with statically linked DXC on Windows. It remains fully offline while searching; GPU discovery and execution do not make network requests.

## Donations
If this project is helpful, consider supporting further development by any convenient means on my website: [lavafrai.ru](https://lavafrai.ru)

## License
See LICENSE in the repository.
