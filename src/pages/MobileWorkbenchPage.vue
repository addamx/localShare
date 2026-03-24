<script setup lang="ts">
import { computed } from "vue";
import { useMobileWorkbench } from "@/composables/useMobileWorkbench";

const {
  bootstrap,
  bootstrapLoading,
  bootstrapError,
  loading,
  loadingItems,
  submitting,
  search,
  pinnedOnly,
  draft,
  draftBytes,
  canSubmit,
  items,
  selectedSummary,
  selectedDetail,
  notice,
  authState,
  serviceState,
  streamState,
  sessionExpiryText,
  accessUrl,
  mobileEntryUrl,
  deviceName,
  serviceLabel,
  summaryCounts,
  statusTone,
  lastSyncedAt,
  formatTimestamp,
  formatSourceKind,
  selectItem,
  activateItem,
  submitDraft,
  refreshNow,
} = useMobileWorkbench();

const noticeToneClass = computed(() => {
  const tone = notice.value?.kind ?? "info";
  const map: Record<string, string> = {
    success: "border-emerald-200 bg-emerald-50 text-emerald-900",
    info: "border-sky-200 bg-sky-50 text-sky-900",
    warning: "border-amber-200 bg-amber-50 text-amber-950",
    error: "border-rose-200 bg-rose-50 text-rose-900",
  };

  return map[tone];
});

const statusToneClass = computed(() => {
  const map: Record<string, string> = {
    success: "border-emerald-200 bg-emerald-50 text-emerald-800",
    warning: "border-amber-200 bg-amber-50 text-amber-900",
    error: "border-rose-200 bg-rose-50 text-rose-800",
  };

  return map[statusTone.value];
});

const onlineHint = computed(() => {
  if (authState.value === "missing") {
    return "等待扫描二维码";
  }

  if (authState.value === "invalid") {
    return "会话已失效";
  }

  if (serviceState.value === "offline") {
    return "局域网服务离线";
  }

  if (serviceState.value === "failed") {
    return "服务异常";
  }

  return streamState.value === "live" ? "实时同步" : "准备同步";
});

const refreshDisabled = computed(
  () => authState.value !== "valid" || loading.value || loadingItems.value,
);

const selectedContent = computed(() => {
  if (selectedDetail.value?.content) {
    return selectedDetail.value.content;
  }

  if (selectedSummary.value?.preview) {
    return selectedSummary.value.preview;
  }

  return "暂无详情";
});

const selectedMeta = computed(() => {
  const item = selectedDetail.value ?? selectedSummary.value;
  if (!item) {
    return [];
  }

  return [
    ["来源", formatSourceKind(item.sourceKind)],
    ["创建", formatTimestamp(item.createdAt)],
    ["更新", formatTimestamp(item.updatedAt)],
    ["字符", `${item.charCount}`],
    ["当前", item.isCurrent ? "是" : "否"],
    ["置顶", item.pinned ? "是" : "否"],
  ];
});

const sessionStatusLabel = computed(() => {
  if (authState.value === "invalid") {
    return "会话过期";
  }

  if (authState.value === "missing") {
    return "缺少 token";
  }

  return `${serviceLabel.value} · ${sessionExpiryText.value}`;
});

const detailReady = computed(() => Boolean(selectedDetail.value || selectedSummary.value));
const interactionDisabled = computed(
  () =>
    authState.value !== "valid" ||
    loading.value ||
    loadingItems.value ||
    serviceState.value === "offline" ||
    serviceState.value === "failed",
);
</script>

