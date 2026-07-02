import { describe, expect, it } from "vitest";
import { formatHealthStatus } from "./health";

describe("formatHealthStatus", () => {
  it("shows an online label when the API is healthy", () => {
    expect(formatHealthStatus({ status: "ok", service: "drive-clone-api" })).toBe(
      "API online",
    );
  });

  it("shows a degraded label for unexpected health payloads", () => {
    expect(formatHealthStatus({ status: "unknown", service: "drive-clone-api" })).toBe(
      "API degraded",
    );
  });
});
