<script setup lang="ts">
import { useBootstrap } from "@/composables/useBootstrap";

const { bootstrap, loading, error, mobileEntryUrl } = useBootstrap();
</script>

<template>
  <main class="min-h-screen px-6 py-6 text-slate-950">
    <div class="mx-auto flex max-w-7xl flex-col gap-6">
      <header
        class="grid gap-4 rounded-[28px] border border-white/70 bg-white/80 p-6 shadow-[0_25px_80px_rgba(15,23,42,0.08)] backdrop-blur xl:grid-cols-[1.6fr_1fr]"
      >
        <section class="space-y-3">
          <p class="text-sm font-semibold uppercase tracking-[0.24em] text-emerald-700">
            Desktop Console
          </p>
          <h1 class="text-4xl font-semibold tracking-tight text-slate-950">
            {{ bootstrap?.appName ?? "LocalShare" }}
          </h1>
          <p class="max-w-3xl text-sm leading-7 text-slate-600">
            T01 只建立工程基线：桌面端和移动端入口已经分离，Rust 模块已经为后续的
            clipboard、http、auth、persistence 链路留好位置。
          </p>
        </section>

        <section class="rounded-[24px] bg-slate-950 p-5 text-slate-50">
          <p class="text-sm font-medium text-emerald-300">Mobile Entry</p>
          <p class="mt-3 break-all font-mono text-sm text-slate-200">
            {{ mobileEntryUrl || "http://localhost:8765/m" }}
          </p>
          <div class="mt-4 flex flex-wrap gap-2 text-xs">
            <span class="rounded-full bg-white/10 px-3 py-1">token lifecycle ready</span>
            <span class="rounded-full bg-white/10 px-3 py-1">sse route reserved</span>
            <span class="rounded-full bg-white/10 px-3 py-1">desktop/mobile split</span>
          </div>
        </section>
      </header>

      <section
        v-if="loading"
        class="rounded-[24px] border border-dashed border-slate-300 bg-white/70 p-6 text-sm text-slate-500"
      >
        正在加载骨架上下文...
      </section>

      <section
        v-else-if="error"
        class="rounded-[24px] border border-rose-200 bg-rose-50 p-6 text-sm text-rose-700"
      >
        {{ error }}
      </section>

      <section v-else class="grid gap-6 xl:grid-cols-[1.2fr_1fr_1fr]">
        <article class="space-y-4 rounded-[24px] bg-white/85 p-5 shadow-[0_18px_60px_rgba(15,23,42,0.08)]">
          <div class="flex items-center justify-between">
            <h2 class="text-lg font-semibold text-slate-900">Runtime Baseline</h2>
            <span class="rounded-full bg-emerald-100 px-3 py-1 text-xs font-medium text-emerald-700">
              ready for T02-T04
            </span>
          </div>
          <dl class="grid gap-3 text-sm text-slate-600">
            <div class="grid grid-cols-[140px_1fr] gap-3">
              <dt>LAN Host</dt>
              <dd class="font-mono text-slate-900">{{ bootstrap?.runtimeConfig.lanHost }}</dd>
            </div>
            <div class="grid grid-cols-[140px_1fr] gap-3">
              <dt>Preferred Port</dt>
              <dd class="font-mono text-slate-900">{{ bootstrap?.runtimeConfig.preferredPort }}</dd>
            </div>
            <div class="grid grid-cols-[140px_1fr] gap-3">
              <dt>Token TTL</dt>
              <dd class="font-mono text-slate-900">
                {{ bootstrap?.runtimeConfig.tokenTtlMinutes }} minutes
              </dd>
            </div>
            <div class="grid grid-cols-[140px_1fr] gap-3">
              <dt>Clipboard Poll</dt>
              <dd class="font-mono text-slate-900">
                {{ bootstrap?.runtimeConfig.clipboardPollIntervalMs }} ms
              </dd>
            </div>
            <div class="grid grid-cols-[140px_1fr] gap-3">
              <dt>Text Limit</dt>
              <dd class="font-mono text-slate-900">{{ bootstrap?.runtimeConfig.maxTextBytes }} bytes</dd>
            </div>
          </dl>
        </article>

        <article class="space-y-4 rounded-[24px] bg-white/85 p-5 shadow-[0_18px_60px_rgba(15,23,42,0.08)]">
          <h2 class="text-lg font-semibold text-slate-900">Rust Modules</h2>
          <ul class="space-y-3 text-sm text-slate-600">
            <li class="rounded-2xl bg-slate-100 px-4 py-3">clipboard: polling and dedupe entry</li>
            <li class="rounded-2xl bg-slate-100 px-4 py-3">http: LAN server and route placeholders</li>
            <li class="rounded-2xl bg-slate-100 px-4 py-3">auth: token lifecycle and write protection</li>
            <li class="rounded-2xl bg-slate-100 px-4 py-3">persistence: sqlite path and migration entry</li>
            <li class="rounded-2xl bg-slate-100 px-4 py-3">network: device name and LAN identity</li>
          </ul>
        </article>

        <article class="space-y-4 rounded-[24px] bg-white/85 p-5 shadow-[0_18px_60px_rgba(15,23,42,0.08)]">
          <h2 class="text-lg font-semibold text-slate-900">Resolved Paths</h2>
          <dl class="space-y-3 text-sm text-slate-600">
            <div>
              <dt class="font-medium text-slate-500">App Dir</dt>
              <dd class="mt-1 break-all font-mono text-slate-900">{{ bootstrap?.paths.appDir }}</dd>
            </div>
            <div>
              <dt class="font-medium text-slate-500">Data Dir</dt>
              <dd class="mt-1 break-all font-mono text-slate-900">{{ bootstrap?.paths.dataDir }}</dd>
            </div>
            <div>
              <dt class="font-medium text-slate-500">DB Path</dt>
              <dd class="mt-1 break-all font-mono text-slate-900">
                {{ bootstrap?.paths.databasePath }}
              </dd>
            </div>
            <div>
              <dt class="font-medium text-slate-500">Logs Dir</dt>
              <dd class="mt-1 break-all font-mono text-slate-900">{{ bootstrap?.paths.logsDir }}</dd>
            </div>
          </dl>
        </article>
      </section>
    </div>
  </main>
</template>
