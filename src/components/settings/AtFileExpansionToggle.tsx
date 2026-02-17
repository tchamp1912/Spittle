import React from "react";
import { useTranslation } from "react-i18next";
import { ToggleSwitch } from "../ui/ToggleSwitch";
import { useSettings } from "../../hooks/useSettings";

interface AtFileExpansionToggleProps {
  descriptionMode?: "inline" | "tooltip";
  grouped?: boolean;
}

export const AtFileExpansionToggle: React.FC<AtFileExpansionToggleProps> =
  React.memo(({ descriptionMode = "tooltip", grouped = false }) => {
    const { t } = useTranslation();
    const { getSetting, updateSetting, isUpdating } = useSettings();

    const enabled = getSetting("at_file_expansion_enabled") || false;

    return (
      <ToggleSwitch
        checked={enabled}
        onChange={(enabled) =>
          updateSetting("at_file_expansion_enabled", enabled)
        }
        isUpdating={isUpdating("at_file_expansion_enabled")}
        label={t("settings.advanced.atFileExpansion.label")}
        description={t("settings.advanced.atFileExpansion.description")}
        descriptionMode={descriptionMode}
        grouped={grouped}
      />
    );
  });
