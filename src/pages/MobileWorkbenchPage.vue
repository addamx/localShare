<script setup lang="ts">
import { useBootstrap } from "@/composables/useBootstrap";

const { bootstrap, loading, error } = useBootstrap();
</script>

<template>
  <main class="min-h-screen px-4 py-5">
    <div class="mx-auto flex max-w-md flex-col gap-4">
      <header class="rounded-[28px] bg-slate-950 p-5 text-slate-50 shadow-[0_20px_60px_rgba(15,23,42,0.18)]">
        <p class="text-xs font-semibold uppercase tracking-[0.24em] text-cyan-300">Mobile H5</p>
        <h1 class="mt-3 text-2xl font-semibold">{{ bootstrap?.appName ?? "LocalShare" }}</h1>
        <p class="mt-2 text-sm leading-6 text-slate-300">
          这是手机端入口骨架，后续 T06 会在这里补历史列表、文本提交、激活和会话提示。
        </p>
      </header>

      <section
        v-if="loading"
        class="rounded-[24px] border border-dashed border-slate-300 bg-white/75 p-5 text-sm text-slate-500"
      >
        正在加载移动端基线...
      </section>

      <section
        v-else-if="error"
        class="rounded-[24px] border border-rose-200 bg-rose-50 p-5 text-sm text-rose-700"
      >
        {{ error }}
      </section>

      <template v-else>
        <section class="rounded-[24px] bg-white/85 p-5 shadow-[0_18px_60px_rgba(15,23,42,0.08)]">
          <p class="text-xs font-semibold uppercase tracking-[0.2em] text-slate-500">Session Area</p>
          <div class="mt-4 grid gap-3 text-sm text-slate-700">
            <div class="flex items-center justify-between rounded-2xl bg-slate-100 px-4 py-3">
              <span>Host</span>
              <span class="font-mono">{{ bootstrap?.runtimeConfig.lanHost }}</span>
            </div>
            <div class="flex items-center justify-between rounded-2xl bg-slate-100 px-4 py-3">
              <span>TTL</span>
              <span class="font-mono">{{ bootstrap?.runtimeConfig.tokenTtlMinutes }}m</span>
            </div>
            <div class="flex items-center justify-between rounded-2xl bg-slate-100 px-4 py-3">
              <span>Route</span>
              <span class="font-mono">{{ bootstrap?.routes.mobile }}</span>
            </div>
          </div>
        </section>

        <section class="rounded-[24px] bg-white/85 p-5 shadow-[0_18px_60px_rgba(15,23,42,0.08)]">
          <p class="text-xs font-semibold uppercase tracking-[0.2em] text-slate-500">Input Placeholder</p>
          <div class="mt-4 rounded-[22px] border border-slate-200 bg-slate-50 px-4 py-5 text-sm text-slate-500">
            这里预留给移动端文本输入区。
          </div>
        </section>

        <section class="rounded-[24px] bg-white/85 p-5 shadow-[0_18px_60px_rgba(15,23,42,0.08)]">
          <p class="text-xs font-semibold uppercase tracking-[0.2em] text-slate-500">History Placeholder</p>
          <ul class="mt-4 space-y-3 text-sm text-slate-600">
            <li class="rounded-2xl bg-slate-100 px-4 py-4">最近历史列表将在 T06 接入</li>
            <li class="rounded-2xl bg-slate-100 px-4 py-4">支持搜索、仅看置顶、刷新</li>
            <li class="rounded-2xl bg-slate-100 px-4 py-4">支持点击后直接激活到桌面剪贴板</li>
          </ul>
        </section>
      </template>
    </div>
  </main>
</template>
