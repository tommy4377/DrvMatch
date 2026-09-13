import { describe, it } from "node:test";
import assert from "node:assert/strict";
import {
  compatibilityLabel,
  formatBytes,
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
});
