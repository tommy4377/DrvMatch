import { describe, it } from "node:test";
import assert from "node:assert/strict";
import {
  compatibilityLabel,
  deviceCategory,
  deviceReviewPriority,
  formatBytes,
  hardwareCategoryLabel,
  recommendationLabel,
  signatureLabel,
  sourceLabel,
  sourceStateLabel,
} from "./presentation.js";

describe("driver presentation", () => {
  it("uses product-facing source names", () => {
    assert.equal(sourceLabel("windowsUpdate"), "Windows Update");
    assert.equal(sourceLabel("microsoftCatalog"), "Microsoft Update Catalog");
    assert.equal(sourceLabel("amd"), "AMD");
    assert.equal(sourceLabel("dell"), "Dell");
    assert.equal(sourceLabel("lenovo"), "Lenovo");
    assert.equal(sourceLabel("hp"), "HP");
  });

  it("keeps recommendation language conservative", () => {
    assert.equal(recommendationLabel("current"), "Current");
    assert.equal(recommendationLabel("optional"), "Optional");
    assert.equal(recommendationLabel("notRecommended"), "Not recommended");
  });

  it("distinguishes package review from compatibility", () => {
    assert.equal(compatibilityLabel("compatible"), "Compatible");
    assert.equal(compatibilityLabel("needsReview"), "Needs package review");
    assert.equal(compatibilityLabel("incompatible"), "Rejected");
  });

  it("presents source health and signature states without fake certainty", () => {
    assert.equal(sourceStateLabel("available"), "Available");
    assert.equal(sourceStateLabel("failed"), "Unavailable");
    assert.equal(signatureLabel("whql"), "WHQL");
    assert.equal(signatureLabel("unknown"), "Not determined");
  });

  it("formats package sizes consistently", () => {
    assert.equal(formatBytes(null), "Size not reported");
    assert.equal(formatBytes(512), "512 B");
    assert.equal(formatBytes(1024), "1 KB");
    assert.equal(formatBytes(5 * 1024 * 1024), "5.0 MB");
  });
  it("groups hardware into product-facing categories", () => {
    const base = { className: null, friendlyName: "", description: "", condition: "current", installedDriver: null };
    assert.equal(deviceCategory({ ...base, className: "Display", friendlyName: "Radeon RX 7600" }), "display");
    assert.equal(deviceCategory({ ...base, className: "Net", friendlyName: "Realtek PCIe 2.5GbE" }), "network");
    assert.equal(deviceCategory({ ...base, className: "AudioEndpoint", friendlyName: "Speakers" }), "audio");
    assert.equal(deviceCategory({ ...base, className: "Bluetooth", friendlyName: "MediaTek Bluetooth" }), "bluetooth");
    assert.equal(hardwareCategoryLabel("system"), "System");
  });

  it("orders real device problems before generic-driver review", () => {
    const base = { className: null, friendlyName: "Device", description: "", installedDriver: null };
    assert.equal(deviceReviewPriority({ ...base, condition: "problem" }), 0);
    assert.equal(deviceReviewPriority({ ...base, condition: "missing" }), 1);
    assert.equal(deviceReviewPriority({ ...base, condition: "current", installedDriver: { genericMicrosoft: true } }), 2);
    assert.equal(deviceReviewPriority({ ...base, condition: "current" }), 3);
  });

});
