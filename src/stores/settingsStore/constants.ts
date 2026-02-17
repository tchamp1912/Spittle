import type { AudioDevice } from "@/bindings";

export const DEFAULT_AUDIO_DEVICE: AudioDevice = {
  index: "default",
  name: "Default",
  is_default: true,
};

export const withDefaultAudioDevice = (devices: AudioDevice[]): AudioDevice[] => [
  DEFAULT_AUDIO_DEVICE,
  ...devices.filter((d) => d.name !== "Default" && d.name !== "default"),
];
