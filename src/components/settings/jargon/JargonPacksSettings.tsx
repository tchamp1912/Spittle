import React, { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { open, save } from "@tauri-apps/plugin-dialog";
import { readTextFile, writeTextFile } from "@tauri-apps/plugin-fs";
import { commands } from "@/bindings";
import { Button } from "../../ui/Button";
import { Input } from "../../ui/Input";
import { SettingsGroup } from "../../ui/SettingsGroup";
import { SettingContainer } from "../../ui/SettingContainer";

type JargonCorrection = { from: string; to: string };
type LocalJargonPack = {
  id: string;
  label: string;
  terms: string[];
  corrections: JargonCorrection[];
};

const parseCsvOrLines = (value: string): string[] =>
  value
    .split(/[\n,]/g)
    .map((item) => item.trim())
    .filter(Boolean);

const parseCorrections = (value: string): JargonCorrection[] =>
  value
    .split("\n")
    .map((line) => line.trim())
    .filter(Boolean)
    .map((line) => {
      const [from, to] = line.split("=>").map((part) => part.trim());
      if (!from || !to) {
        return null;
      }
      return { from, to };
    })
    .filter((item): item is JargonCorrection => item !== null);

export const JargonPacksSettings: React.FC = () => {
  const { t } = useTranslation();
  const [packs, setPacks] = useState<LocalJargonPack[]>([]);
  const [loading, setLoading] = useState(true);
  const [isSaving, setIsSaving] = useState(false);
  const [id, setId] = useState("");
  const [label, setLabel] = useState("");
  const [termsInput, setTermsInput] = useState("");
  const [correctionsInput, setCorrectionsInput] = useState("");
  const [selectedPackIds, setSelectedPackIds] = useState<string[]>([]);

  const refreshPacks = async () => {
    try {
      const result = await commands.getJargonPacks();
      setPacks(
        Array.isArray(result)
          ? result.map((pack) => ({
              id: pack.id,
              label: pack.label,
              terms: pack.terms ?? [],
              corrections: pack.corrections ?? [],
            }))
          : [],
      );
    } catch (error) {
      console.error("Failed to load jargon packs:", error);
      setPacks([]);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    refreshPacks();
  }, []);

  const persistPacks = async (nextPacks: LocalJargonPack[]) => {
    setIsSaving(true);
    try {
      const result = await commands.updateJargonPacks(nextPacks);
      if (result.status === "ok") {
        setPacks(result.data);
      }
    } catch (error) {
      console.error("Failed to update jargon packs:", error);
    } finally {
      setIsSaving(false);
    }
  };

  const addPack = async () => {
    const trimmedId = id.trim();
    const trimmedLabel = label.trim();
    if (!trimmedId || !trimmedLabel) {
      return;
    }
    if (packs.some((pack) => pack.id === trimmedId)) {
      return;
    }

    const nextPack: LocalJargonPack = {
      id: trimmedId,
      label: trimmedLabel,
      terms: parseCsvOrLines(termsInput),
      corrections: parseCorrections(correctionsInput),
    };
    await persistPacks([...packs, nextPack]);
    setId("");
    setLabel("");
    setTermsInput("");
    setCorrectionsInput("");
  };

  const deletePack = async (packId: string) => {
    await persistPacks(packs.filter((pack) => pack.id !== packId));
    setSelectedPackIds((prev) => prev.filter((id) => id !== packId));
  };

  const toggleSelection = (packId: string) => {
    setSelectedPackIds((prev) =>
      prev.includes(packId)
        ? prev.filter((id) => id !== packId)
        : [...prev, packId],
    );
  };

  const importPacks = async () => {
    try {
      const selected = await open({
        title: t("settings.jargonPacks.importDialogTitle"),
        filters: [{ name: "JSON", extensions: ["json"] }],
        multiple: false,
      });
      if (!selected || Array.isArray(selected)) {
        return;
      }
      const json = await readTextFile(selected);
      const result = await commands.importJargonPacksJson(json, false);
      if (result.status === "ok") {
        setPacks(
          result.data.map((pack) => ({
            id: pack.id,
            label: pack.label,
            terms: pack.terms ?? [],
            corrections: pack.corrections ?? [],
          })),
        );
      }
    } catch (error) {
      console.error("Failed to import jargon packs:", error);
    }
  };

  const exportPacks = async () => {
    try {
      const result = await commands.exportJargonPacksJson(
        selectedPackIds.length > 0 ? selectedPackIds : null,
      );
      if (result.status !== "ok") {
        return;
      }
      const path = await save({
        title: t("settings.jargonPacks.exportDialogTitle"),
        defaultPath: "spittle-jargon-packs.json",
        filters: [{ name: "JSON", extensions: ["json"] }],
      });
      if (!path) {
        return;
      }
      await writeTextFile(path, result.data);
    } catch (error) {
      console.error("Failed to export jargon packs:", error);
    }
  };

  const summary = useMemo(() => {
    const termCount = packs.reduce((sum, pack) => sum + pack.terms.length, 0);
    const correctionCount = packs.reduce(
      (sum, pack) => sum + pack.corrections.length,
      0,
    );
    return { termCount, correctionCount };
  }, [packs]);

  return (
    <div className="max-w-3xl w-full mx-auto space-y-6">
      <SettingsGroup title={t("settings.jargonPacks.title")}>
        <SettingContainer
          title={t("settings.jargonPacks.actions.title")}
          description={t("settings.jargonPacks.actions.description")}
          descriptionMode="inline"
          grouped
        >
          <div className="flex flex-wrap gap-2">
            <Button
              onClick={importPacks}
              variant="secondary"
              size="sm"
              disabled={isSaving}
            >
              {t("settings.jargonPacks.actions.import")}
            </Button>
            <Button
              onClick={exportPacks}
              variant="secondary"
              size="sm"
              disabled={isSaving || packs.length === 0}
            >
              {t("settings.jargonPacks.actions.export")}
            </Button>
          </div>
        </SettingContainer>
      </SettingsGroup>

      <SettingsGroup title={t("settings.jargonPacks.create.title")}>
        <SettingContainer
          title={t("settings.jargonPacks.create.title")}
          description={t("settings.jargonPacks.create.description")}
          descriptionMode="inline"
          grouped
        >
          <div className="grid grid-cols-1 gap-2 md:grid-cols-2">
            <Input
              value={id}
              onChange={(event) => setId(event.target.value)}
              placeholder={t("settings.jargonPacks.create.idPlaceholder")}
              variant="compact"
              disabled={isSaving}
            />
            <Input
              value={label}
              onChange={(event) => setLabel(event.target.value)}
              placeholder={t("settings.jargonPacks.create.labelPlaceholder")}
              variant="compact"
              disabled={isSaving}
            />
          </div>
          <textarea
            className="w-full rounded-md border border-mid-gray/20 bg-transparent p-2 text-sm mt-2"
            rows={4}
            value={termsInput}
            onChange={(event) => setTermsInput(event.target.value)}
            placeholder={t("settings.jargonPacks.create.termsPlaceholder")}
            disabled={isSaving}
          />
          <textarea
            className="w-full rounded-md border border-mid-gray/20 bg-transparent p-2 text-sm mt-2"
            rows={4}
            value={correctionsInput}
            onChange={(event) => setCorrectionsInput(event.target.value)}
            placeholder={t("settings.jargonPacks.create.correctionsPlaceholder")}
            disabled={isSaving}
          />
          <div className="mt-2">
            <Button
              onClick={addPack}
              variant="primary"
              size="sm"
              disabled={isSaving || !id.trim() || !label.trim()}
            >
              {t("settings.jargonPacks.create.add")}
            </Button>
          </div>
        </SettingContainer>
      </SettingsGroup>

      <SettingsGroup title={t("settings.jargonPacks.list.title")}>
        <div className="space-y-2">
          {loading && (
            <div className="px-4 py-2 text-sm text-mid-gray">
              {t("common.loading")}
            </div>
          )}
          {!loading && packs.length === 0 && (
            <div className="px-4 py-2 text-sm text-mid-gray">
              {t("settings.jargonPacks.list.empty")}
            </div>
          )}
          {packs.map((pack) => (
            <div
              key={pack.id}
              className="px-4 py-3 border border-mid-gray/20 rounded-lg flex items-start gap-3"
            >
              <input
                type="checkbox"
                checked={selectedPackIds.includes(pack.id)}
                onChange={() => toggleSelection(pack.id)}
                className="mt-1"
              />
              <div className="flex-1 min-w-0">
                <p className="text-sm font-medium">{pack.label}</p>
                <p className="text-xs text-mid-gray">{pack.id}</p>
                <p className="text-xs text-mid-gray mt-1">
                  {t("settings.jargonPacks.list.summary", {
                    terms: pack.terms.length,
                    corrections: pack.corrections.length,
                  })}
                </p>
              </div>
              <Button
                onClick={() => deletePack(pack.id)}
                variant="secondary"
                size="sm"
                disabled={isSaving}
              >
                {t("common.delete")}
              </Button>
            </div>
          ))}
        </div>
      </SettingsGroup>

      <SettingsGroup title={t("settings.jargonPacks.summary.title")}>
        <div className="px-4 py-2 text-sm text-mid-gray">
          {t("settings.jargonPacks.summary.text", {
            packs: packs.length,
            terms: summary.termCount,
            corrections: summary.correctionCount,
          })}
        </div>
      </SettingsGroup>
    </div>
  );
};
