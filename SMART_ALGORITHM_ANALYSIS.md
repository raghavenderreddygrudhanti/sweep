# Sweep — Deep Analysis of the Smart Cleaning Algorithm

*Analysis only — no code was changed. Scope: `src/recommend_engine.rs`, `src/commands/recommend.rs`, `src/commands/smart.rs`, `src/scanner/mod.rs`, `src/whitelist.rs`, `src/oplog.rs`, branch `feat/ai-smart-scan`.*

---

## 1. How the algorithm works today

There are effectively **three parallel decision paths** in the codebase, and they don't share logic:

1. **`recommend_engine::score_item`** — the real engine. An additive integer score over a single path:
   - Classify the path by string-matching against `NEVER_TOUCH` / `REGENERABLE` lists → `Sensitive | UserOwned | Replaceable | Regenerable`.
   - `Sensitive` → hard `Keep` (−100). `UserOwned` → −60.
   - Size buckets: +35 (>5 GB) / +30 (>1 GB) / +20 (>500 MB) / +10 (>100 MB).
   - Access age: +25 (>180 d) / +20 (>90 d) / +10 (>30 d) / −40 (<7 d).
   - +20 if `Regenerable`; installer present +15 / absent −10; "gitignored" name +15.
   - Thresholds: **≥80 = SAFE CLEAN, ≥40 = REVIEW, else KEEP**.
2. **`commands/smart.rs` (rules path)** — a *separate*, simpler heuristic (size > 50 MB caches, installers always "safe", Downloads > 180 days, stale build dirs).
3. **`commands/smart.rs` (`--deep` AI path)** — SmolLM2-360M asked free-text "is this safe to delete?", parsed with `contains("safe")`, **display-only**.

The cleanest design instinct (a scored engine with classes and reasons) is in #1, but it is undermined by the issues below.

---

## 2. Correctness & safety findings (highest priority)

### 2.1 The whitelist is implemented but never enforced
`whitelist.rs` builds a sensible default-protected set (`~/Documents`, `~/.ssh`, Keychains, Mail, …) and exposes `is_protected()`. **Nothing in `recommend_engine` or `commands/recommend.rs` calls it.** Protection currently depends entirely on the in-engine `NEVER_TOUCH` string list, which is a *different* and narrower set. A user who adds a path to `~/.sweep/whitelist.txt` gets no protection in the scoring path. This is the single most important gap: a safety mechanism that looks active but isn't.

### 2.2 "Listed in .gitignore" never reads .gitignore
`is_in_gitignore()` only matches hardcoded directory names (`node_modules`, `target`, `build`, …). It awards +15 and the reason string *"Listed in .gitignore (regenerable)"* even when the folder is tracked by git. A folder literally named `build` or `dist` that contains committed source gets a deletion bonus on a false premise.

### 2.3 The "Freed: X" number is measured wrong
In both `recommend.rs` and `smart.rs`, the action loop deletes a child and *then* reads its size:
```
let ok = remove_dir_all(&p).is_ok();
if ok { freed += p.metadata().map(len).unwrap_or(0); }   // p no longer exists → 0
```
For directories (the common case), `metadata()` after deletion fails, so `freed` is badly under-reported. The pre-computed `size` is discarded. Users are told they reclaimed far less than they did.

### 2.4 Scoring granularity is too coarse to be "smart"
`score_item` scores whole top-level directories (`~/Library/Caches`, `~/Library/Developer`, `~/Downloads`) as a single unit, using the parent's aggregate size and the parent's own timestamp. But `~/Library/Caches` holds dozens of apps — some used daily, some abandoned years ago. Scoring the parent throws away all resolution: you can't keep the active app's cache while clearing the dead one. **Per-subdirectory scoring is the biggest single lever for making decisions genuinely intelligent.**

### 2.5 Age signal is built on an unreliable timestamp
`get_access_age_days` uses the directory's `accessed()` (atime), falling back to `modified()`. Two problems, both confirmed by the research:
- A **directory's** atime/mtime reflects metadata churn, not when the cached *data* was last useful.
- On macOS APFS, atime is **non-strict by default** (updated only past a threshold), and Apple's own timestamp APIs are documented as unreliable. The signal apps actually use for "Last Opened" is the extended attribute **`com.apple.lastuseddate#PS`**, which the code never reads. ([Eclectic Light Co.](https://eclecticlight.co/2025/10/24/be-careful-when-interpreting-apfs-timestamps/))

### 2.6 Classification is fragile string matching
`classify` uses `rel_path.contains(pattern)` with no path canonicalization. `.cache` as a substring will match anywhere in a path; symlinks and `..` aren't resolved; ordering between `NEVER_TOUCH` and `REGENERABLE` is implicit. This is workable for the curated source list but will misfire the moment scanning becomes more general (e.g., per-subdir or arbitrary `scan` paths).

