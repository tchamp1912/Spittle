# ChatGPT Jargon Pack Guide

This guide explains how to create custom jargon packs for Spittle using ChatGPT and import them in the Jargon Packs settings pane.

## What a jargon pack contains

Each pack has:

- `id`: stable machine-friendly identifier
- `label`: display name in the UI
- `terms`: canonical words/phrases to preserve
- `corrections`: spoken phrase to canonical replacement mapping

## JSON format

Import JSON must match this structure:

```json
{
  "version": 1,
  "packs": [
    {
      "id": "example_domain",
      "label": "Example Domain",
      "terms": ["Canonical Term", "Another Term"],
      "corrections": [
        { "from": "spoken term", "to": "Canonical Term" }
      ]
    }
  ]
}
```

## Prompt template for ChatGPT

Copy this prompt and customize the domain:

```text
Generate a JSON jargon pack for Spittle.

Requirements:
- Output valid JSON only, no markdown.
- Use this exact schema:
  {
    "version": 1,
    "packs": [
      {
        "id": "snake_case_id",
        "label": "Human Label",
        "terms": ["..."],
        "corrections": [{"from":"spoken phrase","to":"CanonicalTerm"}]
      }
    ]
  }
- Include 40-80 high-value terms.
- Include 20-40 correction pairs for common speech-to-text mistakes.
- Keep corrections directional: from spoken/misheard -> canonical.
- Avoid duplicates (case-insensitive).
- Keep terms concise and practical for dictation workflows.

Domain:
<PUT DOMAIN HERE>
```

## Quality checklist before import

- JSON parses with no syntax errors.
- `id` and `label` are non-empty.
- `terms` have no empty strings.
- `corrections` have non-empty `from` and `to`.
- No duplicate pack IDs in one file.
- Canonical spellings in `to` match entries in `terms` when relevant.

## Import steps

1. Open `Settings -> Jargon Packs`.
2. Click `Import JSON`.
3. Choose your generated JSON file.
4. Verify pack counts and summary values.
5. Enable profiles in `Settings -> Jargon` as needed.

## Export steps

1. Open `Settings -> Jargon Packs`.
2. Optionally select specific packs.
3. Click `Export JSON`.
4. Save the output file for sharing or version control.

## Notes

- Domain selector is fail-open: if sidecar selection fails, Spittle continues using manual profiles.
- Invalid/empty pack entries are ignored during import.
