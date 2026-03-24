import { invoke } from "@tauri-apps/api/core";
import { desktopRoute, mobileRoute } from "@/config/runtime";
import type { AppBootstrap } from "@/types/bootstrap";

const bootstrapFallback: AppBootstrap = {
  appName: "LocalShare",
  routes: {
    desktop: desktopRoute,
    mobile: mobileRoute,
  },
  runtimeConfig: {
    lanHost: "0.0.0.0",
    preferredPort: 8765,
    maxTextBytes: 65536,
    clipboardPollIntervalMs: 800,
    tokenTtlMinutes: 30,
    databaseFileName: "localshare.db",
    mobileRoute: mobileRoute,
  },
  paths: {
    appDir: "~/.localshare",
    dataDir: "~/.localshare/data",
    databasePath: "~/.localshare/data/localshare.db",
    logsDir: "~/.localshare/logs",
  },
  services: {
    clipboard: {
      mode: "polling",
      pollIntervalMs: 800,
      dedupWindowMs: 1600,
      maxTextBytes: 65536,
      currentItemTracking: true,
      running: false,
      subscriberCount: 0,
      refreshEventTopic: "localshare://clipboard/refresh",
    },
    httpServer: {
      bindHost: "0.0.0.0",
      preferredPort: 8765,
      healthEndpoint: "/api/v1/health",
      mobileBasePath: mobileRoute,
      sseEndpoint: "/api/v1/events",
      effectivePort: 8765,
      state: "running",
      lastError: null,
    },
    auth: {
      tokenTtlMinutes: 30,
      rotationEnabled: true,
      bearerHeaderName: "Authorization",
    },
    session: {
      sessionId: "bootstrap-session",
      expiresAt: Date.now() + 30 * 60 * 1000,
      status: "active",
      accessUrl: `http://localhost:8765${mobileRoute}?token=bootstrap-token`,
      publicHost: "localhost",
      publicPort: 8765,
      mobileBasePath: mobileRoute,
      tokenTtlMinutes: 30,
      bearerHeaderName: "Authorization",
      tokenQueryKey: "token",
    },
    persistence: {
      databasePath: "~/.localshare/data/localshare.db",
      migrationsEnabled: true,
      schemaVersion: 0,
      ready: true,
    },
    network: {
      deviceName: "desktop-host",
      accessHost: "localhost",
      accessHosts: ["localhost"],
      lanDiscoveryEnabled: true,
    },
  },
};

export function isTauriRuntime() {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

export async function loadBootstrapContext() {
  if (!isTauriRuntime()) {
    return bootstrapFallback;
  }

  try {
    return await invoke<AppBootstrap>("get_bootstrap_context");
  } catch (error) {
    console.warn("Failed to load Tauri bootstrap context, falling back to web scaffold.", error);
    return bootstrapFallback;
  }
}
