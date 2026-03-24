import { computed, onBeforeUnmount, ref, shallowRef, watch } from "vue";
import { isTauriRuntime } from "@/lib/bootstrap";
import { createWorkbenchApiClient, WorkbenchApiError } from "@/lib/workbenchApi";
import { useBootstrap } from "@/composables/useBootstrap";
import type {
  ClipboardItemDetail,
  ClipboardItemSummary,
  HealthResponse,
  MobileSummaryCounts,
  SessionResponse,
  WorkbenchNotice,
} from "@/types/workbench";
import type { AppBootstrap } from "@/types/bootstrap";

const BYTE_ENCODER = new TextEncoder();
const SHORT_NOTICE_MS = 2800;
const SEARCH_DEBOUNCE_MS = 280;

export function useMobileWorkbench() {
  const { bootstrap, loading: bootstrapLoading, error: bootstrapError, mobileEntryUrl } = useBootstrap();

  const search = ref("");
  const pinnedOnly = ref(false);
  const draft = ref("");
  const items = ref<ClipboardItemSummary[]>([]);
  const selectedItemId = ref<string | null>(null);
  const selectedDetail = ref<ClipboardItemDetail | null>(null);
  const session = ref<SessionResponse | null>(null);
  const health = ref<HealthResponse | null>(null);
  const apiClient = shallowRef<ReturnType<typeof createWorkbenchApiClient> | null>(null);
  const notice = ref<WorkbenchNotice | null>(null);
  const loading = ref(true);
  const loadingItems = ref(false);
  const submitting = ref(false);
  const authState = ref<"idle" | "missing" | "valid" | "invalid">("idle");
  const serviceState = ref<"unknown" | "online" | "offline" | "failed">("unknown");
  const streamState = ref<"idle" | "connecting" | "live" | "reconnecting">("idle");
  const lastSyncedAt = ref<number | null>(null);
  const now = ref(Date.now());
  const draftBytes = computed(() => BYTE_ENCODER.encode(draft.value).length);
  const canSubmit = computed(
    () =>
      !submitting.value &&
      authState.value === "valid" &&
      serviceState.value === "online" &&
      draftBytes.value > 0,
  );

  const selectedSummary = computed<ClipboardItemSummary | null>(() => {
    if (selectedItemId.value) {
      return items.value.find((item) => item.id === selectedItemId.value) ?? null;
    }

    return items.value[0] ?? null;
  });

  const selectedItem = computed<ClipboardItemDetail | ClipboardItemSummary | null>(() => {
    return selectedDetail.value ?? selectedSummary.value;
  });

  const currentSession = computed(() => session.value ?? bootstrap.value?.services.session ?? null);
  const summaryCounts = computed<MobileSummaryCounts>(() => ({
    total: items.value.length,
    pinned: items.value.filter((item) => item.pinned).length,
    current: items.value.filter((item) => item.isCurrent).length,
  }));
  const sessionExpiryText = computed(() => {
    const current = currentSession.value;
    if (!current) {
      return "会话未就绪";
    }

    const remaining = current.expiresAt - now.value;
    if (remaining <= 0) {
      return "会话已过期";
    }

    const minutes = Math.floor(remaining / 60_000);
    const seconds = Math.floor((remaining % 60_000) / 1000);
    return minutes > 0 ? `${minutes} 分 ${String(seconds).padStart(2, "0")} 秒` : `${seconds} 秒`;
  });
  const accessUrl = computed(() => {
    if (session.value?.accessUrl) {
      return session.value.accessUrl;
    }

    if (typeof window !== "undefined" && !isTauriRuntime()) {
      return window.location.href;
    }

    return bootstrap.value?.services.session.accessUrl ?? "";
  });
  const deviceName = computed(() => {
    if (session.value?.deviceName) {
      return session.value.deviceName;
    }

    if (isTauriRuntime()) {
      return bootstrap.value?.services.network.deviceName ?? "LocalShare";
    }

    return "移动设备";
  });
  const serviceLabel = computed(() => {
    if (health.value) {
      return health.value.status;
    }

    return bootstrap.value?.services.httpServer.state ?? "starting";
  });
  const statusTone = computed(() => {
    if (authState.value === "invalid") {
      return "error";
    }

    if (authState.value === "missing" || serviceState.value === "offline" || serviceState.value === "failed") {
      return "warning";
    }

    return "success";
  });
  const statusText = computed(() => {
    if (authState.value === "invalid") {
      return "会话失效";
    }

    if (authState.value === "missing") {
      return "等待扫码";
    }

    if (serviceState.value === "offline") {
      return "服务离线";
    }

    if (serviceState.value === "failed") {
      return "服务异常";
    }

    if (streamState.value === "live") {
      return "实时同步";
    }

    if (streamState.value === "connecting") {
      return "同步连接中";
    }

    return "可用";
  });

  let searchTimer: number | null = null;
  let noticeTimer: number | null = null;
  let eventSource: EventSource | null = null;
  let initialSessionKey = "";
  const ticker = typeof window !== "undefined"
    ? window.setInterval(() => {
        now.value = Date.now();
      }, 15_000)
    : null;

  function resolveToken(currentBootstrap: AppBootstrap | null) {
    const tokenKey = currentBootstrap?.services.session.tokenQueryKey ?? "token";
    const locationToken = getTokenFromLocation(tokenKey);
    if (locationToken) {
      return locationToken;
    }

    if (isTauriRuntime()) {
      return getTokenFromUrl(currentBootstrap?.services.session.accessUrl ?? "", tokenKey);
    }

    return "";
  }

  function resolveApiOrigin(currentBootstrap: AppBootstrap | null) {
    if (!currentBootstrap) {
      return "";
    }

    if (isTauriRuntime()) {
      return new URL(currentBootstrap.services.session.accessUrl).origin;
    }

    return typeof window !== "undefined" ? window.location.origin : "";
  }

  function getTokenFromLocation(tokenKey: string) {
    if (typeof window === "undefined") {
      return "";
    }

    return new URLSearchParams(window.location.search).get(tokenKey)?.trim() ?? "";
  }

  function getTokenFromUrl(url: string, tokenKey: string) {
    try {
      return new URL(url).searchParams.get(tokenKey)?.trim() ?? "";
    } catch {
      return "";
    }
  }

  function isWorkbenchApiError(error: unknown): error is WorkbenchApiError {
    return error instanceof WorkbenchApiError;
  }

  function isUnauthorized(error: WorkbenchApiError) {
    return error.code === "UNAUTHORIZED" || error.status === 401 || error.status === 403;
  }

  function clearNotice() {
    if (noticeTimer !== null && typeof window !== "undefined") {
      window.clearTimeout(noticeTimer);
    }
    noticeTimer = null;
    notice.value = null;
  }

  function pushNotice(kind: WorkbenchNotice["kind"], message: string, autoClear = true) {
    clearNotice();
    notice.value = { kind, message };

    if (autoClear && typeof window !== "undefined") {
      noticeTimer = window.setTimeout(() => {
        notice.value = null;
      }, SHORT_NOTICE_MS);
    }
  }

  function closeEventSource() {
    if (eventSource) {
      eventSource.close();
      eventSource = null;
    }
    streamState.value = authState.value === "valid" ? "reconnecting" : "idle";
  }

  async function loadHealth() {
    if (!apiClient.value) {
      return;
    }

    try {
      health.value = await apiClient.value.health();
      serviceState.value = health.value.status === "failed" ? "failed" : "online";
    } catch (error) {
      health.value = null;
      serviceState.value = "offline";
      if (isWorkbenchApiError(error)) {
        pushNotice("warning", `服务状态不可用: ${error.message}`);
      } else {
        pushNotice("warning", "服务状态不可用，请检查局域网连接");
      }
      throw error;
    }
  }

  async function loadSession() {
    if (!apiClient.value) {
      return;
    }

    try {
      session.value = await apiClient.value.session();
      authState.value = "valid";
    } catch (error) {
      if (isWorkbenchApiError(error) && isUnauthorized(error)) {
        authState.value = "invalid";
        closeEventSource();
        pushNotice("error", "会话已失效，请重新扫码打开");
      } else {
        pushNotice("warning", "会话信息加载失败，请稍后重试");
      }
      throw error;
    }
  }

  async function loadSelectedDetail(itemId: string) {
    if (!apiClient.value || authState.value !== "valid") {
      return;
    }

    try {
      const detail = await apiClient.value.getClipboardItem(itemId);
      if (selectedItemId.value === itemId) {
        selectedDetail.value = detail;
      }
    } catch (error) {
      if (isWorkbenchApiError(error) && isUnauthorized(error)) {
        authState.value = "invalid";
        closeEventSource();
        pushNotice("error", "会话已失效，请重新扫码打开");
      } else if (isWorkbenchApiError(error)) {
        pushNotice("warning", error.message);
      }
    }
  }

  async function loadItems(options: { keepSelection?: boolean } = {}) {
    if (!apiClient.value || authState.value !== "valid") {
      return;
    }

    loadingItems.value = true;
    try {
      const response = await apiClient.value.listClipboardItems({
        search: search.value.trim() || undefined,
        pinnedOnly: pinnedOnly.value,
        limit: 80,
      });

      items.value = response.items;
      const previousSelection = selectedItemId.value;

      if (options.keepSelection && previousSelection) {
        const stillVisible = response.items.some((item) => item.id === previousSelection);
        selectedItemId.value = stillVisible ? previousSelection : response.items[0]?.id ?? null;
      } else {
        selectedItemId.value = response.items[0]?.id ?? null;
      }

      if (selectedItemId.value) {
        void loadSelectedDetail(selectedItemId.value);
      } else {
        selectedDetail.value = null;
      }

      lastSyncedAt.value = Date.now();
    } catch (error) {
      if (isWorkbenchApiError(error) && isUnauthorized(error)) {
        authState.value = "invalid";
        closeEventSource();
        pushNotice("error", "会话已失效，请重新扫码打开");
      } else if (isWorkbenchApiError(error)) {
        pushNotice("warning", error.message);
      } else {
        pushNotice("warning", "历史列表同步失败");
      }
      throw error;
    } finally {
      loadingItems.value = false;
    }
  }

  function connectEventStream() {
    closeEventSource();
    if (!apiClient.value || authState.value !== "valid") {
      return;
    }

    try {
      eventSource = new EventSource(apiClient.value.eventsUrl());
      streamState.value = "connecting";

      eventSource.onopen = () => {
        streamState.value = "live";
      };

      eventSource.addEventListener("ready", () => {
        streamState.value = "live";
      });

      eventSource.addEventListener("refresh", () => {
        void loadItems({ keepSelection: true });
      });

      eventSource.onerror = () => {
        if (authState.value === "valid") {
          streamState.value = "reconnecting";
        }
      };
    } catch {
      streamState.value = "idle";
    }
  }

  async function initializeWorkspace(currentBootstrap: AppBootstrap | null) {
    if (!currentBootstrap) {
      return;
    }

    const sessionKey = currentBootstrap.services.session.sessionId;
    if (sessionKey === initialSessionKey && apiClient.value) {
      return;
    }

    initialSessionKey = sessionKey;
    clearNotice();
    closeEventSource();
    session.value = null;
    health.value = null;
    selectedDetail.value = null;
    items.value = [];
    selectedItemId.value = null;
    authState.value = "idle";
    serviceState.value = "unknown";
    streamState.value = "idle";
    loading.value = true;

    const token = resolveToken(currentBootstrap);
    if (!token) {
      authState.value = "missing";
      loading.value = false;
      pushNotice("info", "请使用带 token 的二维码链接打开移动端页面", false);
      return;
    }

    const origin = resolveApiOrigin(currentBootstrap);
    if (!origin) {
      loading.value = false;
      pushNotice("warning", "无法解析局域网服务地址", false);
      return;
    }

    apiClient.value = createWorkbenchApiClient({
      origin,
      token,
      tokenQueryKey: currentBootstrap.services.session.tokenQueryKey,
    });

    try {
      await loadHealth();
      if (health.value?.status === "failed") {
        pushNotice("warning", "服务当前处于异常状态，请在桌面端检查端口与权限");
        return;
      }

      await loadSession();
      await loadItems({ keepSelection: false });
      connectEventStream();
      pushNotice("success", "历史已同步");
    } catch (error) {
      if (isWorkbenchApiError(error) && isUnauthorized(error)) {
        // Session state is already updated by the lower-level loader.
        return;
      }

      if (!isWorkbenchApiError(error)) {
        pushNotice("warning", "移动端工作台初始化失败");
      }
    } finally {
      loading.value = false;
    }
  }

  function scheduleReload() {
    if (!apiClient.value || authState.value !== "valid") {
      return;
    }

    if (searchTimer !== null && typeof window !== "undefined") {
      window.clearTimeout(searchTimer);
    }

    if (typeof window === "undefined") {
      void loadItems({ keepSelection: false });
      return;
    }

    searchTimer = window.setTimeout(() => {
      void loadItems({ keepSelection: false });
    }, SEARCH_DEBOUNCE_MS);
  }

  async function refreshNow() {
    if (!apiClient.value || authState.value !== "valid") {
      return;
    }

    try {
      await loadHealth();
      await loadSession();
      await loadItems({ keepSelection: true });
      connectEventStream();
      pushNotice("success", "已刷新历史");
    } catch {
      // Lower-level loaders already surfaced the reason.
    }
  }

  async function submitDraft() {
    const content = draft.value.trim();
    if (!content) {
      pushNotice("warning", "空内容不可提交");
      return;
    }

    const maxBytes = bootstrap.value?.runtimeConfig.maxTextBytes ?? 0;
    if (maxBytes > 0 && BYTE_ENCODER.encode(content).length > maxBytes) {
      pushNotice("warning", `文本过长，已超过 ${maxBytes} 字节限制`);
      return;
    }

    if (!apiClient.value || authState.value !== "valid") {
      pushNotice("warning", "当前会话不可用，请重新扫码");
      return;
    }

    submitting.value = true;
    try {
      const response = await apiClient.value.submitClipboardItem({
        content,
        pinned: false,
        activate: false,
      });

      draft.value = "";
      selectedItemId.value = response.item.id;
      selectedDetail.value = response.item;
      pushNotice(response.created ? "success" : "info", response.created ? "文本已提交" : "文本已复用");
      await loadItems({ keepSelection: true });

      if (typeof window !== "undefined") {
        window.scrollTo({ top: 0, behavior: "smooth" });
      }
    } catch (error) {
      if (isWorkbenchApiError(error) && isUnauthorized(error)) {
        authState.value = "invalid";
        closeEventSource();
        pushNotice("error", "会话已失效，请重新扫码打开");
      } else if (isWorkbenchApiError(error)) {
        pushNotice("warning", error.message);
      } else {
        pushNotice("warning", "文本提交失败");
      }
    } finally {
      submitting.value = false;
    }
  }

  async function activateItem(item: ClipboardItemSummary) {
    if (!apiClient.value || authState.value !== "valid") {
      pushNotice("warning", "当前会话不可用，请重新扫码");
      return;
    }

    try {
      const detail = await apiClient.value.activateClipboardItem(item.id);
      selectedItemId.value = detail.id;
      selectedDetail.value = detail;
      pushNotice("success", "内容已激活到桌面剪贴板");
      await loadItems({ keepSelection: true });
    } catch (error) {
      if (isWorkbenchApiError(error) && isUnauthorized(error)) {
        authState.value = "invalid";
        closeEventSource();
        pushNotice("error", "会话已失效，请重新扫码打开");
      } else if (isWorkbenchApiError(error)) {
        pushNotice("warning", error.message);
      } else {
        pushNotice("warning", "激活失败");
      }
    }
  }

  function selectItem(itemId: string) {
    selectedItemId.value = itemId;
    void loadSelectedDetail(itemId);
  }

  function formatTimestamp(timestamp: number) {
    return new Intl.DateTimeFormat("zh-CN", {
      month: "2-digit",
      day: "2-digit",
      hour: "2-digit",
      minute: "2-digit",
    }).format(new Date(timestamp));
  }

  function formatSourceKind(value: string) {
    if (value === "mobile_web") {
      return "手机提交";
    }

    if (value === "desktop_local") {
      return "桌面复制";
    }

    return value;
  }

  watch(
    bootstrap,
    (value) => {
      void initializeWorkspace(value);
    },
    { immediate: true },
  );

  watch([search, pinnedOnly], () => {
    if (loading.value || authState.value !== "valid") {
      return;
    }

    scheduleReload();
  });

  onBeforeUnmount(() => {
    if (ticker !== null && typeof window !== "undefined") {
      window.clearInterval(ticker);
    }
    if (searchTimer !== null && typeof window !== "undefined") {
      window.clearTimeout(searchTimer);
    }
    if (noticeTimer !== null && typeof window !== "undefined") {
      window.clearTimeout(noticeTimer);
    }
    closeEventSource();
  });

  return {
    bootstrap,
    bootstrapLoading,
    bootstrapError,
    mobileEntryUrl,
    search,
    pinnedOnly,
    draft,
    items,
    selectedItem,
    selectedSummary,
    selectedDetail,
    selectedItemId,
    session,
    health,
    notice,
    loading,
    loadingItems,
    submitting,
    authState,
    serviceState,
    streamState,
    lastSyncedAt,
    draftBytes,
    canSubmit,
    sessionExpiryText,
    accessUrl,
    deviceName,
    serviceLabel,
    summaryCounts,
    statusTone,
    statusText,
    formatTimestamp,
    formatSourceKind,
    selectItem,
    activateItem,
    submitDraft,
    refreshNow,
  };
}
