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
 -m, --mnemonic-size <N>  Mnemonic size in words (12 or 24; default: 24)
 -p, --passphrase <STR>   BIP‑39 passphrase (optional)

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
- Simplify your pattern / reduce the required prefix length / try to use [leet](https://en.wikipedia.org/wiki/Leet) to simplify the search.
- Avoid limiting yourself to a single case.

### Estimated time at ~3k addr/s
For end‑anchored patterns like `...XYZ$` (matching the last k characters), the probability to match an exact k‑char suffix is `(1/58)^k` (case‑sensitive). For non‑anchored patterns (substring anywhere), we approximate the probability as `(L - k + 1) * (1/58)^k` where `L` is the address length. TRON Base58Check addresses are typically `L = 34` chars.

Case‑insensitive (NC) matching increases the chance for alphabetic characters: if your suffix consists only of letters, probability per letter doubles, i.e. `(2/58)^k = (1/29)^k`. If the suffix includes digits, adjust per position accordingly (digits unaffected).

Formulas (expected tries):
- Anchored, case‑sensitive (CS): `58^k`
- Non‑anchored, CS: `58^k / (L - k + 1)`
- Anchored, NC (letters‑only): `29^k`
- Non‑anchored, NC (letters‑only): `29^k / (L - k + 1)`

Time ≈ tries / 3000 seconds (assuming ~3,000 addr/s).

Examples (avg @ 3k addr/s; L=34):

| k | Anchored (CS) | Non‑anchored (CS) | Anchored (NC letters) | Non‑anchored (NC letters) |
|---|---|---|---|---|
| 1 | ~0.02 s | ~0.00057 s | ~0.0097 s | ~0.00028 s |
| 2 | ~1.12 s | ~0.034 s   | ~0.28 s    | ~0.0085 s  |
| 3 | ~1m 05s | ~2.03 s    | ~8.13 s    | ~0.254 s   |
| 4 | ~1h 03m | ~2m 02s    | ~3m 56s    | ~7.61 s    |
| 5 | ~2d 12h 46m | ~2h 01m 33s | ~1h 54m  | ~3m 48s    |

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

## Donations
If this project is helpful, consider supporting further development by any convenient means on my website: [lavafrai.ru](https://lavafrai.ru)

## License
See LICENSE in the repository.
