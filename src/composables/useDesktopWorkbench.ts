import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import * as QRCode from "qrcode";
import { computed, onBeforeUnmount, onMounted, reactive, ref, watch, type Ref } from "vue";

import type { AppBootstrap, SessionSnapshot } from "@/types/bootstrap";
import type {
  ClipboardItemRecord,
  ClipboardListQuery,
  ClipboardRefreshEvent,
  ConnectivityReport,
  DesktopEntryCandidate,
  DesktopStatusChip,
  DesktopWorkbenchState,
  WorkbenchNotice,
} from "@/types/workbench";

const CLIPBOARD_REFRESH_EVENT = "localshare://clipboard/refresh";
const DEFAULT_HISTORY_LIMIT = 80;
const SEARCH_DEBOUNCE_MS = 260;
const BANNER_TTL_MS = 2400;

function normalizeAccessUrl(accessUrl: string, effectivePort?: number | null) {
  if (!effectivePort) {
    return accessUrl;
  }

  try {
    const url = new URL(accessUrl);
    url.port = String(effectivePort);
    return url.toString();
  } catch {
    return accessUrl;
  }
}

function buildAccessUrlForHost(accessUrl: string, host: string, effectivePort?: number | null) {
  try {
    const url = new URL(normalizeAccessUrl(accessUrl, effectivePort));
    url.hostname = host;
    if (effectivePort) {
      url.port = String(effectivePort);
    }
    return url.toString();
  } catch {
    return accessUrl;
  }
}

