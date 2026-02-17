import React, { useState, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { useSettings } from "../../../hooks/useSettings";
import { SettingsGroup } from "../../ui/SettingsGroup";
import { SettingContainer } from "../../ui/SettingContainer";
import { Input } from "../../ui/Input";
import { Button } from "../../ui/Button";
import { commands } from "@/bindings";

interface JargonProfile {
  label: string;
  terms: string[];
  corrections: { from: string; to: string }[];
}

type SelectorPreset = "conservative" | "balanced" | "aggressive" | "custom";

const SELECTOR_PRESETS: Record<
  Exclude<SelectorPreset, "custom">,
  {
    timeout: number;
    topK: number;
    minScore: number;
    hysteresis: number;
  }
> = {
  conservative: { timeout: 40, topK: 1, minScore: 0.22, hysteresis: 0.12 },
  balanced: { timeout: 50, topK: 2, minScore: 0.14, hysteresis: 0.08 },
  aggressive: { timeout: 70, topK: 3, minScore: 0.08, hysteresis: 0.04 },
};

export const JargonSettings: React.FC = () => {
  const { t } = useTranslation();
  const { getSetting, updateSetting, isUpdating } = useSettings();

  const enabledProfiles =
    (getSetting("jargon_enabled_profiles" as any) as string[]) || [];
  const customTerms =
    (getSetting("jargon_custom_terms" as any) as string[]) || [];
  const customCorrections =
    (getSetting("jargon_custom_corrections" as any) as any[]) || [];
  const domainSelectorEnabled =
    (getSetting("domain_selector_enabled" as any) as boolean) || false;
  const domainSelectorTimeout =
    (getSetting("domain_selector_timeout_ms" as any) as number) || 120;
  const domainSelectorTopK =
    (getSetting("domain_selector_top_k" as any) as number) || 2;
  const domainSelectorMinScore =
    (getSetting("domain_selector_min_score" as any) as number) || 0.1;
  const domainSelectorHysteresis =
    (getSetting("domain_selector_hysteresis" as any) as number) || 0.08;
  const domainSelectorBlendManual =
    (getSetting("domain_selector_blend_manual_profiles" as any) as boolean) ??
    true;
  const debugMode = (getSetting("debug_mode" as any) as boolean) || false;

  const [builtinProfiles, setBuiltinProfiles] = useState<
    Record<string, JargonProfile>
  >({});
  const [newTerm, setNewTerm] = useState("");
  const [newCorrectionFrom, setNewCorrectionFrom] = useState("");
  const [newCorrectionTo, setNewCorrectionTo] = useState("");

  useEffect(() => {
    commands.getJargonBuiltinProfiles().then((result) => {
      const profiles = result as any as Record<string, JargonProfile>;
      setBuiltinProfiles(profiles);
    });
  }, []);

  const toggleProfile = (profileId: string) => {
    const newProfiles = enabledProfiles.includes(profileId)
      ? enabledProfiles.filter((p) => p !== profileId)
      : [...enabledProfiles, profileId];
    updateSetting("jargon_enabled_profiles" as any, newProfiles);
  };

  const handleAddTerm = () => {
    const trimmed = newTerm.trim();
    if (
      trimmed &&
      trimmed.length <= 100 &&
      !customTerms.some((t) => t.toLowerCase() === trimmed.toLowerCase())
    ) {
      updateSetting("jargon_custom_terms" as any, [...customTerms, trimmed]);
      setNewTerm("");
    }
  };

  const handleRemoveTerm = (term: string) => {
    updateSetting(
      "jargon_custom_terms" as any,
      customTerms.filter((t) => t !== term),
    );
  };

  const handleAddCorrection = () => {
    const from = newCorrectionFrom.trim();
    const to = newCorrectionTo.trim();
    if (
      from &&
      to &&
      !customCorrections.some(
        (c) => c.from.toLowerCase() === from.toLowerCase(),
      )
    ) {
      updateSetting("jargon_custom_corrections" as any, [
        ...customCorrections,
        { from, to },
      ]);
      setNewCorrectionFrom("");
      setNewCorrectionTo("");
    }
  };

  const handleRemoveCorrection = (index: number) => {
    updateSetting(
      "jargon_custom_corrections",
      customCorrections.filter((_, i) => i !== index),
    );
  };

  const handleTermKeyPress = (e: React.KeyboardEvent) => {
    if (e.key === "Enter") {
      e.preventDefault();
      handleAddTerm();
    }
  };

  const handleCorrectionKeyPress = (e: React.KeyboardEvent) => {
    if (e.key === "Enter") {
      e.preventDefault();
      handleAddCorrection();
    }
  };

  // Compute active counts
  const profileTermCount = enabledProfiles.reduce((acc, id) => {
    const profile = builtinProfiles[id];
    return acc + (profile?.terms.length || 0);
  }, 0);
  const profileCorrectionCount = enabledProfiles.reduce((acc, id) => {
    const profile = builtinProfiles[id];
    return acc + (profile?.corrections.length || 0);
  }, 0);
  const totalTerms = profileTermCount + customTerms.length;
  const totalCorrections = profileCorrectionCount + customCorrections.length;

  const sortedProfileIds = Object.keys(builtinProfiles).sort();
  const activePreset: SelectorPreset =
    (Object.entries(SELECTOR_PRESETS).find(([_, preset]) => {
      return (
        preset.timeout === domainSelectorTimeout &&
        preset.topK === domainSelectorTopK &&
        Math.abs(preset.minScore - domainSelectorMinScore) < 0.001 &&
        Math.abs(preset.hysteresis - domainSelectorHysteresis) < 0.001
      );
    })?.[0] as SelectorPreset | undefined) || "custom";

  const applySelectorPreset = (presetKey: Exclude<SelectorPreset, "custom">) => {
    const preset = SELECTOR_PRESETS[presetKey];
    updateSetting("domain_selector_timeout_ms" as any, preset.timeout as any);
    updateSetting("domain_selector_top_k" as any, preset.topK as any);
    updateSetting("domain_selector_min_score" as any, preset.minScore as any);
    updateSetting("domain_selector_hysteresis" as any, preset.hysteresis as any);
  };

  return (
    <div className="max-w-3xl w-full mx-auto space-y-6">
      <SettingsGroup title={t("settings.jargon.profiles.title")}>
        <SettingContainer
          title={t("settings.jargon.profiles.title")}
          description={t("settings.jargon.profiles.description")}
          descriptionMode="inline"
          grouped
        >
          <div className="flex flex-wrap gap-2">
            {sortedProfileIds.map((id) => {
              const profile = builtinProfiles[id];
              const isEnabled = enabledProfiles.includes(id);
              return (
                <Button
                  key={id}
                  onClick={() => toggleProfile(id)}
                  variant={isEnabled ? "primary" : "secondary"}
                  size="sm"
                  disabled={isUpdating("jargon_enabled_profiles")}
                >
                  {profile.label}
                </Button>
              );
            })}
          </div>
        </SettingContainer>
      </SettingsGroup>

      <SettingsGroup title={t("settings.jargon.selector.title")}>
        <SettingContainer
          title={t("settings.jargon.selector.title")}
          description={t("settings.jargon.selector.simpleDescription")}
          descriptionMode="inline"
          grouped
        >
          <div className="flex flex-wrap items-center gap-2">
            <Button
              onClick={() =>
                updateSetting(
                  "domain_selector_enabled" as any,
                  !domainSelectorEnabled as any,
                )
              }
              variant={domainSelectorEnabled ? "primary" : "secondary"}
              size="sm"
              disabled={isUpdating("domain_selector_enabled")}
            >
              {domainSelectorEnabled
                ? t("common.enabled")
                : t("common.disabled")}
            </Button>
            <Button
              onClick={() =>
                updateSetting(
                  "domain_selector_blend_manual_profiles" as any,
                  !domainSelectorBlendManual as any,
                )
              }
              variant={domainSelectorBlendManual ? "primary" : "secondary"}
              size="sm"
              disabled={isUpdating("domain_selector_blend_manual_profiles")}
            >
              {domainSelectorBlendManual
                ? t("settings.jargon.selector.blendManualOn")
                : t("settings.jargon.selector.blendManualOff")}
            </Button>
          </div>
          <div className="mt-3">
            <p className="text-xs text-mid-gray mb-2">
              {t("settings.jargon.selector.modeLabel")}
            </p>
            <div className="flex flex-wrap gap-2">
              <Button
                onClick={() => applySelectorPreset("conservative")}
                variant={activePreset === "conservative" ? "primary" : "secondary"}
                size="sm"
                disabled={!domainSelectorEnabled}
              >
                {t("settings.jargon.selector.modes.conservative")}
              </Button>
              <Button
                onClick={() => applySelectorPreset("balanced")}
                variant={activePreset === "balanced" ? "primary" : "secondary"}
                size="sm"
                disabled={!domainSelectorEnabled}
              >
                {t("settings.jargon.selector.modes.balanced")}
              </Button>
              <Button
                onClick={() => applySelectorPreset("aggressive")}
                variant={activePreset === "aggressive" ? "primary" : "secondary"}
                size="sm"
                disabled={!domainSelectorEnabled}
              >
                {t("settings.jargon.selector.modes.aggressive")}
              </Button>
            </div>
            {activePreset === "custom" && (
              <p className="text-xs text-mid-gray mt-2">
                {t("settings.jargon.selector.customModeHint")}
              </p>
            )}
          </div>

          {debugMode && (
            <details className="mt-4">
              <summary className="text-xs text-mid-gray cursor-pointer select-none">
                {t("settings.jargon.selector.advancedTitle")}
              </summary>
              <div className="grid grid-cols-1 md:grid-cols-2 gap-2 mt-2">
                <Input
                  type="number"
                  value={String(domainSelectorTimeout)}
                  onChange={(event) =>
                    updateSetting(
                      "domain_selector_timeout_ms" as any,
                      Number(event.target.value) as any,
                    )
                  }
                  placeholder={t("settings.jargon.selector.timeoutPlaceholder")}
                  variant="compact"
                  disabled={isUpdating("domain_selector_timeout_ms")}
                />
                <Input
                  type="number"
                  value={String(domainSelectorTopK)}
                  onChange={(event) =>
                    updateSetting(
                      "domain_selector_top_k" as any,
                      Number(event.target.value) as any,
                    )
                  }
                  placeholder={t("settings.jargon.selector.topKPlaceholder")}
                  variant="compact"
                  disabled={isUpdating("domain_selector_top_k")}
                />
                <Input
                  type="number"
                  step="0.01"
                  min="0"
                  max="1"
                  value={String(domainSelectorMinScore)}
                  onChange={(event) =>
                    updateSetting(
                      "domain_selector_min_score" as any,
                      Number(event.target.value) as any,
                    )
                  }
                  placeholder={t("settings.jargon.selector.minScorePlaceholder")}
                  variant="compact"
                  disabled={isUpdating("domain_selector_min_score")}
                />
                <Input
                  type="number"
                  step="0.01"
                  min="0"
                  max="1"
                  value={String(domainSelectorHysteresis)}
                  onChange={(event) =>
                    updateSetting(
                      "domain_selector_hysteresis" as any,
                      Number(event.target.value) as any,
                    )
                  }
                  placeholder={t("settings.jargon.selector.hysteresisPlaceholder")}
                  variant="compact"
                  disabled={isUpdating("domain_selector_hysteresis")}
                />
              </div>
            </details>
          )}
        </SettingContainer>
      </SettingsGroup>

      <SettingsGroup title={t("settings.jargon.customTerms.title")}>
        <SettingContainer
          title={t("settings.jargon.customTerms.title")}
          description={t("settings.jargon.customTerms.description")}
          descriptionMode="inline"
          grouped
        >
          <div className="flex items-center gap-2">
            <Input
              type="text"
              className="max-w-48"
              value={newTerm}
              onChange={(e) => setNewTerm(e.target.value)}
              onKeyDown={handleTermKeyPress}
              placeholder={t("settings.jargon.customTerms.placeholder")}
              variant="compact"
              disabled={isUpdating("jargon_custom_terms")}
            />
            <Button
              onClick={handleAddTerm}
              disabled={
                !newTerm.trim() ||
                newTerm.trim().length > 100 ||
                isUpdating("jargon_custom_terms")
              }
              variant="primary"
              size="md"
            >
              {t("settings.jargon.customTerms.add")}
            </Button>
          </div>
        </SettingContainer>
        {customTerms.length > 0 && (
          <div className="px-4 p-2 flex flex-wrap gap-1">
            {customTerms.map((term) => (
              <Button
                key={term}
                onClick={() => handleRemoveTerm(term)}
                disabled={isUpdating("jargon_custom_terms")}
                variant="secondary"
                size="sm"
                className="inline-flex items-center gap-1 cursor-pointer"
              >
                <span>{term}</span>
                <svg
                  className="w-3 h-3"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth={2}
                    d="M6 18L18 6M6 6l12 12"
                  />
                </svg>
              </Button>
            ))}
          </div>
        )}
      </SettingsGroup>

      <SettingsGroup title={t("settings.jargon.customCorrections.title")}>
        <SettingContainer
          title={t("settings.jargon.customCorrections.title")}
          description={t("settings.jargon.customCorrections.description")}
          descriptionMode="inline"
          grouped
        >
          <div className="flex items-center gap-2">
            <Input
              type="text"
              className="max-w-36"
              value={newCorrectionFrom}
              onChange={(e) => setNewCorrectionFrom(e.target.value)}
              onKeyDown={handleCorrectionKeyPress}
              placeholder={t(
                "settings.jargon.customCorrections.fromPlaceholder",
              )}
              variant="compact"
              disabled={isUpdating("jargon_custom_corrections")}
            />
            <span className="text-mid-gray">&rarr;</span>
            <Input
              type="text"
              className="max-w-36"
              value={newCorrectionTo}
              onChange={(e) => setNewCorrectionTo(e.target.value)}
              onKeyDown={handleCorrectionKeyPress}
              placeholder={t("settings.jargon.customCorrections.toPlaceholder")}
              variant="compact"
              disabled={isUpdating("jargon_custom_corrections")}
            />
            <Button
              onClick={handleAddCorrection}
              disabled={
                !newCorrectionFrom.trim() ||
                !newCorrectionTo.trim() ||
                isUpdating("jargon_custom_corrections")
              }
              variant="primary"
              size="md"
            >
              {t("settings.jargon.customCorrections.add")}
            </Button>
          </div>
        </SettingContainer>
        {customCorrections.length > 0 && (
          <div className="px-4 p-2 space-y-1">
            {customCorrections.map((correction, index) => (
              <div key={index} className="flex items-center gap-2 text-sm">
                <span className="text-mid-gray/80">{correction.from}</span>
                <span className="text-mid-gray">&rarr;</span>
                <span>{correction.to}</span>
                <Button
                  onClick={() => handleRemoveCorrection(index)}
                  disabled={isUpdating("jargon_custom_corrections")}
                  variant="secondary"
                  size="sm"
                  className="ml-auto"
                  aria-label={t("settings.jargon.customCorrections.remove")}
                >
                  <svg
                    className="w-3 h-3"
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                  >
                    <path
                      strokeLinecap="round"
                      strokeLinejoin="round"
                      strokeWidth={2}
                      d="M6 18L18 6M6 6l12 12"
                    />
                  </svg>
                </Button>
              </div>
            ))}
          </div>
        )}
      </SettingsGroup>

      {(totalTerms > 0 || totalCorrections > 0) && (
        <SettingsGroup title={t("settings.jargon.summary.title")}>
          <div className="px-4 py-2 text-sm text-mid-gray">
            <p>
              {t("settings.jargon.summary.terms", { count: totalTerms })}
              {" | "}
              {t("settings.jargon.summary.corrections", {
                count: totalCorrections,
              })}
            </p>
          </div>
        </SettingsGroup>
      )}
    </div>
  );
};
