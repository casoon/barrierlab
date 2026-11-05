/** Ergebnis eines Prüfers: wie sicher die Aussage ist. */
export type Outcome = "fail" | "review" | "pass" | "untested";

export type Severity = "low" | "medium" | "high" | "critical";

export type WcagLevel = "A" | "AA" | "AAA";

/** Warum eine Regel nicht gelaufen ist. */
export type NotRun = "capability_missing" | "disabled" | "not_applicable" | "errored";

export interface Location {
  file?: string;
  url?: string;
  selector?: string;
  node?: string;
  source_hint?: string;
}

export interface Evidence {
  source: string;
  field?: string;
  value?: string;
}

export interface Finding {
  rule_id: string;
  rule_name?: string;
  outcome: Outcome;
  severity: Severity;
  message: string;
  location?: Location;
  role?: string;
  name?: string;
  wcag?: string[];
  wcag_level?: WcagLevel;
  tags?: string[];
  help?: string;
  help_url?: string;
  suggestion?: string;
  suggested_code?: string;
  snippet?: string;
  evidence?: Evidence[];
}

export interface RuleRun {
  rule_id: string;
  viewport?: string;
  wcag?: string[];
  not_run?: NotRun;
  findings: number;
  reason?: string;
}

export interface Summary {
  fail: number;
  review: number;
  pass: number;
  untested: number;
  critical: number;
  high: number;
  medium: number;
  low: number;
  rules_not_run: number;
}

export type InputKind =
  | "stylesheet_unavailable"
  | "stylesheet_unresolved"
  | "document_incomplete"
  | "other";

/** Vermerk über unvollständige Eingaben; vom Host eingetragen, von keiner Regel gelesen. */
export interface InputNote {
  kind: InputKind;
  subject?: string;
  detail?: string;
}

/** Der Bericht von `Scan.run()`; spiegelt die Serialisierung von `a11y-report`. */
export interface Report {
  /** Version des JSON-Vertrags (`SCHEMA_VERSION`, derzeit 1). */
  schema: number;
  findings: Finding[];
  rule_runs?: RuleRun[];
  inputs?: InputNote[];
  summary: Summary;
}
