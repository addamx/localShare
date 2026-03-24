export interface RuntimeConfig {
  lanHost: string;
  preferredPort: number;
  maxTextBytes: number;
  clipboardPollIntervalMs: number;
  tokenTtlMinutes: number;
  databaseFileName: string;
  mobileRoute: string;
}

export interface AppPaths {
  appDir: string;
  dataDir: string;
  databasePath: string;
  logsDir: string;
}

export interface ClipboardStatus {
  mode: string;
  pollIntervalMs: number;
  dedupWindowMs?: number;
  maxTextBytes: number;
  currentItemTracking: boolean;
  running?: boolean;
  subscriberCount?: number;
  refreshEventTopic?: string;
}

export interface HttpServerStatus {
  bindHost: string;
  preferredPort: number;
  healthEndpoint: string;
  mobileBasePath: string;
  sseEndpoint: string;
  effectivePort?: number | null;
  state?: string;
  lastError?: string | null;
}

export interface AuthStatus {
  tokenTtlMinutes: number;
  rotationEnabled: boolean;
  bearerHeaderName: string;
}

export interface PersistenceStatus {
  databasePath: string;
  migrationsEnabled: boolean;
  schemaVersion?: number;
  ready?: boolean;
}

export interface NetworkStatus {
  deviceName: string;
  accessHost: string;
  accessHosts: string[];
  lanDiscoveryEnabled: boolean;
}

export interface SessionSnapshot {
  sessionId: string;
  expiresAt: number;
  status: "active" | "rotated" | "expired";
  accessUrl: string;
  publicHost: string;
  publicPort: number;
  mobileBasePath: string;
  tokenTtlMinutes: number;
  bearerHeaderName: string;
  tokenQueryKey: string;
}

export interface ServiceOverview {
  clipboard: ClipboardStatus;
  httpServer: HttpServerStatus;
  auth: AuthStatus;
  session: SessionSnapshot;
  persistence: PersistenceStatus;
  network: NetworkStatus;
}

export interface RouteOverview {
  desktop: string;
  mobile: string;
}

export interface AppBootstrap {
  appName: string;
  routes: RouteOverview;
  runtimeConfig: RuntimeConfig;
  paths: AppPaths;
  services: ServiceOverview;
}
