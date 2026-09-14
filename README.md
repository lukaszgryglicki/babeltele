# babeltele

A standard UNIX pipe tool, written in Rust, that reads **whatever** you feed it on
stdin, autodetects what it is (prose in 19 languages, JSON, source code, Markdown,
CSV, logs, `key=value` blocks, chat transcripts, HTML) and writes it back out as
**BabelTele** — the dense, LLM-native notation described in

> Jiayi Zhu, Haoxuan Peng, Junxi Wang, Liang Ke, Chen Zhang, Linfeng Zhang (SJTU),
> *"Large Language Models Do Not Always Need Readable Language"*, arXiv:2606.19857 (June 2026).

```
$ echo "Q3 revenue is projected to rise ~30% YoY; if it lands, the Berlin team ships the mobile app in October." | babeltele
Q3 rev proj rise ~30% YoY; ? lands ⇒ Berlin team ships mobile app @ Oct
$ echo "Q3 revenue is projected to rise ~30% YoY; if it lands, the Berlin team ships the mobile app in October." | babeltele -O chars
Q3rev proj📈~30%YoY; ?lands⇒Berlin👥🚀📱app@Oct
```

(The paper's own example for this sentence is `Q3rev proj📈~30%YoY; ?lands⇒Berlin👥🚀📱@Oct`.)

The two outputs show the tool's two optimisation targets: the default minimises **LLM tokens**
(what you pay for and what fits in a context window — a symbol is only used when it is not more
expensive than the words it replaces), `-O chars` minimises **characters** and looks like the
paper's examples. See [How much does it save?](#how-much-does-it-save) for the honest numbers.

It works **fully offline** — a compiled multilingual lexicon (~10k concepts and phrases
distilled from several LLMs), a precomputed BPE token-cost table for every lexicon string, and a
small symbolic grammar. No network, no API key, no model download. An optional `--llm` mode
delegates the compression to a real model if you want the "authentic" thing.

---

## Is the viral claim true?

Mostly **yes** — the numbers come straight from the paper — but the wording is hyped:

| Claim in the post | Paper | Verdict |
|---|---|---|
| "27.9% of the length, 99.5% of the meaning" | Table 1: compressed text averages 27.9% of the original length (measured in **tokens**, Appendix B.5); an LLM judge scores semantic fidelity at 99.5% | ✅ accurate |
| "Universal cross-model transfer, no fine-tuning" | Text compressed by one model family is decoded by other families with ~no loss | ✅ accurate |
| "Agent memory overhead cut ~50%, beats summarization" | Agent-memory experiment: ≈50% fewer context tokens while outperforming summarization baselines | ✅ accurate |
| "40% faster multi-agent systems, zero loss" | 39–44% fewer **communication tokens** with 96.6–99.7% task-score retention | ⚠ *tokens*, not wall-clock speed; loss is 0.3–3.4%, not zero |
| "LLMs **spontaneously** collapsed natural text" | Models were **prompted** (zero-shot) to compress for an LLM audience | ❌ not spontaneous |
| "A secret language only they understand" | It is a family of model-produced representations that humans can partially read, not a fixed protocol or code | ❌ hype |

So: real paper, real results, breathless framing. "BabelTele" is a *style*, not a standard —
which is exactly why a deterministic offline encoder like this one can produce it and every
current LLM can read it (see the test results below).

## Install

```sh
cargo build --release          # → target/release/babeltele  (single binary, ~9.5 MB)
cargo install --path .         # optional: put it on your PATH
```

A prebuilt, fully static, stripped **FreeBSD amd64** binary is committed as
[`bin/babeltele-freebsd-amd64`](bin/babeltele-freebsd-amd64) (built with
`RUSTFLAGS="-C target-feature=+crt-static" cargo build --release --target x86_64-unknown-freebsd`);
it has no runtime dependencies (built on FreeBSD 15.1-RELEASE amd64):

```sh
install -m 755 bin/babeltele-freebsd-amd64 /usr/local/bin/babeltele
```

Requires Rust 1.82+. Dependencies: `clap`, `regex`, `serde_json`, `whatlang`, `tiktoken-rs`
(embedded `o200k_base` vocabulary, used only to print real token counts with `-s`).

## Usage

```
babeltele [OPTIONS] [FILES]...          # no files → stdin

  -l, --level <1|2|3>   1 = light (keeps articles & punctuation)
                        2 = BabelTele (default)
                        3 = aggressive (also drops clause commas and omissible relative
                            pronouns; with -O chars additionally disemvowels long words)
  -O, --optimize <T>    tokens (default) | chars     — what to minimise, see below
  -k, --kind <KIND>     auto | prose | json | code | markdown | csv | log | kv | dialogue | html
  -L, --lang <LANG>     auto | en pl de fr es it pt nl ru uk cs sk sv da no fi tr hu ro
  -s, --stats           detection result + character AND token (o200k_base) statistics on stderr
      --legend          symbol legend for humans
      --llm <BACKEND>   copilot | api   (opt-in; see --help-llm)
      --model <MODEL>   model for --llm
```

Everything on stdin is fine — a paragraph, a whole mailbox export, `git log`, a JSON blob,
a Python file, a chat transcript:

```sh
cat memo.txt        | babeltele -s
pbpaste             | babeltele -l 3
kubectl logs api    | babeltele
curl -s api/health  | babeltele
git log -20         | babeltele
babeltele README.md notes.md > notes.bt
```

### `-O tokens` vs `-O chars`

LLM tokenizers (o200k_base for GPT-4o/5, cl100k_base for GPT-4; other modern BPE vocabularies
behave similarly) price text very differently from `wc -c`:

* a common English word is **1 token** with or without its leading space (`" because"` = 1);
* most logic symbols are **2 tokens**, emoji **2–4**, a flag emoji **4**, CJK characters 1–3
  depending on the vocabulary (`∵` = 2, `🇩🇪` = 4, `問題` = 1 in o200k but 3 in cl100k) — so
  `because→∵` *doubles* the cost while saving 6 characters;
* gluing a symbol to a word splits the word (`@nightly` = 3, `@ nightly` = 2), but a space before
  **digits** costs an extra token (`~30%` = 3, `~ 30%` = 4);
* o200k knows Polish, German, French… well (`" ponieważ"` = 1 token), so cross-lingual
  substitution rarely pays for those languages either.

Therefore every one of the ~30k lexicon strings was tokenized with both vocabularies at build
time (`src/tokcost.rs`). In the default **`-O tokens`** mode a substitution is applied only if it
costs no more tokens than the words it replaces (ties allowed for ASCII and for universal
1-token symbols such as `→ ≤ ~ & @`), operators are spaced off words and glued to digits, and
hard-coded symbol paths (`¬`, `∅`, `∃`, log-level emoji, JSON literals, disemvoweling) fall back
to the cheaper plain word. **`-O chars`** is the maximal symbolic rendering from the paper's
examples: half the characters, but often *more* tokens than the token-optimised output.

### What it does per input kind

Symbolic renderings below are what you get with `-O chars`; in the default token mode a word is
only replaced when the replacement is not more expensive (`because` stays, `and`→`&`, `at least`→`≥`,
`Thursday`→`Thu`, `October`→`Oct`, `so far`→`已`, `have to`→`必`).

| Kind | Transformation |
|---|---|
| **prose** | drops articles/fillers/hedges, maps ~10k words & phrases to CJK/emoji/symbols (`because`→`∵`, `team`→`👥`, `yesterday`→`昨日`), number words → digits with units (`three million users`→`3M👤`, `from 5 to 10 minutes`→`5–10min`, `thirty percent`→`30%`), conditionals (`If X, then Y`→`?X⇒Y`, `unless`→`⇐¬`), negation `¬`, existence `∃/∄`, copula `=` before quantities, months/weekdays/countries → `Oct`/`Thu`/`🇩🇪`, sentence `.`→`;`, URLs/e-mails/paths/`code`/identifiers/`host:port`/`key=value` preserved verbatim |
| **json** | `{k:v}` without quotes where safe, `true/false/null`→`✅/❌/∅`, prose strings compressed, arrays of same-shape objects → `[k1,k2]:(v1,v2)(v1,v2)` |
| **code** | identifiers & string literals untouched; comments compressed as prose; blank lines gone; indentation shrunk; operators → `→ ⇒ ≠ ≤ ≥ ∧ ∨ ≡`; `null/true/false`→`∅/✓/✗` |
| **markdown** | headings `##x`, lists `•`, tasks `☐/☑`, quotes `❝`, links `text→url`, tables keep `|`, fences handled as code |
| **csv** | `[header]` then `(row)` tuples, booleans/nulls → symbols, long cells compressed, `Poland`→`🇵🇱` |
| **log** | date once per day, `FATAL/ERROR/WARN/INFO/DEBUG/TRACE` → `💀❌⚠ℹ🐛👣`, `key=value` kept, consecutive duplicates → `×N` |
| **kv** | `KEY:value`, booleans → `✅/❌`, comments compressed |
| **dialogue** | `User/Assistant/System` → `U/A/S` (others → initials), utterances compressed |
| **html** | tags stripped, entities decoded, text compressed as prose |

Language is autodetected with `whatlang` + a stopword vote (so 3-word inputs and mixed
Slavic/Romance pairs still resolve). Dedicated lexicons exist for **en, pl, de, fr, es, it, pt,
nl, ru, uk, cs**; the other supported languages get the universal concept table and filler removal.

### Examples

```
$ babeltele -s memo.txt
babeltele: kind=prose lang=en (1.00) level=2 optimize=tokens mode=offline chars 862 → 617 (71.6%) tokens 179 → 135 (75.4%)
Hi team, here quick update @ Atlas migration proj
@ yesterday we moved ~80% customer data new DB cluster @ Frankfurt; remaining twenty percent = mostly archived records before 2015, which we plan migrate over weekend
one significant problem: nightly backup job takes ~ six hours vs expected two, because compression step runs @ single thread; ? we cannot fix this ≤ Thu ⇒ we will必 postpone cutover following week; Marta looking multi-threaded impl, she believes it could reduce time ≥60%
pls not deploy anything prod until backup issue resolved; budget for whole proj =€45,000 & we spent 31,000已, so we still fine
Tom

$ babeltele -s -O chars memo.txt
babeltele: kind=prose lang=en (1.00) level=2 optimize=chars mode=offline chars 862 → 430 (49.9%) tokens 179 → 152 (84.9%)
Hi👥,here速update@Atlas migration proj
@昨日we移~80%客data新DB cluster@Frankfurt; remaining 20%=mostly archived records前2015,which we計画migrate over weekend
∃one❗問題: nightly backup job takes ~6h vs expected 2,∵compression step runs@single thread; ?we ¬可🔧此≤Thu⇒we将必postpone cutover following週; Marta looking multi-threaded impl,she believes it可📉時≥60%
pls ¬🚀anything prod until backup issue✅;予算4∀proj=€45,000&we spent 31,000已,∴we still✅
Tom
```

Note the last line of each stats header: the symbolic version is 30% shorter in characters but
costs **17 more tokens** than the token-optimised one.

```
$ babeltele -s app.log
babeltele: kind=log lang=en (1.00) level=2 optimize=tokens mode=offline chars 608 → 345 (56.7%) tokens 208 → 113 (54.3%)
03-11 09:14:02 INFO server started @ port 8080
09:14:03 INFO connected DB @ db-prod-1:5432
09:15:41 WARN req took longer than expected: 2350ms path=/pay user=4412
09:15:42 ERROR failed reach bank gateway: connection timed out after 3000ms ×3
09:15:50 INFO circuit breaker opened for gateway, retrying @30s

$ babeltele -O chars app.log | head -3
03-11 09:14:02ℹ server始@port 8080
09:14:03ℹ connected DB@db-prod-1:5432
09:15:41⚠ req took longer than expected: 2350ms path=/pay user=4412
```

```
$ babeltele -s service.json
babeltele: kind=json lang=en (1.00) level=2 optimize=tokens mode=offline chars 424 → 273 (64.4%) tokens 128 → 76 (59.4%)
service:payments-api
version:2.14.0
healthy:true
replicas:3
owner:null
description:"This service handles all incoming payment requests mobile app & forwards them bank gateway"
endpoints:[path,method,auth]:(/pay,POST,true)(/status,GET,false)
limits:{rps:500,timeout_ms:3000}
```

```
$ echo "Der Umsatz wird im dritten Quartal voraussichtlich um etwa dreißig Prozent steigen." | babeltele
rev im 3. Q ~30% steigen
$ echo "Der Umsatz wird im dritten Quartal voraussichtlich um etwa dreißig Prozent steigen." | babeltele -O chars
rev∈3.Q ~30%📈
$ echo "Zespół nie może naprawić tego problemu, ponieważ baza danych jest niedostępna." | babeltele
👥 nie może 🔧 tego problemu, ponieważ DB ∄
$ echo "Zespół nie może naprawić tego problemu, ponieważ baza danych jest niedostępna." | babeltele -O chars
👥¬可🔧此問題,∵DB ∄
```

## How much does it save?

**Typical token compression gain** — the question that matters for prompts and agent memory:

| Source | What was measured | Tokens kept | Saving |
|---|---|---|---|
| Paper, document compression (Table 1) | LLM-produced BabelTele, length in tokens | **27.9%** average (on LongBench individual models keep 5–25%) | 72% |
| Paper, LoCoMo agent memory | 2 819.5 → 1 382.2 tokens per memory, 96.5% score retention | 49% | 51% |
| Paper, multi-agent communication | communication tokens | 56–61% | 39–44% |
| **This tool, offline, default** (`-O tokens`, level 2) | 10 example files, 1 337 o200k tokens → 966 | **72%** | **28%** |
| This tool, offline, `-O chars`, level 2 | same files, 5 240 chars → 2 791 (53%) but tokens 1 337 → 1 067 | 80% | 20% |
| This tool, `--llm copilot` | 4 files, 760 tokens; depends on the model, see below | 39–57% | 43–61% |

Per input kind, offline, level 2 (`scripts/measure_tokens.py`; o200k_base ≈ cl100k_base within 1–5 pp):

| Example | `-O tokens` chars | **`-O tokens` tokens** | `-O chars` chars | `-O chars` tokens |
|---|---|---|---|---|
| memo.txt (English prose) | 72% | **76%** | 50% | 85% |
| memo_pl.txt (Polish prose) | 75% | **83%** | 61% | 88% |
| chat.txt (dialogue) | 71% | **86%** | 51% | 98% |
| release-notes.md | 73% | **80%** | 52% | 90% |
| config_loader.py (code) | 73% | **86%** | 68% | 91% |
| service.env (key=value) | 73% | **83%** | 57% | 96% |
| customers.csv | 91% | **95%** | 66% | 120% ⚠ |
| service.json | 65% | **59%** | 54% | 69% |
| app.log | 57% | **54%** | 45% | 57% |
| status.html | 36% | **29%** | 28% | 30% |
| **all ten** | 69% | **72%** | 53% | 80% |

Reading the table: the deterministic encoder removes **15–25% of the tokens of English prose
(24% on the memo, 14% on the chat), ≈ 17% of Polish, and 40–70% of structured text** (logs,
JSON, HTML — where the saving comes from dropping syntax, not from symbols). Levels: 1 keeps ≈ 81%, 2 ≈ 72%,
3 ≈ 71–72% (level 3's extra deletions are worth 1–2 pp in tokens; its disemvoweling only runs
in `-O chars`, because `cmprssn` costs more tokens than `compression`). CSV barely shrinks because
it is already dense, and `-O chars` makes it *grow* in tokens (flag emoji for countries).
The paper's 3.6× comes from a model rewriting the text; an offline lexicon cannot invent
sentence-level collapses — use `--llm` for that, at the price of latency and fidelity (below).

## Do LLMs actually understand the output?

Yes. Three encoded documents (the memo, the log and the JSON above) were sent, with 12 factual
questions, to **every model reachable through the GitHub Copilot CLI**, from the smallest to the
largest — once encoded with the default `-O tokens` (1 894 chars → 1 235, 515 tokens → 324) and
once with `-O chars` (1 894 → 929 chars, 515 → 357 tokens). Scored against ground truth:

**Default token mode: 300 / 300 answers correct across all 25 models** (every model 12/12;
timings 7–20 s per call; `docs/model-test-prompt-tokens.txt`, results in `docs/model-test-results.md`).

Symbolic `-O chars` mode: **299 / 300**:

| Model | Score | | Model | Score |
|---|---|---|---|---|
| claude-haiku-4.5 | 12/12 | | gpt-5-mini | 12/12 |
| claude-sonnet-5 | 12/12 | | gpt-5.4-mini | 12/12 |
| claude-opus-4.7 | 12/12 | | gpt-5.3-codex | 12/12 |
| claude-opus-4.8 | 12/12 | | gpt-5.4 | 12/12 |
| claude-opus-5 | 12/12 | | gpt-5.5 | 12/12 |
| claude-fable-5 | 12/12 ¹ | | gpt-5.6-sol / terra / luna | 12/12 each |
| claude-fable-5.1 | 12/12 | | gpt-6-astra | 12/12 |
| gemini-3.5-flash | 11/12 ² | | grok-4.5 / grok-4.6 | 12/12 each |
| gemini-3.6 / 3.7 / 3.8-flash | 12/12 each | | kimi-k3 / kimi-k2.7-code | 12/12 each |
| mai-code-1.1-flash | 12/12 | | | |

¹ needed a plainer instruction sentence (in both runs) — that model's content filter blocked the
test framing (and also blocked the *plain-English* greeting "Hi team, here is a quick update…",
so unrelated to the encoding). ² omitted the "retrying in 30 s" detail of one log line in chars
mode; correct in token mode.

A Polish-source memo encoded to 61% (chars mode) was likewise decoded correctly by
`claude-haiku-4.5`, `gpt-5-mini`, `gemini-3.5-flash` and `kimi-k2.7-code`.

Reproduce: the prompts are `docs/model-test-prompt-tokens.txt` and `docs/model-test-prompt.txt`; run
`copilot -s --no-color --model <id> --available-tools= --no-ask-user -p "$(cat docs/model-test-prompt-tokens.txt)"`.

## Optional LLM mode

The default path never touches the network. If you want a real model to do the compression
(with the exact prompt from the paper's Appendix C.1):

```sh
echo "..." | babeltele --llm copilot --model claude-haiku-4.5     # local GitHub Copilot CLI
export BABELTELE_API_KEY=sk-...                                    # or OPENAI_API_KEY
echo "..." | babeltele --llm api --model gpt-4o-mini               # any OpenAI-compatible endpoint
BABELTELE_API_URL=http://localhost:11434/v1/chat/completions babeltele --llm api --model llama3 < in.txt
```

The API key is handed to `curl` through `-K -` on stdin (never on the command line) and the
request body goes through a `0600` temp file. `babeltele --help-llm` prints the details.

Measured through the Copilot CLI on four example files (memo, Polish memo, log, JSON:
2 727 chars, 760 o200k tokens), then decoded by three *other* models with the 12-question test:

| Compressor model | Chars kept | Tokens kept | Wall time per call | Decode fidelity (3 decoders × 12 Q) |
|---|---|---|---|---|
| claude-haiku-4.5 | 25% | **39%** | 10–34 s | 27/36 — dropped "to the following week" and "at least", merged two log lines |
| gemini-3.5-flash | 41% | 53% | 13–22 s | – |
| gpt-5-mini | 46% | 53% | 12–22 s | – (invented absolute dates for "yesterday"/"Thursday") |
| claude-opus-5 | 44% | 57% | 7–10 s | – |
| gpt-5.5 | 49% | 56% | 7–10 s | **36/36** |
| *offline `-O tokens`, for comparison* | 68% | 69% | < 0.01 s | 12/12 by every one of the 25 models |

So the LLM mode reaches the paper's ballpark (≈ 40–60% of the tokens, 25–50% of the characters)
at the price of one extra model round-trip per document (7–34 s here); and the more aggressively
a model compresses, the more small facts it drops or paraphrases — the 0.3–3.4% loss the paper
reports, occasionally worse (invented dates, a Polish memo rewritten in English or with a German
`KRITISCH`). The offline encoder is deterministic, keeps every number, name and identifier, and
is instant.

## Symbol legend (for humans)

`babeltele --legend` prints the full table. Most of the CJK/emoji entries only appear with
`-O chars` (or, in token mode, where a multi-word phrase collapses into one cheap symbol). The
most frequent ones:

```
⇒ then/implies    ⇐ if (trailing)   ? if (leading)   ∵ because   ∴ therefore   ¬ not   ∅ none/null
∧ and   ∨ or   ≠   ≥ at least   ≤ at most / by (deadline)   ~ approximately   → to/into/becomes
@ at/in (time & place)   ∈ in/member of   ∀ every/all   ∃ there is   ∄ there is no   Δ change   Σ total
✅ yes/true/done/resolved   ❌ no/false/failed   ⚠ warning   💀 fatal   ℹ info   🐛 debug/bug
📈 rise   📉 fall   🚀 launch/ship/deploy   🔧 fix   👥 team   👤 user/person   💰 money   🏦 bank
将 will (future)   已 already   必 must   可 can   昨日 yesterday   今日 today   明日 tomorrow   週 week   月 month
k/M/B thousand/million/billion   YoY   Q1–Q4   h/min/s/d/w/mo/y   ×N repeated N times   🇺🇸🇩🇪🇵🇱 countries
```

## Design notes & limitations

* **Deterministic, not generative.** The encoder is a lexicon + grammar, so it is fast
  (microseconds per line), reproducible and offline — but it will never invent the clever
  ad-hoc collapses an LLM does. Expect ~1.4× fewer tokens (~2× fewer characters with `-O chars`),
  not the paper's ~3.6×; use `--llm` for that.
* **Lossy by design at level 2.** Articles, hedges, discourse fillers, auxiliaries and
  sign-offs are dropped; tense is reduced to `将`/`已` (chars mode); intensifiers ("very")
  disappear. Level 1 keeps them; level 3 additionally drops commas before connectors, omissible
  relative pronouns (`the bug which we fixed` → `bug we fixed`) and, in chars mode, vowels of
  long unknown words.
* **Never expands.** A word is only replaced when the replacement is not longer in characters
  (both modes) and not more expensive in o200k/cl100k tokens (token mode) — enforced at lexicon
  load time and by tests against the generated cost table.
* **Protected spans** (URLs, e-mails, paths, `code`, `@handles`, `#tags`, `$VARS`, `--flags`,
  `snake_case`, `host:port`, `key=value`, filenames, version numbers) pass through untouched.
* Proper-noun heuristics are capitalization-based for English/German; ambiguous names
  (`May`, `March`, `Turkey`…) are only mapped when the context makes them a date/country.
* `-s` reports characters **and** real o200k_base tokens (via `tiktoken-rs`); the token-mode
  filter also checks cl100k_base. Other vendors' tokenizers were not measured here — they are
  BPE vocabularies with the same general shape (spaces absorbed into words, rare symbols and
  emoji split into several tokens), so the direction of the savings should carry over, the exact
  percentages will differ.
* The two modes are different trade-offs, not "better/worse": `-O chars` is for humans who want
  the paper's look or for byte-limited channels; `-O tokens` is for prompts, memory and
  agent-to-agent messages.

## Development

```sh
cargo test --release       # 21 unit tests + 10 CLI integration tests
cargo run --release -- -s < some_file
```

Layout: `src/prose.rs` (tokenizer + encoder), `src/lexicon.rs` + `src/lexdata*.rs`
(concept tables), `src/tokcost.rs` (generated BPE cost table), `src/detect.rs` (kind & language
detection), `src/numwords.rs` (numbers/units/currencies), `src/json.rs`, `src/formats.rs`
(code/markdown/csv/log/kv/dialogue/html), `src/llm.rs` (optional back-ends), `src/main.rs` (CLI),
`tests/cli.rs`.

After editing any `src/lexdata*.rs` table, regenerate the cost table (a unit test fails
otherwise). This needs Python with `tiktoken` (`python3 -m venv .venv && .venv/bin/pip install tiktoken`):

```sh
cargo build --release
target/release/babeltele --dump-lexicon | .venv/bin/python scripts/gen_tokcost.py > src/tokcost.rs
cargo test --release
.venv/bin/python scripts/measure_tokens.py --levels 1,2,3                    # token/char table above
.venv/bin/python scripts/measure_tokens.py --levels 2 --extra-args "-O chars"
```

## License

MIT
