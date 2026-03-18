<template>
  <main class="flex min-h-screen items-center justify-center px-6 py-8">
    <section class="max-w-lg rounded-[28px] bg-white/85 p-8 text-center shadow-[0_20px_60px_rgba(15,23,42,0.08)]">
      <p class="text-sm font-semibold uppercase tracking-[0.22em] text-slate-500">404</p>
      <h1 class="mt-4 text-3xl font-semibold text-slate-950">Route Not Found</h1>
      <p class="mt-3 text-sm leading-7 text-slate-600">
        当前工程骨架只预留了桌面路由 `/` 和移动路由 `/m`。
      </p>
      <div class="mt-6 flex justify-center gap-3 text-sm">
        <RouterLink class="rounded-full bg-slate-950 px-4 py-2 text-white" to="/">返回桌面端</RouterLink>
        <RouterLink class="rounded-full bg-slate-200 px-4 py-2 text-slate-900" to="/m">查看移动端</RouterLink>
      </div>
    </section>
  </main>
</template>
