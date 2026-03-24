<script setup lang="ts">
import { computed } from "vue";

import { useBootstrap } from "@/composables/useBootstrap";
import { useDesktopWorkbench } from "@/composables/useDesktopWorkbench";

const { bootstrap, loading: bootstrapLoading, error: bootstrapError } = useBootstrap();
const workbench = useDesktopWorkbench(bootstrap);

const bannerClass = computed(() => {
  const notice = workbench.banner;
  if (!notice) {
    return "";
  }

  return {
    info: "border border-slate-200 bg-slate-50 text-slate-700",
    neutral: "border border-slate-200 bg-slate-50 text-slate-700",
    success: "border border-emerald-200 bg-emerald-50 text-emerald-700",
    warning: "border border-amber-200 bg-amber-50 text-amber-700",
    error: "border border-rose-200 bg-rose-50 text-rose-700",
  }[notice.kind];
});

const serviceIssue = computed(() => {
  return (
    bootstrapError.value ||
    workbench.serverIssue ||
    workbench.listError ||
    workbench.detailError
  );
});

function formatDateTime(value: number) {
  return new Intl.DateTimeFormat("zh-CN", {
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  }).format(new Date(value));
}

function formatSource(kind: string) {
  if (kind === "desktop_local") {
    return "桌面端";
  }

  if (kind === "mobile_web") {
    return "手机端";
  }

  return kind;
}

function formatBytes(count: number) {
  return `${count} chars`;
}

function formatChipValue(value: string) {
  return value.length > 18 ? `${value.slice(0, 18)}...` : value;
}
</script>

