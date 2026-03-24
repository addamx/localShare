import type { SessionSnapshot } from "@/types/bootstrap";

export interface ApiErrorPayload {
  code: string;
  message: string;
}

export interface ApiEnvelope<T> {
  ok: boolean;
  data: T | null;
  error: ApiErrorPayload | null;
  ts: number;
}

export interface HealthResponse {
  service: string;
  status: string;
  bindHost: string;
  preferredPort: number;
  effectivePort: number | null;
  databaseReady: boolean;
  sessionReady: boolean;
  mobileBasePath: string;
  healthEndpoint: string;
  sseEndpoint: string;
}

export interface SessionResponse {
  deviceName: string;
  publicHost: string;
  publicPort: number;
  accessUrl: string;
  healthEndpoint: string;
  sseEndpoint: string;
  mobileBasePath: string;
  sessionId: string;
  sessionStatus: "active" | "rotated" | "expired";
  expiresAt: number;
  tokenTtlMinutes: number;
  bearerHeaderName: string;
  tokenQueryKey: string;
  rotationEnabled: boolean;
  maxTextBytes: number;
  readOnly: boolean;
}

export interface ClipboardItemSummary {
  id: string;
  preview: string;
  charCount: number;
  sourceKind: string;
  sourceDeviceId: string | null;
  pinned: boolean;
  isCurrent: boolean;
  deletedAt: number | null;
  createdAt: number;
  updatedAt: number;
}

export interface ClipboardItemDetail extends ClipboardItemSummary {
  content: string;
  contentType: string;
  hash: string;
}

export type ClipboardItemRecord = ClipboardItemDetail;

export interface ClipboardListResponse {
  items: ClipboardItemSummary[];
}

export interface ClipboardWriteRequest {
  content: string;
  pinned?: boolean;
  activate?: boolean;
}

export interface ClipboardWriteResponse {
  item: ClipboardItemDetail;
  created: boolean;
  reusedExisting: boolean;
}

export interface ClipboardListQuery {
  search: string | null;
  pinnedOnly: boolean;
  includeDeleted: boolean;
  createdBefore: number | null;
  beforeId: string | null;
  limit: number;
}

export interface ServerEvent {
  kind: string;
  scope: string;
  itemId: string | null;
  ts: number;
}

export interface ClipboardRefreshEvent {
  itemId: string;
  created: boolean;
  reusedExisting: boolean;
  isCurrent: boolean;
  sourceKind: string;
  observedAtMs: number;
}

export interface MobileSummaryCounts {
  total: number;
  pinned: number;
  current: number;
}

export type WorkbenchNoticeKind = "success" | "info" | "warning" | "error";

export interface WorkbenchNotice {
  kind: WorkbenchNoticeKind;
  message: string;
}

export interface DesktopStatusChip {
  label: string;
  value: string;
}

export interface DesktopEntryCandidate {
  host: string;
  url: string;
  preferred: boolean;
}

export interface ConnectivityCheck {
  host: string;
  url: string;
  tcpOk: boolean;
  httpOk: boolean;
  httpStatusLine: string | null;
  error: string | null;
}

export interface ConnectivityReport {
  bindHost: string;
  preferredPort: number;
  effectivePort: number;
  serverState: string;
  serverError: string | null;
  accessUrl: string;
  checks: ConnectivityCheck[];
}

export interface DesktopWorkbenchState {
  session: SessionSnapshot | null;
  mobileEntryUrl: string;
  mobileEntryCandidates: DesktopEntryCandidate[];
  connectivityReport: ConnectivityReport | null;
  qrCodeDataUrl: string;
  qrCodeError: string;
  loadingInitial: boolean;
  loadingHistory: boolean;
  loadingDetail: boolean;
  busyAction: string | null;
  busyItemId: string | null;
  banner: WorkbenchNotice | null;
  listError: string;
  detailError: string;
  historySearch: string;
  pinnedOnly: boolean;
  items: ClipboardItemDetail[];
  selectedItemId: string | null;
  selectedItem: ClipboardItemDetail | null;
  statusChips: DesktopStatusChip[];
  serverIssue: string;
  tokenExpiryLabel: string;
  currentTimeLabel: string;
  refreshHistory: (options?: { silent?: boolean }) => Promise<void>;
  selectItem: (itemId: string) => Promise<void>;
  activateItem: (itemId: string) => Promise<void>;
  togglePin: (itemId: string, pinned: boolean) => Promise<void>;
  deleteItem: (itemId: string) => Promise<void>;
  clearHistory: () => Promise<void>;
  rotateSession: () => Promise<void>;
  copyMobileEntryUrl: (url?: string) => Promise<void>;
  runConnectivityCheck: (silent?: boolean) => Promise<void>;
}
