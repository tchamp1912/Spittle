import { commands } from "@/bindings";
import type { AppSettings as Settings } from "@/bindings";

export const settingUpdaters: {
  [K in keyof Settings]?: (value: Settings[K]) => Promise<unknown>;
} = {
  always_on_microphone: (value) =>
    commands.updateMicrophoneMode(value as boolean),
  audio_feedback: (value) =>
    commands.changeAudioFeedbackSetting(value as boolean),
  audio_feedback_volume: (value) =>
    commands.changeAudioFeedbackVolumeSetting(value as number),
  sound_theme: (value) => commands.changeSoundThemeSetting(value as string),
  start_hidden: (value) => commands.changeStartHiddenSetting(value as boolean),
  autostart_enabled: (value) =>
    commands.changeAutostartSetting(value as boolean),
  update_checks_enabled: (value) =>
    commands.changeUpdateChecksSetting(value as boolean),
  push_to_talk: (value) => commands.changePttSetting(value as boolean),
  selected_microphone: (value) =>
    commands.setSelectedMicrophone(
      (value as string) === "Default" || value === null
        ? "default"
        : (value as string),
    ),
  clamshell_microphone: (value) =>
    commands.setClamshellMicrophone(
      (value as string) === "Default" ? "default" : (value as string),
    ),
  selected_output_device: (value) =>
    commands.setSelectedOutputDevice(
      (value as string) === "Default" || value === null
        ? "default"
        : (value as string),
    ),
  recording_retention_period: (value) =>
    commands.updateRecordingRetentionPeriod(value as string),
  translate_to_english: (value) =>
    commands.changeTranslateToEnglishSetting(value as boolean),
  selected_language: (value) =>
    commands.changeSelectedLanguageSetting(value as string),
  overlay_position: (value) =>
    commands.changeOverlayPositionSetting(value as string),
  debug_mode: (value) => commands.changeDebugModeSetting(value as boolean),
  custom_words: (value) => commands.updateCustomWords(value as string[]),
  word_correction_threshold: (value) =>
    commands.changeWordCorrectionThresholdSetting(value as number),
  paste_method: (value) => commands.changePasteMethodSetting(value as string),
  typing_tool: (value) => commands.changeTypingToolSetting(value as string),
  clipboard_handling: (value) =>
    commands.changeClipboardHandlingSetting(value as string),
  auto_submit: (value) => commands.changeAutoSubmitSetting(value as boolean),
  auto_submit_key: (value) =>
    commands.changeAutoSubmitKeySetting(value as string),
  history_limit: (value) => commands.updateHistoryLimit(value as number),
  post_process_enabled: (value) =>
    commands.changePostProcessEnabledSetting(value as boolean),
  post_process_auto_prompt_selection: (value) =>
    commands.changePostProcessAutoPromptSelectionSetting(value as boolean),
  post_process_selected_prompt_id: (value) =>
    commands.setPostProcessSelectedPrompt(value as string),
  mute_while_recording: (value) =>
    commands.changeMuteWhileRecordingSetting(value as boolean),
  audio_segment_size_seconds: (value) =>
    commands.changeAudioSegmentSizeSecondsSetting(value as number),
  append_trailing_space: (value) =>
    commands.changeAppendTrailingSpaceSetting(value as boolean),
  at_file_expansion_enabled: (value) =>
    commands.changeAtFileExpansionSetting(value as boolean),
  jargon_enabled_profiles: (value) =>
    commands.updateJargonProfiles(value as string[]),
  jargon_custom_terms: (value) =>
    commands.updateJargonCustomTerms(value as string[]),
  jargon_custom_corrections: (value) =>
    commands.updateJargonCustomCorrections(value as any),
  domain_selector_enabled: (value) =>
    commands.updateDomainSelectorEnabledSetting(value as boolean),
  domain_selector_timeout_ms: (value) =>
    commands.updateDomainSelectorTimeoutMsSetting(value as number),
  domain_selector_top_k: (value) =>
    commands.updateDomainSelectorTopKSetting(value as number),
  domain_selector_min_score: (value) =>
    commands.updateDomainSelectorMinScoreSetting(value as number),
  domain_selector_hysteresis: (value) =>
    commands.updateDomainSelectorHysteresisSetting(value as number),
  domain_selector_blend_manual_profiles: (value) =>
    commands.updateDomainSelectorBlendManualProfilesSetting(
      value as boolean,
    ),
  log_level: (value) => commands.setLogLevel(value as any),
  app_language: (value) => commands.changeAppLanguageSetting(value as string),
  experimental_enabled: (value) =>
    commands.changeExperimentalEnabledSetting(value as boolean),
  show_tray_icon: (value) =>
    commands.changeShowTrayIconSetting(value as boolean),
};
