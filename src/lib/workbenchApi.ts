import type {
  ApiEnvelope,
  ApiErrorPayload,
  ClipboardItemDetail,
  ClipboardListQuery,
  ClipboardListResponse,
  ClipboardWriteRequest,
  ClipboardWriteResponse,
  HealthResponse,
  SessionResponse,
} from "@/types/workbench";

export class WorkbenchApiError extends Error {
  public readonly code: string;
  public readonly status: number;
  public readonly ts: number | null;

  constructor(code: string, message: string, status: number, ts: number | null = null) {
    super(message);
    this.name = "WorkbenchApiError";
    this.code = code;
    this.status = status;
    this.ts = ts;
  }
}

export interface WorkbenchApiClient {
  health: () => Promise<HealthResponse>;
  session: () => Promise<SessionResponse>;
  listClipboardItems: (query?: ClipboardListQuery) => Promise<ClipboardListResponse>;
  getClipboardItem: (itemId: string) => Promise<ClipboardItemDetail>;
  submitClipboardItem: (input: ClipboardWriteRequest) => Promise<ClipboardWriteResponse>;
  activateClipboardItem: (itemId: string) => Promise<ClipboardItemDetail>;
  eventsUrl: () => string;
}

export interface WorkbenchApiClientConfig {
  origin: string;
  token: string;
  tokenQueryKey?: string;
}

export function createWorkbenchApiClient(config: WorkbenchApiClientConfig): WorkbenchApiClient {
  const origin = new URL(config.origin).origin;
  const tokenQueryKey = config.tokenQueryKey ?? "token";

  const withToken = (path: string) => {
    const url = new URL(path, origin);
    url.searchParams.set(tokenQueryKey, config.token);
    return url;
  };

  const request = async <T,>(path: string, init: RequestInit = {}): Promise<T> => {
    const response = await fetch(withToken(path), {
      ...init,
      headers: {
        Accept: "application/json",
        ...(init.body ? { "Content-Type": "application/json" } : {}),
        ...(init.headers ?? {}),
      },
    });

    const rawBody = await response.text();
    let payload: ApiEnvelope<T> | null = null;

    if (rawBody.trim().length > 0) {
      try {
        payload = JSON.parse(rawBody) as ApiEnvelope<T>;
      } catch (error) {
        throw new WorkbenchApiError(
          "INVALID_RESPONSE",
          `failed to parse API response: ${error instanceof Error ? error.message : String(error)}`,
          response.status,
        );
      }
    }

    if (!response.ok) {
      const apiError = payload?.error;
      if (apiError) {
        throw toWorkbenchApiError(apiError, response.status, payload?.ts ?? null);
      }

      throw new WorkbenchApiError(
        "HTTP_ERROR",
        `request failed with HTTP ${response.status}`,
        response.status,
        payload?.ts ?? null,
      );
    }

    if (!payload) {
      throw new WorkbenchApiError("EMPTY_RESPONSE", "empty API response", response.status);
    }

    if (!payload.ok) {
      throw toWorkbenchApiError(
        payload.error ?? { code: "API_ERROR", message: "request failed" },
        response.status,
        payload.ts,
      );
    }

    return payload.data as T;
  };

  return {
    health: () => request<HealthResponse>("/api/v1/health"),
    session: () => request<SessionResponse>("/api/v1/session"),
    listClipboardItems: (query = {}) => {
      const url = new URL("/api/v1/clipboard-items", origin);
      url.searchParams.set(tokenQueryKey, config.token);

      if (query.search && query.search.trim().length > 0) {
        url.searchParams.set("search", query.search.trim());
      }

      if (query.pinnedOnly) {
        url.searchParams.set("pinnedOnly", "true");
      }

      if (typeof query.limit === "number") {
        url.searchParams.set("limit", String(query.limit));
      }

      return request<ClipboardListResponse>(url.pathname + url.search);
    },
    getClipboardItem: (itemId: string) =>
      request<ClipboardItemDetail>(`/api/v1/clipboard-items/${encodeURIComponent(itemId)}`),
    submitClipboardItem: (input: ClipboardWriteRequest) =>
      request<ClipboardWriteResponse>("/api/v1/clipboard-items", {
        method: "POST",
        body: JSON.stringify(input),
      }),
    activateClipboardItem: (itemId: string) =>
      request<ClipboardItemDetail>(`/api/v1/clipboard-items/${encodeURIComponent(itemId)}/activate`, {
        method: "POST",
      }),
    eventsUrl: () => withToken("/api/v1/events").toString(),
  };
}

function toWorkbenchApiError(
  error: ApiErrorPayload,
  status: number,
  ts: number | null,
): WorkbenchApiError {
  return new WorkbenchApiError(error.code, error.message, status, ts);
}
