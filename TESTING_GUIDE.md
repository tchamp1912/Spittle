# @File Expansion - Testing Guide

## ✅ Build Status

- **Rust:** ✅ Compiled successfully
- **Tests:** ✅ 14/14 unit tests passing
- **Frontend:** ✅ ESLint passed
- **App:** ✅ Running now (see process below)
- **Bundle:** ✅ Built at `src-tauri/target/release/bundle/macos/Spittle.app`

## App Running Now

```
thomas           30270   0.1  0.5 442775088  76752   ??  SN    7:39PM   0:39.08 target/debug/spittle
```

## Quick Test Instructions

### 1. Access Settings

- App is running (check menu bar tray icon)
- Click menu bar icon → Settings
- Or press the keyboard shortcut shown

### 2. Enable Feature

- Navigate to **Advanced** tab
- Find **App** section
- Enable **Experimental Features** toggle
- Look for **Experimental** section below
- Enable **@File Expansion** toggle

### 3. Set Up Workspace (Choose One)

#### Option A: Using Cursor (Recommended)

1. Install the Cursor extension:
   ```bash
   # Build the extension (optional, it auto-compiles)
   cd extensions/cursor-context
   npm install  # or bun install
   npm run build
   ```
2. Open Cursor with a code project
3. Spittle will auto-detect the workspace via the extension

#### Option B: Using iTerm2

1. Open iTerm2 and `cd` to a code project directory
2. Spittle will read the CWD via osascript

#### Option C: MRU Fallback

1. Previous workspaces are remembered in settings
2. Will use most recent workspace

### 4. Test Dictation

1. **Start recording:** Option+Space (macOS default)
2. **Dictate:** `"Check @auth.ts"` or `"Look at @utils.rs"` or `"See @src/lib/helpers.ts"`
3. **Stop recording:** Option+Space again
4. **Observe:**
   - Text is pasted (with `@` tokens intact, e.g., "Check auth.ts")
   - **File snippet appended** below the text (if exactly 1 match found)
   - Snippet includes: filename, language, code preview

### 5. Verify Results

#### ✅ Expected Behavior (File Found & Expanded)

````
Check auth.ts
------------------------------------------------------------
### Referenced file: src/auth.ts
```typescript
export const auth = {
  login: async (user: string, pass: string) => {
    // implementation
  }
}
````

#### ✅ No Change (0 Matches)

```
Check @nonexistent_file.ts
(No snippet appended, text pasted as-is)
```

#### ✅ No Change (2+ Matches - Ambiguous)

```
Check @utils.ts
(Two files match: src/utils.ts and lib/utils.ts)
(No snippet, text pasted unchanged with @token)
```

#### ✅ Email Skipped (Not a Token)

```
Send this to user@example.com
(Not treated as @token, no expansion attempted)
```

## Test Cases to Try

1. **Basename match:**
   - Project structure: `src/auth.ts`
   - Dictate: `"Check @auth.ts"`
   - Expected: ✅ File snippet appended

2. **Relative path match:**
   - Project structure: `src/lib/helpers.ts`
   - Dictate: `"See @src/lib/helpers.ts"`
   - Expected: ✅ File snippet appended

3. **Quoted filename:**
   - Project structure: `src/my module.ts`
   - Dictate: `"Check @\"my module.ts\""`
   - Expected: ✅ File snippet appended

4. **No match:**
   - Dictate: `"See @does-not-exist.ts"`
   - Expected: ✅ Text pasted, no snippet (0 matches)

5. **Ambiguous (2+ matches):**
   - Project structure: `src/auth.ts` and `lib/auth.ts`
   - Dictate: `"Check @auth.ts"`
   - Expected: ✅ Text pasted, no snippet (ambiguous)
   - Workaround: `"Check @src/auth.ts"` (use relative path)

6. **Email not treated as token:**
   - Dictate: `"Contact user@example.com about the issue"`
   - Expected: ✅ Text pasted, no expansion attempt

7. **Binary file skipped:**
   - Project structure: `assets/logo.png`
   - Dictate: `"Add @logo.png to banner"`
   - Expected: ✅ Text pasted, file skipped (binary)

## Troubleshooting

### Feature Toggle Not Showing

- ✅ Requirement: Must enable "Experimental Features" first
- Then scroll down to find "@File Expansion"

### Not Expanding Files

1. **Feature enabled?** Check Advanced → Experimental → @File Expansion
2. **Workspace detected?** Check frontmost app:
   - Cursor/VS Code open with project?
   - Or iTerm2 cd'd to project?
   - Or MRU has recent workspace?
3. **Token syntax correct?** Use `@filename` or `@"quoted name"` format
4. **Ambiguous?** Check if file matches multiple entries
5. **Binary file?** Check if file is text-based (code files work)

### History vs Paste

- ✅ **History:** Shows pre-expansion (clean text without snippets)
- ✅ **Paste:** Includes expanded snippets
- This is by design (history stores raw transcription)

## Performance Notes

- **First expansion:** May take 1-2 seconds (workspace indexing)
- **Subsequent expansions:** Much faster (same workspace)
- **Large projects:** Max 50K files indexed, max 10 levels deep
- **Snippet size:** Max 200 lines or 25KB per file

## Logs & Debugging

Check console logs (if dev mode running):

```bash
# Dev mode to see logs
export USE_AI_STUB=1
CMAKE_POLICY_VERSION_MINIMUM=3.5 bun run tauri dev
```

Log output shows:

- `@tokens` found and their count
- Workspace root detected (Cursor/iTerm2/MRU)
- File walks (entry count)
- Resolution attempts (0/1/2+ matches)
- Snippet extraction (success/binary-skip)

## Reset Settings

To disable the feature:

- Advanced Settings → Experimental → @File Expansion toggle → OFF

To clear MRU workspaces:

- Settings stored in `~/.config/Spittle/store.json` (approximate location)
- Or just open different workspace in Cursor/iTerm2 to update MRU

## Next Steps After Testing

1. ✅ Verify feature works with your workflow
2. ✅ Report any edge cases or improvements
3. ✅ Install Cursor extension for seamless detection
4. ✅ Customize directory filters if needed

---

**Ready to test!** The app is running now. Open it and navigate to Advanced Settings.
