<template>
  <div class="error-screen">
    <div class="error-container">
      <!-- Header with Logo -->
      <div class="header-content">
        <div class="logo-container">
          <img src="/icon.png" alt="Compute42" class="logo" />
        </div>
        <h1 class="app-title">Compute42</h1>
      </div>

      <!-- Error Icon and Message -->
      <div class="error-content">
        <div class="error-icon">
          <n-icon size="64" class="error-icon-svg">
            <CloseCircleOutline />
          </n-icon>
        </div>
        <h2 class="error-title">Error</h2>
        <p class="error-message">{{ errorMessage || 'An error has occurred.' }}</p>
        <p class="error-instruction">Please restart Compute42 to continue.</p>
      </div>

      <!-- Action Button -->
      <div class="error-actions">
        <n-button type="primary" size="large" @click="restartApplication">
          Restart Compute42
        </n-button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { NIcon, NButton } from 'naive-ui';
import { CloseCircleOutline } from '@vicons/ionicons5';

const errorMessage = ref<string>('');

const restartApplication = () => {
  // Reload the application
  window.location.reload();
};

// Listen for system error events
onMounted(async () => {
  const { unifiedEventService, EventCategory } = await import('../../services/unifiedEventService');

  await unifiedEventService.addEventListener(EventCategory.System, 'error', async (event) => {
    const payload = event.payload;
    if (payload.error) {
      errorMessage.value = payload.error;
    }
  });
});
</script>

<style scoped>
.error-screen {
  position: fixed;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  background: var(--n-color);
  display: flex;
  justify-content: center;
  align-items: center;
  z-index: 10000;
  overflow: hidden;
}

.error-container {
  max-width: 600px;
  width: 90%;
  padding: 40px;
  text-align: center;
}

.header-content {
  margin-bottom: 40px;
}

.logo-container {
  margin-bottom: 20px;
}

.logo {
  width: 64px;
  height: 64px;
}

.app-title {
  font-size: 28px;
  font-weight: 600;
  margin: 0;
  color: var(--n-text-color);
}

.error-content {
  margin-bottom: 40px;
}

.error-icon {
  margin-bottom: 24px;
  display: flex;
  justify-content: center;
}

.error-icon-svg {
  color: #d03050;
}

.error-title {
  font-size: 24px;
  font-weight: 600;
  margin: 0 0 16px 0;
  color: var(--n-text-color);
}

.error-message {
  font-size: 16px;
  line-height: 1.6;
  margin: 0 0 16px 0;
  color: var(--n-text-color-2);
}

.error-instruction {
  font-size: 14px;
  line-height: 1.5;
  margin: 0;
  color: var(--n-text-color-3);
}

.error-actions {
  display: flex;
  justify-content: center;
  gap: 12px;
}
</style>
