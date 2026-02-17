# Domain Selector Eval Framework

This project includes a JSON-driven eval harness for jargon profile selection in:

- `src/managers/domain_selector.rs` (`profile_selector_passes_eval_suite` test)
- `src/managers/domain_selector.rs` (`prompt_selector_passes_eval_suite` test)

Default eval file:

- `tests/domain_selector_profiles_evals.json`
- `tests/prompt_selector_evals.json`

## Running

From `src-tauri`:

```bash
cargo test profile_selector_passes_eval_suite -- --nocapture
cargo test prompt_selector_passes_eval_suite -- --nocapture
```

## Custom eval files

Set `SPITTLE_DOMAIN_SELECTOR_EVALS` to point to another JSON file:

```bash
SPITTLE_DOMAIN_SELECTOR_EVALS=/absolute/path/to/evals.json cargo test profile_selector_passes_eval_suite -- --nocapture
```

Set `SPITTLE_PROMPT_SELECTOR_EVALS` for prompt-routing evals:

```bash
SPITTLE_PROMPT_SELECTOR_EVALS=/absolute/path/to/prompt_evals.json cargo test prompt_selector_passes_eval_suite -- --nocapture
```

## JSON schema

```json
{
  "description": "optional label",
  "min_pass_rate": 0.8,
  "settings": {
    "top_k": 2,
    "min_score": 0.08,
    "timeout_ms": 80
  },
  "cases": [
    {
      "id": "unique_case_name",
      "input": "text to route",
      "expect_any_of": ["coding", "business"],
      "forbid": ["law_enforcement"],
      "expect_none": false
    }
  ]
}
```

Notes:

- `expect_none=true` means no profile should be selected.
- `expect_any_of` passes if any expected profile appears in selected output.
- `forbid` fails if any forbidden profile appears in selected output.

## Prompt selector eval schema

```json
{
  "description": "optional label",
  "min_pass_rate": 0.85,
  "settings": {
    "min_score": 0.08,
    "timeout_ms": 50,
    "hysteresis": 0.06
  },
  "cases": [
    {
      "id": "unique_case_name",
      "input": "text to route",
      "expect_prompt": "default_action_items",
      "fallback_prompt": "default_improve_transcriptions"
    }
  ]
}
```
