import { createRouter, createWebHistory, RouteRecordRaw } from 'vue-router';
import MainLayout from '../components/layouts/MainLayout.vue';

// Views are now lazy-loaded

const routes: Array<RouteRecordRaw> = [
  {
    path: '/',
    component: MainLayout,
    children: [
      {
        path: '',
        name: 'Home',
        component: () => import('../components/HomeView/EditorLayout.vue'), // This will show EditorView and TerminalView
      },
      {
        path: '/about',
        name: 'About',
        component: () => import('../views/AboutView.vue'),
      },
      {
        path: '/packages',
        name: 'PackageManagement',
        component: () => import('../views/PackageManagement.vue'),
      },
      {
        path: '/settings',
        name: 'Settings',
        component: () => import('../views/SettingsView.vue'),
      },
    ],
  },
  // Add other routes as needed
];

const router = createRouter({
  history: createWebHistory(import.meta.env.BASE_URL), // Using createWebHistory for cleaner URLs
  // Use createWebHashHistory() if you prefer hash-based routing or have issues with server config for history mode.
  routes,
});

// Add navigation guards for debugging
router.beforeEach((to, from, next) => {
  console.log(`Router: Navigating from ${from.name || from.path} to ${to.name || to.path}`);
  next();
});

router.afterEach((to, from) => {
  console.log(`Router: Navigation completed to ${to.name || to.path}`);
});

export default router;
