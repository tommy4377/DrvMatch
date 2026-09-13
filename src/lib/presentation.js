// @ts-check

/** @param {import('./types').SignatureStatus} signature */
export function signatureLabel(signature) {
  return ({
    whql: "WHQL",
    inbox: "Microsoft inbox",
    authenticode: "Authenticode",
    signedUnclassified: "Microsoft-signed",
    unsigned: "Unsigned",
    unknown: "Not determined",
  })[signature];
}

/** @param {import('./types').DriverSourceKind} source */
export function sourceLabel(source) {
  return ({
    windowsUpdate: "Windows Update",
    microsoftCatalog: "Microsoft Update Catalog",
    amd: "AMD",
    nvidia: "NVIDIA",
    intel: "Intel",
    dell: "Dell",
    lenovo: "Lenovo",
    hp: "HP",
  })[source];
}

/** @param {import('./types').CompatibilityState} state */
export function compatibilityLabel(state) {
  return state === "compatible"
    ? "Compatible"
    : state === "needsReview"
      ? "Needs package review"
      : "Rejected";
}

/** @param {import('./types').RecommendationState} state */
export function recommendationLabel(state) {
  return ({
    recommended: "Recommended",
    optional: "Optional",
    current: "Current",
    missing: "Missing",
    notRecommended: "Not recommended",
  })[state];
}

/** @param {import('./types').SourceHealthState} state */
export function sourceStateLabel(state) {
  return state === "available" ? "Available" : state === "skipped" ? "Skipped" : "Unavailable";
}

/** @param {import('./types').DriverCandidate} candidate */
export function formatCandidateDate(candidate) {
  if (candidate.publicationDate) return candidate.publicationDate;
  return candidate.driverDate
    ? new Date(candidate.driverDate * 1000).toLocaleDateString()
    : "Date not reported";
}

/** @param {number | null} bytes */
export function formatBytes(bytes) {
  if (bytes === null) return "Size not reported";
  const units = ["B", "KB", "MB", "GB"];
  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit += 1;
  }
  return `${value.toFixed(unit > 1 ? 1 : 0)} ${units[unit]}`;
}


/** @param {import('./types').Device} device @returns {import('./types').HardwareCategory} */
export function deviceCategory(device) {
  const cls = (device.className ?? "").toLocaleLowerCase().replace(/\s+/g, "");
  const text = `${device.friendlyName} ${device.description}`.toLocaleLowerCase();

  if (["display", "monitor"].includes(cls)) return "display";
  if (["net", "netclient", "netservice", "nettrans"].includes(cls)) return "network";
  if (["bluetooth"].includes(cls)) return "bluetooth";
  if (["media", "audioendpoint", "sound", "soundvideoandgamecontrollers"].includes(cls)) return "audio";
  if (["diskdrive", "storagevolume", "volume", "scsiadapter", "hdc", "cdrom"].includes(cls)) return "storage";
  if (["keyboard", "mouse", "hidclass"].includes(cls)) return "input";
  if (["system", "computer", "processor", "softwarecomponent", "securitydevices"].includes(cls)) return "system";
  if (["usb", "usbdevice"].includes(cls)) return "usb";
  if (["camera", "image"].includes(cls)) return "camera";

  if (/\b(display|monitor)\b/.test(text)) return "display";
  if (/\b(network|ethernet|wi-?fi|wireless|lan)\b/.test(text)) return "network";
  if (/\bbluetooth\b/.test(text)) return "bluetooth";
  if (/\b(audio|sound|speaker|microphone|headset|media)\b/.test(text)) return "audio";
  if (/\b(disk|storage|nvme|sata|scsi|volume)\b/.test(text)) return "storage";
  if (/\b(keyboard|mouse|controller|input)\b/.test(text)) return "input";
  if (/\b(processor|chipset|system|firmware)\b/.test(text)) return "system";
  if (/\busb\b/.test(text)) return "usb";
  if (/\b(camera|webcam)\b/.test(text)) return "camera";
  return "other";
}

/** @param {import('./types').HardwareCategory} category */
export function hardwareCategoryLabel(category) {
  return ({
    display: "Display",
    network: "Network",
    audio: "Audio",
    bluetooth: "Bluetooth",
    storage: "Storage",
    input: "Input",
    system: "System",
    usb: "USB",
    camera: "Camera",
    other: "Other",
  })[category];
}

/** @param {import('./types').Device} device */
export function deviceReviewPriority(device) {
  if (device.condition === "problem") return 0;
  if (device.condition === "missing") return 1;
  if (device.installedDriver?.genericMicrosoft) return 2;
  return 3;
}