<template>
  <main class="min-h-screen px-4 py-5 text-slate-950 xl:px-6 xl:py-6">
    <div class="mx-auto flex max-w-7xl flex-col gap-5">
      <section
        class="grid gap-5 rounded-[32px] border border-white/70 bg-white/82 p-6 shadow-[0_30px_90px_rgba(15,23,42,0.08)] backdrop-blur xl:grid-cols-[1.25fr_0.95fr]"
      >
        <div class="space-y-5">
          <div class="space-y-3">
            <p class="text-xs font-semibold uppercase tracking-[0.28em] text-emerald-700">
              Desktop Console
            </p>
            <h1 class="text-4xl font-semibold tracking-tight text-slate-950">
              {{ bootstrap?.appName ?? "LocalShare" }}
            </h1>
            <p class="max-w-3xl text-sm leading-7 text-slate-600">
              桌面端主工作台，负责历史查看、搜索、置顶、删除、清空、当前项回填和局域网会话入口。
            </p>
          </div>

          <div class="flex flex-wrap gap-2">
            <span
              v-for="chip in workbench.statusChips"
              :key="chip.label"
              class="inline-flex items-center gap-2 rounded-full border border-slate-200 bg-white/90 px-3 py-1 text-xs font-medium text-slate-600 shadow-sm"
            >
              <span class="text-slate-400">{{ chip.label }}</span>
              <span class="font-mono text-slate-900">{{ formatChipValue(chip.value) }}</span>
            </span>
          </div>

          <div
            v-if="serviceIssue"
            class="rounded-[24px] border border-rose-200 bg-rose-50/95 px-4 py-3 text-sm leading-6 text-rose-700"
          >
            <p class="font-semibold">服务提示</p>
            <p class="mt-1">{{ serviceIssue }}</p>
          </div>

          <div
            v-if="workbench.banner"
            :class="[
              bannerClass,
              'rounded-[24px] px-4 py-3 text-sm leading-6 shadow-sm transition-all duration-200',
            ]"
          >
            {{ workbench.banner.message }}
          </div>
        </div>

        <aside
          class="rounded-[28px] bg-slate-950 p-5 text-slate-50 shadow-[0_28px_70px_rgba(15,23,42,0.25)]"
        >
          <div class="flex items-start justify-between gap-4">
            <div>
              <p class="text-sm font-medium text-emerald-300">Mobile Entry</p>
              <h2 class="mt-1 text-lg font-semibold">Session QR</h2>
              <p class="mt-2 text-xs leading-5 text-slate-300">
                token expires in {{ workbench.tokenExpiryLabel }}
              </p>
            </div>

            <button
              class="rounded-full border border-white/10 bg-white/10 px-3 py-2 text-xs font-medium text-slate-100 transition hover:bg-white/15 disabled:cursor-not-allowed disabled:opacity-50"
              :disabled="workbench.busyAction === 'rotate'"
              @click="workbench.rotateSession"
            >
              刷新 token
            </button>
          </div>

          <div class="mt-4 grid gap-4 sm:grid-cols-[168px_1fr]">
            <div class="rounded-[24px] bg-white p-3">
              <img
                v-if="workbench.qrCodeDataUrl"
                :src="workbench.qrCodeDataUrl"
                alt="mobile session QR code"
                class="aspect-square w-full rounded-[18px] object-cover"
              />
              <div
                v-else
                class="flex aspect-square items-center justify-center rounded-[18px] border border-dashed border-slate-300 bg-slate-50 text-center text-xs text-slate-400"
              >
                {{ workbench.qrCodeError || "二维码生成中..." }}
              </div>
            </div>

            <div class="space-y-3">
              <div class="rounded-2xl border border-white/10 bg-white/5 p-3">
                <p class="text-xs uppercase tracking-[0.2em] text-slate-400">Access URL</p>
                <p class="mt-2 break-all font-mono text-sm leading-6 text-slate-100">
                  {{ workbench.mobileEntryUrl || "session not ready" }}
                </p>
              </div>

              <div
                v-if="workbench.mobileEntryCandidates.length > 1"
                class="rounded-2xl border border-white/10 bg-white/5 p-3"
              >
                <p class="text-xs uppercase tracking-[0.2em] text-slate-400">Available Hosts</p>
                <div class="mt-3 space-y-2">
                  <div
                    v-for="candidate in workbench.mobileEntryCandidates"
                    :key="candidate.host"
                    class="flex items-center justify-between gap-3 rounded-2xl bg-white/5 px-3 py-2"
                  >
                    <div class="min-w-0">
                      <p class="font-mono text-xs text-slate-100">
                        {{ candidate.host }}
                        <span
                          v-if="candidate.preferred"
                          class="ml-2 rounded-full bg-emerald-400/15 px-2 py-0.5 text-[10px] font-semibold uppercase tracking-[0.16em] text-emerald-300"
                        >
                          primary
                        </span>
                      </p>
                      <p class="mt-1 truncate font-mono text-[11px] text-slate-400">
                        {{ candidate.url }}
                      </p>
                    </div>
                    <button
                      class="shrink-0 rounded-full border border-white/10 bg-white/10 px-3 py-1.5 text-[11px] font-medium text-slate-100 transition hover:bg-white/15"
                      @click="workbench.copyMobileEntryUrl(candidate.url)"
                    >
                      复制
                    </button>
                  </div>
                </div>
              </div>

              <div class="flex flex-wrap gap-2">
                <button
                  class="rounded-full bg-emerald-400 px-3 py-2 text-xs font-semibold text-slate-950 transition hover:bg-emerald-300 disabled:cursor-not-allowed disabled:opacity-50"
                  :disabled="!workbench.mobileEntryUrl"
                  @click="workbench.copyMobileEntryUrl"
                >
                  复制链接
                </button>
                <button
                  class="rounded-full border border-white/10 bg-white/10 px-3 py-2 text-xs font-medium text-slate-100 transition hover:bg-white/15 disabled:cursor-not-allowed disabled:opacity-50"
                  :disabled="workbench.loadingHistory"
                  @click="workbench.refreshHistory()"
                >
                  刷新历史
                </button>
                <button
                  class="rounded-full border border-white/10 bg-white/10 px-3 py-2 text-xs font-medium text-slate-100 transition hover:bg-white/15"
                  @click="workbench.runConnectivityCheck()"
                >
                  连接自检
                </button>
              </div>

              <dl class="grid gap-2 text-xs text-slate-300">
                <div class="flex items-center justify-between rounded-2xl bg-white/5 px-3 py-2">
                  <dt>服务状态</dt>
                  <dd class="font-mono">{{ bootstrap?.services.httpServer.state ?? "unknown" }}</dd>
                </div>
                <div class="flex items-center justify-between rounded-2xl bg-white/5 px-3 py-2">
                  <dt>当前时间</dt>
                  <dd class="font-mono">{{ workbench.currentTimeLabel }}</dd>
                </div>
                <div class="flex items-center justify-between rounded-2xl bg-white/5 px-3 py-2">
                  <dt>到期倒计时</dt>
                  <dd class="font-mono">{{ workbench.tokenExpiryLabel }}</dd>
                </div>
              </dl>

              <div
                v-if="workbench.connectivityReport"
                class="rounded-2xl border border-white/10 bg-white/5 p-3"
              >
                <div class="flex items-center justify-between gap-3">
                  <p class="text-xs uppercase tracking-[0.2em] text-slate-400">Self Check</p>
                  <p class="font-mono text-[11px] text-slate-300">
                    {{ workbench.connectivityReport.serverState }} :{{
                      workbench.connectivityReport.effectivePort
                    }}
                  </p>
                </div>
                <div class="mt-3 space-y-2">
                  <div
                    v-for="check in workbench.connectivityReport.checks"
                    :key="check.host"
                    class="rounded-2xl bg-white/5 px-3 py-2"
                  >
                    <div class="flex items-center justify-between gap-3">
                      <p class="font-mono text-xs text-slate-100">{{ check.host }}</p>
                      <span
                        class="rounded-full px-2 py-0.5 text-[10px] font-semibold uppercase tracking-[0.16em]"
                        :class="
                          check.httpOk
                            ? 'bg-emerald-400/15 text-emerald-300'
                            : check.tcpOk
                              ? 'bg-amber-400/15 text-amber-300'
                              : 'bg-rose-400/15 text-rose-300'
                        "
                      >
                        {{ check.httpOk ? "http ok" : check.tcpOk ? "tcp only" : "failed" }}
                      </span>
                    </div>
                    <p class="mt-1 truncate font-mono text-[11px] text-slate-400">
                      {{ check.url }}
                    </p>
                    <p class="mt-1 text-[11px] text-slate-400">
                      {{ check.httpStatusLine || check.error || "no response" }}
                    </p>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </aside>
      </section>

      <section v-if="bootstrapLoading" class="rounded-[28px] border border-dashed border-slate-300 bg-white/70 p-6 text-sm text-slate-500">
        正在加载桌面工作台...
      </section>

      <section
        v-else-if="bootstrapError"
        class="rounded-[28px] border border-rose-200 bg-rose-50 p-6 text-sm leading-7 text-rose-700"
      >
        {{ bootstrapError }}
      </section>

      <section v-else class="grid gap-5 xl:grid-cols-[1.12fr_0.88fr]">
        <article class="rounded-[30px] border border-white/70 bg-white/88 p-5 shadow-[0_18px_60px_rgba(15,23,42,0.08)]">
          <div class="flex flex-wrap items-center justify-between gap-3">
            <div>
              <p class="text-xs font-semibold uppercase tracking-[0.2em] text-slate-500">History</p>
              <h2 class="mt-1 text-2xl font-semibold text-slate-950">剪贴板历史</h2>
            </div>

            <div class="flex flex-wrap gap-2">
              <button
                class="rounded-full border border-slate-200 bg-slate-950 px-4 py-2 text-sm font-medium text-white transition hover:bg-slate-800 disabled:cursor-not-allowed disabled:opacity-50"
                :disabled="workbench.loadingHistory"
                @click="workbench.refreshHistory()"
              >
                刷新
              </button>
              <button
                class="rounded-full border border-slate-200 bg-white px-4 py-2 text-sm font-medium text-slate-700 transition hover:bg-slate-50 disabled:cursor-not-allowed disabled:opacity-50"
                :disabled="workbench.busyAction !== null"
                @click="workbench.clearHistory"
              >
                清空
              </button>
            </div>
          </div>

          <div class="mt-4 flex flex-col gap-3 sm:flex-row">
            <label
              class="flex flex-1 items-center gap-3 rounded-2xl border border-slate-200 bg-white px-4 py-3 shadow-sm transition focus-within:border-emerald-300 focus-within:ring-2 focus-within:ring-emerald-100"
            >
              <span class="text-xs font-semibold uppercase tracking-[0.18em] text-slate-400">
                Search
              </span>
              <input
                v-model="workbench.historySearch"
                type="text"
                placeholder="搜索内容、预览或来源"
                class="w-full border-0 bg-transparent text-sm text-slate-900 outline-none placeholder:text-slate-400"
              />
            </label>

            <button
              class="rounded-2xl border px-4 py-3 text-sm font-medium transition"
              :class="
                workbench.pinnedOnly
                  ? 'border-emerald-300 bg-emerald-50 text-emerald-700'
                  : 'border-slate-200 bg-white text-slate-600 hover:bg-slate-50'
              "
              @click="workbench.pinnedOnly = !workbench.pinnedOnly"
            >
              仅看置顶
            </button>
          </div>

          <div
            v-if="workbench.loadingHistory"
            class="mt-5 rounded-[24px] border border-dashed border-slate-300 bg-slate-50 p-6 text-sm text-slate-500"
          >
            正在刷新历史...
          </div>

          <div v-else class="mt-5 space-y-3">
            <article
              v-for="item in workbench.items"
              :key="item.id"
              class="group cursor-pointer rounded-[26px] border px-4 py-4 transition duration-200"
              :class="
                workbench.selectedItemId === item.id
                  ? 'border-emerald-300 bg-emerald-50/60 shadow-[0_14px_40px_rgba(16,185,129,0.12)]'
                  : 'border-slate-200 bg-white hover:border-slate-300 hover:shadow-[0_14px_35px_rgba(15,23,42,0.06)]'
              "
              @click="workbench.selectItem(item.id)"
            >
              <div class="flex items-start justify-between gap-3">
                <div class="space-y-2">
                  <div class="flex flex-wrap gap-2 text-[11px] font-semibold uppercase tracking-[0.18em]">
                    <span
                      class="rounded-full px-2.5 py-1"
                      :class="item.pinned ? 'bg-amber-100 text-amber-700' : 'bg-slate-100 text-slate-600'"
                    >
                      {{ item.pinned ? "Pinned" : "History" }}
                    </span>
                    <span
                      class="rounded-full px-2.5 py-1"
                      :class="item.isCurrent ? 'bg-emerald-100 text-emerald-700' : 'bg-slate-100 text-slate-500'"
                    >
                      {{ item.isCurrent ? "Current" : "Snapshot" }}
                    </span>
                    <span class="rounded-full bg-slate-100 px-2.5 py-1 text-slate-500">
                      {{ formatSource(item.sourceKind) }}
                    </span>
                  </div>
                  <p class="max-w-3xl text-sm leading-6 text-slate-700">
                    {{ item.preview || item.content }}
                  </p>
                </div>

                <div class="flex shrink-0 flex-col items-end gap-2 text-xs text-slate-500">
                  <span class="font-mono">{{ formatDateTime(item.createdAt) }}</span>
                  <span>{{ formatBytes(item.charCount) }}</span>
                </div>
              </div>

              <div class="mt-3 flex flex-wrap gap-2">
                <button
                  class="rounded-full bg-slate-950 px-3 py-1.5 text-xs font-medium text-white transition hover:bg-slate-800 disabled:cursor-not-allowed disabled:opacity-50"
                  :disabled="workbench.busyItemId === item.id"
                  @click.stop="workbench.activateItem(item.id)"
                >
                  激活
                </button>
                <button
                  class="rounded-full border border-slate-200 bg-white px-3 py-1.5 text-xs font-medium text-slate-700 transition hover:bg-slate-50 disabled:cursor-not-allowed disabled:opacity-50"
                  :disabled="workbench.busyItemId === item.id"
                  @click.stop="workbench.togglePin(item.id, !item.pinned)"
                >
                  {{ item.pinned ? "取消置顶" : "置顶" }}
                </button>
                <button
                  class="rounded-full border border-rose-200 bg-rose-50 px-3 py-1.5 text-xs font-medium text-rose-700 transition hover:bg-rose-100 disabled:cursor-not-allowed disabled:opacity-50"
                  :disabled="workbench.busyItemId === item.id"
                  @click.stop="workbench.deleteItem(item.id)"
                >
                  删除
                </button>
              </div>
            </article>

            <div
              v-if="!workbench.items.length"
              class="rounded-[24px] border border-dashed border-slate-300 bg-slate-50 p-6 text-sm leading-7 text-slate-500"
            >
              当前筛选条件下没有历史记录。
            </div>
          </div>
        </article>

        <article class="rounded-[30px] border border-white/70 bg-white/88 p-5 shadow-[0_18px_60px_rgba(15,23,42,0.08)]">
          <div class="flex items-center justify-between gap-3">
            <div>
              <p class="text-xs font-semibold uppercase tracking-[0.2em] text-slate-500">Detail</p>
              <h2 class="mt-1 text-2xl font-semibold text-slate-950">内容详情</h2>
            </div>

            <span
              v-if="workbench.selectedItem"
              class="rounded-full bg-emerald-100 px-3 py-1 text-xs font-semibold text-emerald-700"
            >
              Current item
            </span>
          </div>

          <template v-if="workbench.loadingDetail">
            <div class="mt-5 rounded-[24px] border border-dashed border-slate-300 bg-slate-50 p-6 text-sm text-slate-500">
              正在加载详情...
            </div>
          </template>

          <template v-else-if="workbench.selectedItem">
            <div class="mt-5 grid gap-3 sm:grid-cols-2">
              <div class="rounded-[22px] bg-slate-50 p-4">
                <p class="text-xs font-semibold uppercase tracking-[0.18em] text-slate-400">Source</p>
                <p class="mt-2 text-sm font-medium text-slate-900">
                  {{ formatSource(workbench.selectedItem.sourceKind) }}
                </p>
                <p class="mt-1 text-xs text-slate-500">
                  {{ workbench.selectedItem.sourceDeviceId || "local device" }}
                </p>
              </div>
              <div class="rounded-[22px] bg-slate-50 p-4">
                <p class="text-xs font-semibold uppercase tracking-[0.18em] text-slate-400">
                  Metadata
                </p>
                <p class="mt-2 text-sm font-medium text-slate-900">
                  {{ formatDateTime(workbench.selectedItem.createdAt) }}
                </p>
                <p class="mt-1 text-xs text-slate-500">
                  {{ workbench.selectedItem.charCount }} chars, {{ workbench.selectedItem.contentType }}
                </p>
              </div>
              <div class="rounded-[22px] bg-slate-50 p-4">
                <p class="text-xs font-semibold uppercase tracking-[0.18em] text-slate-400">Hash</p>
                <p class="mt-2 break-all font-mono text-xs leading-5 text-slate-700">
                  {{ workbench.selectedItem.hash }}
                </p>
              </div>
              <div class="rounded-[22px] bg-slate-50 p-4">
                <p class="text-xs font-semibold uppercase tracking-[0.18em] text-slate-400">Status</p>
                <p class="mt-2 text-sm font-medium text-slate-900">
                  {{ workbench.selectedItem.pinned ? "Pinned" : "Unpinned" }}
                </p>
                <p class="mt-1 text-xs text-slate-500">
                  {{ workbench.selectedItem.isCurrent ? "Current clipboard item" : "Historical snapshot" }}
                </p>
              </div>
            </div>

            <pre
              class="mt-4 max-h-[36rem] overflow-auto rounded-[28px] bg-slate-950 p-5 text-sm leading-7 whitespace-pre-wrap text-slate-50 shadow-inner"
            >{{ workbench.selectedItem.content }}</pre>

            <div class="mt-4 flex flex-wrap gap-2">
              <button
                class="rounded-full bg-slate-950 px-4 py-2 text-sm font-medium text-white transition hover:bg-slate-800 disabled:cursor-not-allowed disabled:opacity-50"
                :disabled="workbench.busyItemId === workbench.selectedItem.id"
                @click="workbench.activateItem(workbench.selectedItem.id)"
              >
                激活为当前
              </button>
              <button
                class="rounded-full border border-slate-200 bg-white px-4 py-2 text-sm font-medium text-slate-700 transition hover:bg-slate-50 disabled:cursor-not-allowed disabled:opacity-50"
                :disabled="workbench.busyItemId === workbench.selectedItem.id"
                @click="
                  workbench.togglePin(
                    workbench.selectedItem.id,
                    !workbench.selectedItem.pinned,
                  )
                "
              >
                {{ workbench.selectedItem.pinned ? "取消置顶" : "置顶" }}
              </button>
              <button
                class="rounded-full border border-rose-200 bg-rose-50 px-4 py-2 text-sm font-medium text-rose-700 transition hover:bg-rose-100 disabled:cursor-not-allowed disabled:opacity-50"
                :disabled="workbench.busyItemId === workbench.selectedItem.id"
                @click="workbench.deleteItem(workbench.selectedItem.id)"
              >
                删除记录
              </button>
            </div>
          </template>

          <div
            v-else
            class="mt-5 rounded-[24px] border border-dashed border-slate-300 bg-slate-50 p-6 text-sm leading-7 text-slate-500"
          >
            选择一条历史记录后，这里会显示完整内容、来源、时间和操作入口。
          </div>
        </article>
      </section>
    </div>
  </main>
</template>
