import React from "react";
import { useTranslation } from "react-i18next";
import { Slider } from "../ui/Slider";
import { useSettings } from "../../hooks/useSettings";

interface SegmentOnSilenceProps {
  descriptionMode?: "inline" | "tooltip";
  grouped?: boolean;
}

export const SegmentOnSilence: React.FC<SegmentOnSilenceProps> = React.memo(
  ({ descriptionMode = "tooltip", grouped = false }) => {
    const { t } = useTranslation();
    const { getSetting, updateSetting, isUpdating } = useSettings();

    const audioSegmentSizeSeconds =
      getSetting("audio_segment_size_seconds") ?? 0;

    return (
      <Slider
        value={audioSegmentSizeSeconds as number}
        onChange={(seconds) =>
          updateSetting("audio_segment_size_seconds", seconds)
        }
        min={0}
        max={10}
        step={0.5}
        disabled={isUpdating("audio_segment_size_seconds")}
        label={t("settings.general.segmentOnSilence.label")}
        description={t("settings.general.segmentOnSilence.description")}
        descriptionMode={descriptionMode}
        grouped={grouped}
        showValue={true}
        formatValue={(value) => `${value.toFixed(1)}s`}
      />
    );
  },
);
