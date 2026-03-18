import { createRouter, createWebHistory } from "vue-router";

const router = createRouter({
  history: createWebHistory(import.meta.env.BASE_URL),
  routes: [
    {
      path: "/",
      name: "desktop",
      component: () => import("@/pages/DesktopWorkbenchPage.vue"),
    },
    {
      path: "/m",
      name: "mobile",
      component: () => import("@/pages/MobileWorkbenchPage.vue"),
    },
    {
      path: "/:pathMatch(.*)*",
      name: "not-found",
      component: () => import("@/pages/NotFoundPage.vue"),
    },
  ],
});

export default router;