### 2.7 `installer_app_exists` rarely matches
It does `app_name.contains(clean_name)` where `clean_name` still contains version numbers and spaces (`"Docker Desktop 4.2"`). Real app bundles (`Docker.app`) won't contain that string, so the check usually fails and installers get −10 ("might need installer") and are kept. Net effect: the feature under-cleans the exact files it was meant to catch.

---

## 3. Deeper algorithmic limitations

### 3.1 You are measuring logical size, not reclaimable space
`scanner::scan_size_native` sums `metadata().len()` over the tree. On APFS this is **wrong in two directions**:
- **Clones are counted at full size each.** `du`, Get Info, and a naive walk all report a cloned file as if it occupied its full length, so totals (and the "After clean: +X GB" prediction) are inflated. Apple gives third-party tools no API to detect clones. ([DaisyDisk on APFS](https://daisydiskapp.com/manual/4/en/Topics/APFS.html), [Eclectic Light Co.](https://eclecticlight.co/2020/04/09/where-did-all-that-free-space-go-on-my-apfs-disk/))
- **Snapshots pin deleted blocks.** Time Machine local snapshots can hold blocks of files you "deleted," so deleting them frees *nothing* until the snapshot is thinned. Old snapshots are frequently the real reason a disk is full. The "available" space macOS reports already includes **purgeable** space, so `available_space()` (used in the impact prediction) overstates what's truly free. ([Apple Community](https://discussions.apple.com/thread/255870214))

A cleaner that wants to claim "this will free X" has to reckon with clones, snapshots, and purgeable — otherwise its headline number is fiction on modern Macs.

### 3.2 The decision model is uncalibrated and binary
Hand-tuned integer additions with hard 80/40 cutoffs give no notion of **confidence** or **expected value**. Two moderate signals can cross 80; one strong risky signal can be outvoted. There's no ranking by "bytes reclaimed per unit risk," which is exactly the quantity a disk cleaner should optimize. The output is three buckets, not a ranked, explainable list.

### 3.3 No deduplication
There is no duplicate detection anywhere. For most users, duplicate large files (downloaded twice, copied between folders, exported renders) are among the safest and largest reclaimable wins, and they require no model — just a disciplined hashing pipeline (§4.4).

### 3.4 The three paths should be one
The good engine (`recommend_engine`) is rules-only; the AI lives in a separate command with its own weaker heuristics and brittle parsing; `smart` and `recommend` duplicate the "scan sources, bucket, delete" flow. Unifying them removes drift and lets the AI do the one thing it's actually suited for (§4.5).

---

## 4. Research-backed improvements

### 4.1 Measure the right quantity (foundation for everything)
- Separate **logical size** (what you show per item) from **estimated reclaimable** (what you promise to free). For the impact line, prefer the disk's **free** space, not `available` (which includes purgeable).
- Detect and surface **APFS snapshots** as their own reclaimable category (they're often the biggest win) rather than walking files. `tmutil`/snapshot inspection is the right source.
- Treat clone-heavy areas with caution in totals; at minimum, label predictions as estimates. This single change makes the tool *honest*, which for a destructive tool is a feature.

### 4.2 Score at the right granularity, with real signals
- Score **per app-subdirectory** inside `Library/Caches`, `Application Support`, `Library/Developer`, etc. — not the umbrella folder.
- Replace the directory-atime age signal with, in priority order: the **`com.apple.lastuseddate#PS`** xattr for user files, then mtime of the *newest descendant* for caches, with atime as a weak tiebreak only.
- Make **app-installed** robust: normalize installer names (strip version tokens/`vN.N`, separators), and match against a lowercased index of `.app` bundles in `/Applications` and `~/Applications` using token-overlap, not naive `contains`.
- Make **gitignore** real: find the enclosing repo (walk up to `.git`), then test the path against that repo's ignore rules (e.g., via the `ignore` crate, the same engine ripgrep uses) instead of a hardcoded name list.

### 4.3 Turn the score into a calibrated, rankable decision
- Convert additive points into a **probability-of-safe** in [0,1] (logistic over the same features), and rank items by **`p_safe × reclaimable_bytes`** — i.e., expected space freed weighted by safety. Present a ranked, explainable list; keep the three buckets only as display bands.
- Make the **whitelist a hard pre-filter** that runs *before* scoring and can never be overridden by score (fixes §2.1). The in-engine `NEVER_TOUCH` set and `whitelist.rs` should be merged into one source of truth.
- Expose **confidence** in the UI and in `--json`, so automation can gate on it.

### 4.4 Add a deduplication pipeline (high value, no model)
Use the standard, well-established tiered approach to keep it fast and correct:
1. **Bucket by exact byte size** — different sizes can't be duplicates.
2. **Partial fingerprint** for size-collisions — hash three 64 KB windows (first/middle/last); this filters out almost all non-duplicates cheaply.
3. **Full-content verification** only within surviving clusters before ever calling two files identical.

Use a **fast non-cryptographic hash (xxHash / xxh3)** for the partial stage and either a full xxh3 or BLAKE3 pass for verification — both are dramatically faster than SHA-256 and BLAKE3 parallelizes across cores. Always present duplicates as *review*, defaulting to keep the newest / shortest-path copy. ([xxHash vs BLAKE3](https://mojoauth.com/compare-hashing-algorithms/xxhash-vs-blake3))

### 4.5 Use the LLM only where it adds value, and constrain it
- The model should run **only on items the rules leave ambiguous** (the REVIEW band), never on the whole tree, and its verdict should be **review-only — it must never promote a user file to auto-delete**.
- Replace free-text `contains("safe")` (which also matches *"not safe"*) with **grammar-constrained JSON** via llama.cpp **GBNF**, forcing output like `{"verdict":"safe|review|keep","confidence":0..1,"reason":"..."}`. GBNF masks invalid tokens during sampling so the output is always parseable. Note the schema is **not** auto-injected into the prompt — you must also describe the expected fields in the prompt text. ([llama.cpp grammars](https://github.com/ggml-org/llama.cpp/blob/master/grammars/README.md), [DeepWiki: structured output](https://deepwiki.com/ggml-org/llama.cpp/8.1-grammar-and-structured-output))
- Feed the model the **derived features** (class, age source, app-installed, gitignore, size) rather than asking it to guess from a filename. A small model is a fine *arbiter over features*; it is a poor *oracle over filenames*.
- Reconcile the docs: the comment says SmolLM2-135M, the code loads 360M-Q8 (`smart.rs:255`).

### 4.6 Close the loop — learn from the user
You already have `oplog.rs` (a detailed audit trail) and `history.rs`. Extend this into a feedback signal: record every **accept / skip** per category, persist to `~/.sweep/`, and **down-weight categories the user repeatedly keeps** (and up-weight ones they always clear). This personalizes the probability in §4.3 with almost no new infrastructure and improves with use — far more impactful than swapping in a bigger model.

---

## 5. Suggested priority order

| # | Change | Type | Effort | Payoff |
|---|--------|------|--------|--------|
| 1 | Wire `whitelist::is_protected` as a hard pre-filter; merge with `NEVER_TOUCH` | Safety | Low | High |
| 2 | Fix `freed` measurement (use pre-scanned size) | Correctness | Low | High |
| 3 | Real gitignore (`ignore` crate) + robust app-installed match | Correctness | Med | Med |
| 4 | Per-subdirectory scoring + `lastuseddate#PS` xattr age | Signals | Med | High |
| 5 | APFS-aware reporting: free-vs-purgeable, surface snapshots | Accuracy | Med | High |
| 6 | Duplicate finder (size → partial → full, xxh3/BLAKE3) | Feature | Med | High |
| 7 | Probability + expected-reclaim ranking; confidence in output | Model | Med | Med |
| 8 | LLM as GBNF-constrained, review-only arbiter over features | AI | Med | Med |
| 9 | Feedback loop on top of `oplog` | Learning | Low | Compounding |

The throughline: **a bigger model is not the lever.** The wins come from measuring the right thing (reclaimable, not logical, size), scoring at the right granularity with trustworthy signals, enforcing the safety net you already wrote, and using the LLM narrowly and verifiably.

---

## Sources

- [Be careful when interpreting APFS timestamps — Eclectic Light Co.](https://eclecticlight.co/2025/10/24/be-careful-when-interpreting-apfs-timestamps/)
- [Where did all that free space go on my APFS disk? — Eclectic Light Co.](https://eclecticlight.co/2020/04/09/where-did-all-that-free-space-go-on-my-apfs-disk/)
- [A systematic approach to MACB timestamps on Unix-like systems — ScienceDirect](https://www.sciencedirect.com/science/article/pii/S2666281722000075)
- [DaisyDisk — APFS, clones and purgeable space](https://daisydiskapp.com/manual/4/en/Topics/APFS.html)
- [Whole disk marked as purgeable — Apple Community](https://discussions.apple.com/thread/255870214)
- [xxHash vs BLAKE3 — hashing algorithm comparison](https://mojoauth.com/compare-hashing-algorithms/xxhash-vs-blake3)
- [llama.cpp GBNF grammars README — ggml-org](https://github.com/ggml-org/llama.cpp/blob/master/grammars/README.md)
- [Grammar and Structured Output — llama.cpp DeepWiki](https://deepwiki.com/ggml-org/llama.cpp/8.1-grammar-and-structured-output)
