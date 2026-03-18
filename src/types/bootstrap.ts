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
  maxTextBytes: number;
  currentItemTracking: boolean;
}

export interface HttpServerStatus {
  bindHost: string;
  preferredPort: number;
  healthEndpoint: string;
  mobileBasePath: string;
  sseEndpoint: string;
}

export interface AuthStatus {
  tokenTtlMinutes: number;
  rotationEnabled: boolean;
  bearerHeaderName: string;
}

export interface PersistenceStatus {
  databasePath: string;
  migrationsEnabled: boolean;
}

export interface NetworkStatus {
  deviceName: string;
  lanDiscoveryEnabled: boolean;
}

export interface ServiceOverview {
  clipboard: ClipboardStatus;
  httpServer: HttpServerStatus;
  auth: AuthStatus;
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
