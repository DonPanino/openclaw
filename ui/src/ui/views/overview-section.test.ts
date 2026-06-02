import { describe, expect, it } from "vitest";
import {
  defaultOverviewSectionOpen,
  isOverviewSectionOpen,
  setOverviewSectionOpen,
} from "./overview-section.ts";

describe("overview section collapse", () => {
  it("opens access when disconnected and collapses when connected by default", () => {
    expect(defaultOverviewSectionOpen("access", false)).toBe(true);
    expect(defaultOverviewSectionOpen("access", true)).toBe(false);
  });

  it("persists collapsed state in settings", () => {
    const next = setOverviewSectionOpen(
      {
        gatewayUrl: "",
        token: "",
        sessionKey: "main",
        lastActiveSessionKey: "main",
        theme: "claw",
        themeMode: "system",
        chatShowThinking: true,
        chatShowToolCalls: true,
        splitRatio: 0.6,
        navCollapsed: false,
        navWidth: 220,
        navGroupsCollapsed: {},
        borderRadius: 50,
      },
      "snapshot",
      false,
    );
    expect(isOverviewSectionOpen(next.overviewSectionsCollapsed, "snapshot", true)).toBe(false);
  });
});
