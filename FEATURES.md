# Features

This file tracks implemented and planned product features, with a working prompt for each item.

## Status Legend

- `Completed`: Implemented in the codebase.
- `Planned`: Not implemented yet.

## Feature List

| ID  | Feature                                                     | Completion Status | Prompt                                                                                                                                                                                                                        |
| --- | ----------------------------------------------------------- | ----------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| F1  | Rebrand app from Spittle to Spittle                           | Completed         | "Update all product branding from Spittle to Spittle across app name, UI strings, docs, package metadata, bundle identifiers, and release/config references. Keep behavior unchanged except branding."                          |
| F2  | `@file` expansion in workspace context                      | Completed         | "When transcription contains `@filename` or `@\"file with spaces\"`, resolve it against the active workspace and append a code snippet for uniquely matched files. Add safety limits for snippet size and skip binary files." |
| F3  | Initial prompt jargon support + post-processing replacement | Completed         | "Inject active jargon terms into the post-processing prompt so technical spellings are preserved, then run post-processing and use the refined output as the final pasted text when available."                               |
| F4  | New settings for added features                             | Completed         | "Add settings toggles and persistence for newly added functionality (including `@file` expansion and cleanup/jargon controls), wire them through backend commands, generated bindings, and settings UI."                      |
| F5  | Template command mode (PR/commit/plan/prompt templates)     | Planned           | "Add a `template` command mode: when first spoken token is `template`, open template flow for PRs/commits/plans/prompts, insert selected template, and support hotkeys to jump between placeholders."                         |
| F6  | CLI mode post-processing                                    | Planned           | "Add CLI mode that transforms speech/text into shell commands using safe post-processing, supports common CLI shortcuts, and provides CLI-specific templates for recurring tasks."                                            |
| F7  | Custom snippets in non-CLI mode                             | Planned           | "Add user-defined snippet triggers for normal dictation mode (non-CLI). When a spoken trigger matches, expand to the configured text block before paste; include settings UI for create/edit/delete and enable/disable."      |
| F21 | Team-shared config packs                                    | Planned           | "Add import/export for configuration packs (templates, snippets, modes, style presets, and jargon sources) using a versioned JSON schema. Support selective import, conflict resolution (keep existing/replace/merge), and safe validation with rollback on failure." |
| F22 | Advanced style presets for post-processing                  | Planned           | "Expand post-processing into configurable style presets (e.g., concise chat, technical docs, polished prose, commit message, CLI-safe). Each preset should define normalization rules, punctuation behavior, capitalization policy, cleanup transforms, and optional rewrite instructions. Add UI to create/edit/duplicate presets and assign defaults per mode/profile." |
| F23 | External/custom jargon sources                              | Planned           | "Add support for multiple custom jargon sources beyond inline terms: local files, folders, and imported glossary packs. Build a merge pipeline with priority order, duplicate handling, and live reload when source files change. Include settings UI for source management, enable/disable per source, and per-mode/per-profile binding." |
| F24 | Intent router sidecar (small open model)                    | Planned           | "Add an intent router sidecar that runs in parallel with transcription to classify utterances into `dictation`, `cli`, `template`, or `command` using a small open embedding model (for example `all-MiniLM-L6-v2` or `bge-small-en-v1.5`) and prototype similarity. Include confidence thresholds, timeout fallback to existing mode behavior, and settings to enable/disable routing." |
| F25 | Domain re-ranker sidecar (small open model)                 | Planned           | "Add a domain re-ranker sidecar that scores candidate post-processed outputs against active context (mode/profile/jargon/workspace signals) using a small open reranker model (for example `ms-marco-MiniLM-L-6-v2`). Run in parallel with existing post-processing under a strict latency budget, select the top-scoring safe candidate, and fall back gracefully when confidence is low or timeout occurs." |

## Notes

- The prompts above are written as implementation prompts for future development work.
- Status should be updated in this file as work is completed.
