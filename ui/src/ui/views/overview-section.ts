import { html, nothing, type TemplateResult } from "lit";
import type { UiSettings } from "../storage.ts";

export const OVERVIEW_SECTION_IDS = [
  "access",
  "snapshot",
  "recent",
  "attention",
  "eventLog",
  "logTail",
] as const;

export type OverviewSectionId = (typeof OVERVIEW_SECTION_IDS)[number];

export function defaultOverviewSectionOpen(
  section: OverviewSectionId,
  connected: boolean,
): boolean {
  switch (section) {
    case "access":
      return !connected;
    case "snapshot":
      return true;
    case "recent":
    case "eventLog":
    case "logTail":
      return false;
    case "attention":
      return true;
    default:
      return false;
  }
}

export function isOverviewSectionOpen(
  collapsed: Record<string, boolean> | undefined,
  section: OverviewSectionId,
  connected: boolean,
): boolean {
  if (collapsed && Object.hasOwn(collapsed, section)) {
    return collapsed[section] !== true;
  }
  return defaultOverviewSectionOpen(section, connected);
}

export function setOverviewSectionOpen(
  settings: UiSettings,
  section: OverviewSectionId,
  open: boolean,
): UiSettings {
  return {
    ...settings,
    overviewSectionsCollapsed: {
      ...(settings.overviewSectionsCollapsed ?? {}),
      [section]: !open,
    },
  };
}

export function renderOverviewSection(params: {
  sectionId: OverviewSectionId;
  title: string;
  subtitle?: string;
  open: boolean;
  onToggle: (sectionId: OverviewSectionId, open: boolean) => void;
  className?: string;
  bodyClass?: string;
  children: TemplateResult;
}) {
  return html`
    <details
      class="card ov-section ${params.className ?? ""}"
      ?open=${params.open}
      @toggle=${(e: Event) => {
        const target = e.target as HTMLDetailsElement;
        if (target !== e.currentTarget) {
          return;
        }
        params.onToggle(params.sectionId, target.open);
      }}
    >
      <summary class="ov-expandable-toggle ov-section__summary">
        <span class="ov-section__titles">
          <span class="ov-section__title">${params.title}</span>
          ${params.subtitle
            ? html`<span class="ov-section__sub">${params.subtitle}</span>`
            : nothing}
        </span>
      </summary>
      <div class="ov-section__body ${params.bodyClass ?? ""}">${params.children}</div>
    </details>
  `;
}