<template>
  <main class="min-h-screen px-4 py-4 text-slate-950">
    <div class="mx-auto flex max-w-xl flex-col gap-4">
      <header
        class="relative overflow-hidden rounded-[32px] border border-slate-200/80 bg-slate-950 p-5 text-slate-50 shadow-[0_24px_80px_rgba(15,23,42,0.24)]"
      >
        <div
          class="pointer-events-none absolute inset-0 bg-[radial-gradient(circle_at_top_right,rgba(34,211,238,0.18),transparent_28%),radial-gradient(circle_at_bottom_left,rgba(16,185,129,0.18),transparent_30%)]"
        ></div>
        <div class="relative space-y-4">
          <div class="space-y-2">
            <p class="text-xs font-semibold uppercase tracking-[0.28em] text-cyan-300">
              Mobile H5
            </p>
            <h1 class="text-3xl font-semibold tracking-tight">
              {{ bootstrap?.appName ?? "LocalShare" }}
            </h1>
            <p class="max-w-md text-sm leading-6 text-slate-300">
              手机端用于浏览历史、提交文本和直接激活到桌面剪贴板。
            </p>
          </div>

          <div class="grid gap-2 text-xs sm:grid-cols-3">
            <div class="rounded-2xl bg-white/10 px-3 py-2">
              <p class="text-slate-400">设备</p>
              <p class="mt-1 font-medium text-white">{{ deviceName }}</p>
            </div>
            <div :class="[statusToneClass, 'rounded-2xl px-3 py-2']">
              <p class="text-slate-400">状态</p>
              <p class="mt-1 font-medium">{{ onlineHint }}</p>
            </div>
            <div class="rounded-2xl bg-white/10 px-3 py-2">
              <p class="text-slate-400">会话</p>
              <p class="mt-1 font-medium text-white">{{ sessionStatusLabel }}</p>
            </div>
          </div>

          <div class="rounded-[24px] border border-white/10 bg-black/20 p-4">
            <p class="text-[11px] uppercase tracking-[0.28em] text-slate-400">Access URL</p>
            <p class="mt-2 break-all font-mono text-xs leading-6 text-slate-100">
              {{ accessUrl || mobileEntryUrl || "请通过二维码链接打开" }}
            </p>
          </div>
        </div>
      </header>

      <section
        v-if="bootstrapLoading || loading"
        class="rounded-[28px] border border-dashed border-slate-300 bg-white/80 p-5 text-sm text-slate-500 shadow-[0_18px_60px_rgba(15,23,42,0.08)]"
      >
        正在连接移动端工作台...
      </section>

      <section
        v-else-if="bootstrapError"
        class="rounded-[28px] border border-rose-200 bg-rose-50 p-5 text-sm text-rose-700"
      >
        {{ bootstrapError }}
      </section>

      <section
        v-if="notice"
        :class="[
          noticeToneClass,
          'rounded-[28px] border p-4 text-sm shadow-[0_18px_60px_rgba(15,23,42,0.08)]',
        ]"
      >
        {{ notice.message }}
      </section>

      <section
        class="rounded-[28px] border border-white/80 bg-white/90 p-5 shadow-[0_18px_60px_rgba(15,23,42,0.08)] backdrop-blur"
      >
        <div class="flex items-center justify-between gap-3">
          <div>
            <p class="text-xs font-semibold uppercase tracking-[0.22em] text-slate-500">
              Text Submit
            </p>
            <h2 class="mt-1 text-lg font-semibold text-slate-950">提交新文本</h2>
          </div>
          <span class="rounded-full bg-slate-100 px-3 py-1 text-xs font-medium text-slate-600">
            {{ draftBytes }} / {{ bootstrap?.runtimeConfig.maxTextBytes ?? 0 }} bytes
          </span>
        </div>

        <form class="mt-4 space-y-3" @submit.prevent="submitDraft">
          <textarea
            v-model="draft"
            :disabled="interactionDisabled"
            rows="5"
            class="min-h-[140px] w-full rounded-[22px] border border-slate-200 bg-slate-50 px-4 py-4 text-sm leading-6 text-slate-950 outline-none transition focus:border-slate-400 focus:bg-white disabled:cursor-not-allowed disabled:bg-slate-100"
            placeholder="在这里输入要发往桌面的纯文本..."
          ></textarea>

          <div class="flex flex-wrap gap-2">
            <button
              type="submit"
              :disabled="!canSubmit"
              class="rounded-full bg-slate-950 px-4 py-3 text-sm font-medium text-white transition hover:bg-slate-800 disabled:cursor-not-allowed disabled:bg-slate-300"
            >
              {{ submitting ? "提交中..." : "提交到桌面" }}
            </button>
            <button
              type="button"
              :disabled="refreshDisabled"
              class="rounded-full border border-slate-200 bg-white px-4 py-3 text-sm font-medium text-slate-700 transition hover:bg-slate-50 disabled:cursor-not-allowed disabled:text-slate-400"
              @click="refreshNow"
            >
              刷新历史
            </button>
            <button
              type="button"
              :disabled="interactionDisabled || !draft"
              class="rounded-full border border-slate-200 bg-white px-4 py-3 text-sm font-medium text-slate-700 transition hover:bg-slate-50 disabled:cursor-not-allowed disabled:text-slate-400"
              @click="draft = ''"
            >
              清空输入
            </button>
          </div>
        </form>
      </section>

      <section
        class="rounded-[28px] border border-white/80 bg-white/90 p-5 shadow-[0_18px_60px_rgba(15,23,42,0.08)] backdrop-blur"
      >
        <div class="flex items-center justify-between gap-3">
          <div>
            <p class="text-xs font-semibold uppercase tracking-[0.22em] text-slate-500">History</p>
            <h2 class="mt-1 text-lg font-semibold text-slate-950">历史浏览</h2>
          </div>
          <button
            type="button"
              :disabled="refreshDisabled"
              class="rounded-full border border-slate-200 bg-white px-4 py-2 text-sm font-medium text-slate-700 transition hover:bg-slate-50 disabled:cursor-not-allowed disabled:text-slate-400"
            @click="refreshNow"
          >
            {{ loadingItems ? "同步中" : "同步" }}
          </button>
        </div>

        <div class="mt-4 grid gap-3">
          <label class="rounded-[22px] border border-slate-200 bg-slate-50 px-4 py-3">
            <span class="block text-[11px] uppercase tracking-[0.22em] text-slate-500">Search</span>
            <input
              v-model="search"
              :disabled="authState !== 'valid'"
              class="mt-2 w-full bg-transparent text-sm text-slate-950 outline-none placeholder:text-slate-400 disabled:cursor-not-allowed"
              placeholder="搜索历史内容"
            />
          </label>

          <div class="flex flex-wrap items-center gap-2">
            <button
              type="button"
              :class="[
                pinnedOnly
                  ? 'border-slate-950 bg-slate-950 text-white'
                  : 'border-slate-200 bg-white text-slate-700',
                'rounded-full border px-4 py-2 text-sm font-medium transition disabled:cursor-not-allowed disabled:border-slate-200 disabled:bg-slate-100 disabled:text-slate-400',
              ]"
              :disabled="authState !== 'valid'"
              @click="pinnedOnly = !pinnedOnly"
            >
              {{ pinnedOnly ? "仅看置顶" : "全部历史" }}
            </button>
            <span class="rounded-full bg-slate-100 px-3 py-2 text-xs text-slate-600">
              共 {{ summaryCounts.total }} 条
            </span>
            <span class="rounded-full bg-slate-100 px-3 py-2 text-xs text-slate-600">
              置顶 {{ summaryCounts.pinned }} 条
            </span>
            <span class="rounded-full bg-slate-100 px-3 py-2 text-xs text-slate-600">
              当前 {{ summaryCounts.current }} 条
            </span>
          </div>
        </div>

        <div class="mt-4 space-y-3">
          <section
            v-if="loadingItems"
            class="rounded-[22px] border border-dashed border-slate-300 bg-slate-50 p-4 text-sm text-slate-500"
          >
            历史列表同步中...
          </section>

          <section
            v-else-if="!items.length"
            class="rounded-[22px] border border-dashed border-slate-300 bg-slate-50 p-4 text-sm text-slate-500"
          >
            {{ search || pinnedOnly ? "没有匹配到符合条件的历史记录" : "还没有历史记录，复制一段文本后会自动出现在这里" }}
          </section>

          <article
            v-for="item in items"
            :key="item.id"
            :class="[
              selectedItemId === item.id
                ? 'border-slate-950 bg-slate-950 text-white shadow-[0_18px_42px_rgba(15,23,42,0.18)]'
                : 'border-slate-200 bg-white text-slate-950',
              'rounded-[24px] border p-4 transition active:scale-[0.99]',
            ]"
            @click="selectItem(item.id)"
          >
            <div class="flex items-start justify-between gap-3">
              <div class="min-w-0 flex-1">
                <p class="text-sm font-medium leading-6">
                  {{ item.preview || "（空白预览）" }}
                </p>
                <p :class="[selectedItemId === item.id ? 'text-slate-300' : 'text-slate-500', 'mt-2 text-xs']">
                  {{ formatSourceKind(item.sourceKind) }} · {{ formatTimestamp(item.createdAt) }} ·
                  {{ item.charCount }} 字符
                </p>
              </div>

              <button
                type="button"
                :disabled="interactionDisabled"
                :class="[
                  selectedItemId === item.id
                    ? 'border-white/20 bg-white/10 text-white'
                    : 'border-slate-200 bg-slate-50 text-slate-700',
                  'shrink-0 rounded-full border px-3 py-2 text-xs font-medium transition disabled:cursor-not-allowed disabled:text-slate-400',
                ]"
                @click.stop="activateItem(item)"
              >
                激活
              </button>
            </div>

            <div class="mt-3 flex flex-wrap gap-2 text-[11px]">
              <span
                v-if="item.isCurrent"
                :class="[
                  selectedItemId === item.id ? 'bg-white/15 text-white' : 'bg-emerald-100 text-emerald-800',
                  'rounded-full px-2.5 py-1 font-medium',
                ]"
              >
                当前
              </span>
              <span
                v-if="item.pinned"
                :class="[
                  selectedItemId === item.id ? 'bg-white/15 text-white' : 'bg-amber-100 text-amber-800',
                  'rounded-full px-2.5 py-1 font-medium',
                ]"
              >
                置顶
              </span>
              <span :class="[selectedItemId === item.id ? 'bg-white/15 text-white' : 'bg-slate-100 text-slate-600', 'rounded-full px-2.5 py-1 font-medium']">
                {{ item.sourceDeviceId ?? "本机" }}
              </span>
            </div>
          </article>
        </div>
      </section>

      <section
        v-if="detailReady"
        class="rounded-[28px] border border-white/80 bg-white/90 p-5 shadow-[0_18px_60px_rgba(15,23,42,0.08)] backdrop-blur"
      >
        <div class="flex items-center justify-between gap-3">
          <div>
            <p class="text-xs font-semibold uppercase tracking-[0.22em] text-slate-500">Detail</p>
            <h2 class="mt-1 text-lg font-semibold text-slate-950">历史详情</h2>
          </div>
          <span class="rounded-full bg-slate-100 px-3 py-1 text-xs text-slate-600">
            {{ selectedDetail?.charCount ?? selectedSummary?.charCount ?? 0 }} 字符
          </span>
        </div>

        <div class="mt-4 rounded-[24px] bg-slate-950 p-4 text-slate-50">
          <p class="text-[11px] uppercase tracking-[0.22em] text-slate-400">Content</p>
          <div class="mt-3 max-h-72 overflow-auto whitespace-pre-wrap break-words text-sm leading-7">
            {{ selectedContent }}
          </div>
        </div>

        <dl class="mt-4 grid gap-2 text-sm">
          <div
            v-for="[label, value] in selectedMeta"
            :key="label"
            class="flex items-center justify-between rounded-2xl bg-slate-50 px-4 py-3 text-slate-700"
          >
            <dt class="text-slate-500">{{ label }}</dt>
            <dd class="font-medium text-slate-950">{{ value }}</dd>
          </div>
        </dl>
      </section>

      <section
        v-if="!detailReady"
        class="rounded-[28px] border border-dashed border-slate-300 bg-white/80 p-5 text-sm text-slate-500 shadow-[0_18px_60px_rgba(15,23,42,0.08)]"
      >
        选择一条历史记录后，可以查看完整文本和元信息。
      </section>

      <section
        v-if="authState === 'invalid' || authState === 'missing'"
        class="rounded-[28px] border border-rose-200 bg-rose-50 p-5 text-sm leading-6 text-rose-700"
      >
        <p class="font-medium">
          {{ authState === 'invalid' ? "会话已失效，请重新扫码。" : "当前页面缺少 token，请使用二维码链接打开。" }}
        </p>
        <p class="mt-2 break-all font-mono text-xs text-rose-600">
          {{ accessUrl || "会话链接未就绪" }}
        </p>
      </section>

      <section
        v-if="serviceState === 'offline' || serviceState === 'failed'"
        class="rounded-[28px] border border-amber-200 bg-amber-50 p-5 text-sm leading-6 text-amber-900"
      >
        <p class="font-medium">局域网服务当前不可用。</p>
        <p class="mt-2">
          请回到桌面端检查端口、网络权限或服务状态，然后再刷新页面。
        </p>
      </section>

      <footer class="pb-4 text-center text-xs text-slate-500">
        <span>
          {{ lastSyncedAt ? `最后同步 ${formatTimestamp(lastSyncedAt)}` : "尚未完成同步" }}
        </span>
      </footer>
    </div>
  </main>
</template>