export function useDesktopWorkbench(
  bootstrap: Ref<AppBootstrap | null>,
): DesktopWorkbenchState {
  const session = ref<SessionSnapshot | null>(null);
  const mobileEntryUrl = ref("");
  const mobileEntryCandidates = ref<DesktopEntryCandidate[]>([]);
  const connectivityReport = ref<ConnectivityReport | null>(null);
  const qrCodeDataUrl = ref("");
  const qrCodeError = ref("");
  const loadingInitial = ref(true);
  const loadingHistory = ref(false);
  const loadingDetail = ref(false);
  const busyAction = ref<string | null>(null);
  const busyItemId = ref<string | null>(null);
  const banner = ref<WorkbenchNotice | null>(null);
  const listError = ref("");
  const detailError = ref("");
  const historySearch = ref("");
  const pinnedOnly = ref(false);
  const items = ref<ClipboardItemRecord[]>([]);
  const selectedItemId = ref<string | null>(null);
  const selectedItem = ref<ClipboardItemRecord | null>(null);
  const nowMs = ref(Date.now());

  let bannerTimer: number | null = null;
  let searchTimer: number | null = null;
  let clockTimer: number | null = null;
  let unlistenClipboardRefresh: UnlistenFn | null = null;

  const serverIssue = computed(() => {
    const server = bootstrap.value?.services.httpServer;
    if (!server) {
      return "";
    }

    if (server.state && server.state !== "running") {
      return server.lastError || `HTTP server is ${server.state}`;
    }

    if (server.lastError) {
      return server.lastError;
    }

    return "";
  });

  const tokenExpiryLabel = computed(() => {
    if (!session.value) {
      return "not ready";
    }

    const remaining = session.value.expiresAt - nowMs.value;
    if (remaining <= 0) {
      return "expired";
    }

    const totalSeconds = Math.floor(remaining / 1000);
    const minutes = Math.floor(totalSeconds / 60);
    const seconds = totalSeconds % 60;

    if (minutes >= 60) {
      const hours = Math.floor(minutes / 60);
      return `${hours}h ${minutes % 60}m`;
    }

    if (minutes > 0) {
      return `${minutes}m ${String(seconds).padStart(2, "0")}s`;
    }

    return `${totalSeconds}s`;
  });

  const currentTimeLabel = computed(() =>
    new Intl.DateTimeFormat("zh-CN", {
      hour: "2-digit",
      minute: "2-digit",
      second: "2-digit",
    }).format(new Date(nowMs.value)),
  );

  const statusChips = computed<DesktopStatusChip[]>(() => {
    const httpStatus = bootstrap.value?.services.httpServer;
    const clipboardStatus = bootstrap.value?.services.clipboard;
    const networkStatus = bootstrap.value?.services.network;
    const sessionStatus = session.value;

    return [
      {
        label: "设备",
        value: networkStatus?.deviceName || "unknown",
      },
      {
        label: "局域网",
        value: httpStatus
          ? `${httpStatus.bindHost}:${httpStatus.effectivePort ?? httpStatus.preferredPort}`
          : "unknown",
      },
      {
        label: "服务",
        value: httpStatus?.state || "stopped",
      },
      {
        label: "刷新订阅",
        value: String(clipboardStatus?.subscriberCount ?? 0),
      },
      {
        label: "会话",
        value: sessionStatus ? sessionStatus.status : "inactive",
      },
      {
        label: "到期",
        value: tokenExpiryLabel.value,
      },
    ];
  });

  watch(
    [
      () => bootstrap.value?.services.session,
      () => bootstrap.value?.services.httpServer.effectivePort,
      () => bootstrap.value?.services.network.accessHosts,
      () => bootstrap.value?.services.network.accessHost,
    ],
    ([next, effectivePort, accessHosts, preferredHost]) => {
      if (!next) {
        return;
      }

      const accessUrl = normalizeAccessUrl(next.accessUrl, effectivePort);
      const hosts = (accessHosts && accessHosts.length
        ? accessHosts
        : [preferredHost || next.publicHost]
      ).filter(Boolean) as string[];

      mobileEntryCandidates.value = hosts.map((host) => ({
        host,
        url: buildAccessUrlForHost(accessUrl, host, effectivePort),
        preferred: host === (preferredHost || next.publicHost),
      }));

      session.value = {
        ...next,
        accessUrl,
        publicPort: effectivePort ?? next.publicPort,
      };
      mobileEntryUrl.value =
        mobileEntryCandidates.value.find((item) => item.preferred)?.url ??
        mobileEntryCandidates.value[0]?.url ??
        accessUrl;
    },
    { immediate: true },
  );

  watch(
    [historySearch, pinnedOnly, session],
    () => {
      scheduleHistoryRefresh();
    },
    { immediate: true },
  );

  watch(
    () => session.value?.accessUrl,
    async (url) => {
      if (!url) {
        qrCodeDataUrl.value = "";
        return;
      }

      qrCodeError.value = "";
      try {
        qrCodeDataUrl.value = await QRCode.toDataURL(url, {
          errorCorrectionLevel: "M",
          margin: 1,
          width: 320,
          color: {
            dark: "#0f172a",
            light: "#ffffff",
          },
        });
      } catch (error) {
        qrCodeDataUrl.value = "";
        qrCodeError.value = describeError(error);
      }
    },
    { immediate: true },
  );

  onMounted(async () => {
    clockTimer = window.setInterval(() => {
      nowMs.value = Date.now();
    }, 1000);

    try {
      unlistenClipboardRefresh = await listen<ClipboardRefreshEvent>(
        CLIPBOARD_REFRESH_EVENT,
        async (event) => {
          const itemId = event.payload?.itemId;
          await refreshHistory({ silent: true });

          if (itemId && selectedItemId.value === itemId) {
            const updated = items.value.find((item) => item.id === itemId);
            if (updated) {
              selectedItem.value = updated;
            }
          }
        },
      );
    } catch (error) {
      setBanner("订阅剪贴板刷新事件失败: " + describeError(error), "error");
    }

    await runConnectivityCheck(true);
    loadingInitial.value = false;
  });

  onBeforeUnmount(() => {
    if (bannerTimer) {
      window.clearTimeout(bannerTimer);
      bannerTimer = null;
    }

    if (searchTimer) {
      window.clearTimeout(searchTimer);
      searchTimer = null;
    }

    if (clockTimer) {
      window.clearInterval(clockTimer);
      clockTimer = null;
    }

    if (unlistenClipboardRefresh) {
      unlistenClipboardRefresh();
      unlistenClipboardRefresh = null;
    }
  });

  function scheduleHistoryRefresh() {
    if (searchTimer) {
      window.clearTimeout(searchTimer);
    }

    searchTimer = window.setTimeout(() => {
      void refreshHistory({ silent: true });
    }, SEARCH_DEBOUNCE_MS);
  }

  function buildHistoryQuery(): ClipboardListQuery {
    return {
      search: historySearch.value.trim() ? historySearch.value.trim() : null,
      pinnedOnly: pinnedOnly.value,
      includeDeleted: false,
      createdBefore: null,
      beforeId: null,
      limit: DEFAULT_HISTORY_LIMIT,
    };
  }

  function setBanner(text: string, kind: WorkbenchNotice["kind"] = "info") {
    banner.value = { message: text, kind };

    if (bannerTimer) {
      window.clearTimeout(bannerTimer);
    }

    if (kind !== "error") {
      bannerTimer = window.setTimeout(() => {
        if (banner.value?.message === text) {
          banner.value = null;
        }
      }, BANNER_TTL_MS);
    }
  }

  function describeError(error: unknown) {
    if (error instanceof Error) {
      return error.message;
    }

    if (typeof error === "string") {
      return error;
    }

    if (error && typeof error === "object" && "message" in error) {
      return String((error as { message: unknown }).message);
    }

    return String(error);
  }

  function syncSelection(nextItems: ClipboardItemRecord[]) {
    if (!nextItems.length) {
      selectedItemId.value = null;
      selectedItem.value = null;
      return;
    }

    const currentId = selectedItemId.value;
    const nextSelected =
      (currentId ? nextItems.find((item) => item.id === currentId) : null) || nextItems[0];

    selectedItemId.value = nextSelected.id;
    selectedItem.value = nextSelected;
  }

  async function refreshHistory(options?: { silent?: boolean }) {
    loadingHistory.value = true;
    listError.value = "";

    try {
      const nextItems = await invoke<ClipboardItemRecord[]>("list_clipboard_items", {
        query: buildHistoryQuery(),
      });
      items.value = nextItems;
      syncSelection(nextItems);

      if (!options?.silent) {
        setBanner("历史已刷新", "success");
      }
      return true;
    } catch (error) {
      const message = describeError(error);
      listError.value = message;
      setBanner(message, "error");
      return false;
    } finally {
      loadingHistory.value = false;
    }
  }

  async function selectItem(itemId: string) {
    selectedItemId.value = itemId;
    detailError.value = "";

    const existing = items.value.find((item) => item.id === itemId);
    if (existing) {
      selectedItem.value = existing;
      return;
    }

    loadingDetail.value = true;
    try {
      const next = await invoke<ClipboardItemRecord>("get_clipboard_item", {
        itemId,
      });
      selectedItem.value = next;
    } catch (error) {
      const message = describeError(error);
      detailError.value = message;
      setBanner(message, "error");
    } finally {
      loadingDetail.value = false;
    }
  }

  async function runItemAction(
    itemId: string,
    actionName: string,
    executor: () => Promise<ClipboardItemRecord | void>,
  ) {
    busyItemId.value = itemId;
    detailError.value = "";

    try {
      await executor();
      const refreshed = await refreshHistory({ silent: true });
      if (refreshed) {
        setBanner(`${actionName}已完成`, "success");
      }
    } catch (error) {
      const message = describeError(error);
      detailError.value = message;
      setBanner(message, "error");
    } finally {
      busyItemId.value = null;
    }
  }

  async function activateItem(itemId: string) {
    await runItemAction(itemId, "激活", async () => {
      selectedItem.value = await invoke<ClipboardItemRecord>("activate_clipboard_item", {
        itemId,
      });
    });
  }

  async function togglePin(itemId: string, pinned: boolean) {
    await runItemAction(itemId, pinned ? "置顶" : "取消置顶", async () => {
      selectedItem.value = await invoke<ClipboardItemRecord>("update_clipboard_item_pin", {
        itemId,
        pinned,
      });
    });
  }

  async function deleteItem(itemId: string) {
    if (!window.confirm("确定要删除这条历史记录吗？")) {
      return;
    }

    await runItemAction(itemId, "删除", async () => {
      await invoke("delete_clipboard_item", {
        itemId,
      });
      if (selectedItemId.value === itemId) {
        selectedItem.value = null;
        selectedItemId.value = null;
      }
    });
  }

  async function clearHistory() {
    if (!window.confirm("确定要清空所有历史记录吗？")) {
      return;
    }

    busyAction.value = "clear";
    try {
      await invoke("clear_clipboard_history");
      selectedItem.value = null;
      selectedItemId.value = null;
      const refreshed = await refreshHistory({ silent: true });
      if (refreshed) {
        setBanner("历史已清空", "success");
      }
    } catch (error) {
      const message = describeError(error);
      setBanner(message, "error");
    } finally {
      busyAction.value = null;
    }
  }

  async function rotateSession() {
    busyAction.value = "rotate";

    try {
      const next = await invoke<SessionSnapshot>("rotate_session_token");
      session.value = { ...next };
      mobileEntryUrl.value = next.accessUrl;
      await runConnectivityCheck(true);
      setBanner("会话已刷新", "success");
    } catch (error) {
      setBanner(describeError(error), "error");
    } finally {
      busyAction.value = null;
    }
  }

  async function copyMobileEntryUrl(url = mobileEntryUrl.value) {
    if (!url) {
      setBanner("移动端入口尚未就绪", "warning");
      return;
    }

    try {
      await navigator.clipboard.writeText(url);
      setBanner("访问链接已复制", "success");
    } catch (error) {
      setBanner(describeError(error), "error");
    }
  }

  async function runConnectivityCheck(silent = false) {
    try {
      connectivityReport.value = await invoke<ConnectivityReport>("get_connectivity_report");
      if (!silent) {
        setBanner("连接自检已刷新", "success");
      }
    } catch (error) {
      if (!silent) {
        setBanner(`连接自检失败: ${describeError(error)}`, "error");
      }
    }
  }

  return reactive({
    session,
    mobileEntryUrl,
    mobileEntryCandidates,
    connectivityReport,
    qrCodeDataUrl,
    qrCodeError,
    loadingInitial,
    loadingHistory,
    loadingDetail,
    busyAction,
    busyItemId,
    banner,
    listError,
    detailError,
    historySearch,
    pinnedOnly,
    items,
    selectedItemId,
    selectedItem,
    statusChips,
    serverIssue,
    tokenExpiryLabel,
    currentTimeLabel,
    refreshHistory,
    selectItem,
    activateItem,
    togglePin,
    deleteItem,
    clearHistory,
    rotateSession,
    copyMobileEntryUrl,
    runConnectivityCheck,
  });
}
