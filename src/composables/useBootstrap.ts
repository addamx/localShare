import { computed, onMounted, ref } from "vue";
import { loadBootstrapContext } from "@/lib/bootstrap";
import type { AppBootstrap } from "@/types/bootstrap";

export function useBootstrap() {
  const bootstrap = ref<AppBootstrap | null>(null);
  const loading = ref(true);
  const error = ref("");

  const mobileEntryUrl = computed(() => {
    if (!bootstrap.value) {
      return "";
    }

    return `http://localhost:${bootstrap.value.runtimeConfig.preferredPort}${bootstrap.value.routes.mobile}`;
  });

  onMounted(async () => {
    try {
      bootstrap.value = await loadBootstrapContext();
    } catch (reason) {
      error.value = reason instanceof Error ? reason.message : String(reason);
    } finally {
      loading.value = false;
    }
  });

  return {
    bootstrap,
    loading,
    error,
    mobileEntryUrl,
  };
}
