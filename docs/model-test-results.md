# BabelTele offline encoder — multi-model comprehension tests

## Run 2 — default `-O tokens` mode (2026-09-14)

Prompt: docs/model-test-prompt-tokens.txt (memo 862→617 chars / 179→135 o200k tokens, log 608→345 / 208→113,
JSON 424→273 / 128→76; the same 12 factual questions). Invocation:
`copilot -s --no-color --model <id> --available-tools= --no-ask-user -p "$(cat docs/model-test-prompt-tokens.txt)"`

**Result: 300 / 300 — every model 12/12.**

| Model | Score | Time | Notes |
|---|---|---|---|
| claude-fable-5 | 12/12 | 10s | all correct (plain framing again; content filter blocked the original instruction sentence) |
| claude-fable-5.1 | 12/12 | 10s | all correct |
| claude-haiku-4.5 | 12/12 | 11s | all correct |
| claude-opus-4.7 | 12/12 | 8s | all correct |
| claude-opus-4.8 | 12/12 | 8s | all correct |
| claude-opus-5 | 12/12 | 8s | all correct |
| claude-sonnet-5 | 12/12 | 7s | all correct |
| gemini-3.5-flash | 12/12 | 14s | all correct |
| gemini-3.6-flash | 12/12 | 20s | all correct |
| gemini-3.7-flash | 12/12 | 14s | all correct |
| gemini-3.8-flash | 12/12 | 14s | all correct |
| gpt-5-mini | 12/12 | 13s | all correct |
| gpt-5.3-codex | 12/12 | 9s | all correct |
| gpt-5.4 | 12/12 | 8s | all correct |
| gpt-5.4-mini | 12/12 | 9s | all correct |
| gpt-5.5 | 12/12 | 7s | all correct |
| gpt-5.6-luna | 12/12 | 7s | all correct |
| gpt-5.6-sol | 12/12 | 7s | all correct |
| gpt-5.6-terra | 12/12 | 7s | all correct |
| gpt-6-astra | 12/12 | 9s | all correct |
| grok-4.5 | 12/12 | 8s | all correct |
| grok-4.6 | 12/12 | 10s | all correct |
| kimi-k2.7-code | 12/12 | 9s | all correct |
| kimi-k3 | 12/12 | 11s | all correct |
| mai-code-1.1-flash | 12/12 | 9s | all correct |

## LLM compression mode (`--llm copilot --model <m>`) — 2026-09-14

Inputs: examples/memo.txt, memo_pl.txt, app.log, service.json (2 727 chars, 760 o200k tokens).
Outputs of the two extreme compressors were then given to three decoder models
(gpt-5.4, claude-sonnet-5, gemini-3.6-flash) with the 12-question test.

| Compressor | chars kept | o200k tokens kept | wall time per call | decode fidelity |
|---|---|---|---|---|
| claude-haiku-4.5 | 680/2727 = 25% | 299/760 = 39% | 10–34 s | 27/36: Q4 lost "to the following week", Q5 lost "at least", Q10 wrong (09:15:50 line merged into 09:15:42) — identical for all 3 decoders |
| gemini-3.5-flash | 1108/2727 = 41% | 406/760 = 53% | 13–22 s | not tested |
| gpt-5-mini | 1258/2727 = 46% | 405/760 = 53% | 12–22 s | not tested; output contains invented absolute dates (2026‑09‑13, Thu 2026‑09‑17) |
| claude-opus-5 | 1190/2727 = 44% | 430/760 = 57% | 7–10 s | not tested |
| gpt-5.5 | 1337/2727 = 49% | 427/760 = 56% | 7–10 s | 36/36 (gemini-3.6-flash added a preamble line but all answers correct) |
| offline `-O tokens` L2 | 1860/2727 = 68% | 526/760 = 69% | <10 ms | 12/12 on all 25 models (above) |

Other observations: gpt-5.5 rewrote the Polish memo in English; claude-haiku-4.5 kept Polish but
inserted German "KRITISCH"; haiku's log compression dropped the INFO/DEBUG level words entirely.

## Run 1 — `-O chars` mode

Prompt: docs/model-test-prompt.txt (chars mode; 299/300) (3 compressed docs: memo 862→430 chars, log 608→273, JSON 424→226; 12 factual questions)

| Model | Score | Time | Notes |
|---|---|---|---|
| claude-fable-5.1 | 12/12 | 11s | all correct |
| claude-fable-5 | 12/12 | (plain framing) | all correct (needed plainer instruction wording: its content filter blocked the test framing sentence, and even the *plain-English* greeting "Hi team, here is a quick update…") |
| claude-haiku-4.5 | 12/12 | 15s | all correct |
| claude-opus-4.7 | 12/12 | 9s | all correct |
| claude-opus-4.8 | 12/12 | 9s | all correct |
| claude-opus-5 | 12/12 | 9s | all correct |
| claude-sonnet-5 | 12/12 | 8s | all correct |
| gemini-3.5-flash | 11/12 | 15s | Q10: "the circuit breaker opened for the gateway." |
| gemini-3.6-flash | 12/12 | 18s | all correct |
| gemini-3.7-flash | 12/12 | 17s | all correct |
| gemini-3.8-flash | 12/12 | 18s | all correct |
| gpt-5-mini | 12/12 | 16s | all correct |
| gpt-5.3-codex | 12/12 | 11s | all correct |
| gpt-5.4-mini | 12/12 | 11s | all correct |
| gpt-5.4 | 12/12 | 7s | all correct |
| gpt-5.5 | 12/12 | 7s | all correct |
| gpt-5.6-luna | 12/12 | 7s | all correct |
| gpt-5.6-sol | 12/12 | 7s | all correct |
| gpt-5.6-terra | 12/12 | 6s | all correct |
| gpt-6-astra | 12/12 | 9s | all correct |
| grok-4.5 | 12/12 | 8s | all correct |
| grok-4.6 | 12/12 | 15s | all correct |
| kimi-k2.7-code | 12/12 | 19s | all correct |
| kimi-k3 | 12/12 | 11s | all correct |
| mai-code-1.1-flash | 12/12 | 10s | all correct |
