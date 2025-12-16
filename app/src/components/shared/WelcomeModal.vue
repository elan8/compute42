<template>
  <div class="modal-overlay">
    <div class="modal-container">
      <!-- Header with Logo -->
      <div class="header-content">
        <div class="header-row">
          <img src="/icon.png" alt="Compute42" class="logo" />
          <h1 class="app-title">Compute42</h1>
        </div>
      </div>

      <!-- Main content -->
      <div class="modal-content">
        <!-- Auth Check Loading -->
        <div v-if="currentMode === 'checking'" class="auth-check-content">
          <div class="loading-spinner">
            <div class="spinner"></div>
          </div>
          <p class="checking-text">Checking your authentication status...</p>
        </div>

        <!-- Login/Signup Form -->
        <div v-else-if="currentMode === 'login' || currentMode === 'signup'" class="auth-content">
          <div class="step-header">
            <h2 class="step-title">
              {{ currentMode === 'login' ? 'Welcome Back' : 'Create Your Account' }}
            </h2>
            <p class="step-subtitle">
              {{
                currentMode === 'login'
                  ? 'Sign in to continue to Compute42'
                  : 'Join Compute42 to get started'
              }}
            </p>
          </div>

          <n-form
            ref="authFormRef"
            :model="authForm"
            :rules="authRules"
            label-placement="top"
            size="large"
          >
            <n-form-item label="Email Address" path="email">
              <n-input
                v-model:value="authForm.email"
                placeholder="Enter your email address"
                :disabled="isLoading"
                @keyup.enter="handleAuthSubmit"
              />
            </n-form-item>

            <n-form-item label="Password" path="password">
              <n-input
                v-model:value="authForm.password"
                type="password"
                placeholder="Enter your password"
                :disabled="isLoading"
                show-password-on="click"
                @keyup.enter="handleAuthSubmit"
              />
            </n-form-item>

            <n-form-item
              v-if="currentMode === 'signup'"
              label="Confirm Password"
              path="confirmPassword"
            >
              <n-input
                v-model:value="authForm.confirmPassword"
                type="password"
                placeholder="Confirm your password"
                :disabled="isLoading"
                show-password-on="click"
                @keyup.enter="handleAuthSubmit"
              />
            </n-form-item>

            <div class="form-actions">
              <n-button
                type="primary"
                size="large"
                :loading="isLoading"
                :disabled="!canSubmitAuth"
                @click="handleAuthSubmit"
                block
              >
                {{ currentMode === 'login' ? 'Sign In' : 'Create Account' }}
              </n-button>
            </div>
          </n-form>

          <div class="mode-switch">
            <n-text>
              {{ currentMode === 'login' ? "Don't have an account?" : 'Already have an account?' }}
              <n-button text type="primary" :disabled="isLoading" @click="toggleMode">
                {{ currentMode === 'login' ? 'Sign up' : 'Sign in' }}
              </n-button>
            </n-text>
          </div>

          <!-- Error display -->
          <n-alert
            v-if="authError"
            type="error"
            :title="authError"
            closable
            @close="clearAuthError"
            class="error-alert"
          />
        </div>

        <!-- Email Verification -->
        <div v-else-if="currentMode === 'emailVerification'" class="verification-content">
          <div class="step-header">
            <h2 class="step-title">Verify Your Email</h2>
            <p class="step-subtitle">We've sent a verification code</p>
          </div>

          <n-form
            ref="verificationFormRef"
            :model="verificationForm"
            :rules="verificationRules"
            label-placement="top"
            size="large"
          >
            <n-form-item label="Verification Code" path="code">
              <n-input
                v-model:value="verificationForm.code"
                placeholder="Enter 6-digit code"
                :disabled="isLoading"
                maxlength="6"
                @keyup.enter="handleVerificationSubmit"
              />
            </n-form-item>

            <div class="form-actions">
              <n-button
                type="primary"
                size="large"
                :loading="isLoading"
                :disabled="!verificationForm.code || verificationForm.code.length !== 6"
                @click="handleVerificationSubmit"
                block
              >
                Verify Email
              </n-button>
            </div>
          </n-form>

          <div class="verification-actions">
            <n-text>
              Didn't receive the code?
              <n-button
                text
                type="primary"
                :disabled="isLoading || resendCooldown > 0"
                @click="handleResendVerification"
              >
                {{ resendCooldown > 0 ? `Resend in ${resendCooldown}s` : 'Resend code' }}
              </n-button>
            </n-text>
          </div>

          <!-- Error display -->
          <n-alert
            v-if="verificationError"
            type="error"
            :title="verificationError"
            closable
            @close="clearVerificationError"
            class="error-alert"
          />
        </div>

        <!-- EULA Agreement -->
        <div v-else-if="currentMode === 'eulaAgreement'" class="eula-content">
          <div class="step-header">
            <h2 class="step-title">Terms of Service</h2>
            <p class="step-subtitle">Please read and agree to our terms of service to continue</p>
          </div>

          <div class="eula-container">
            <n-scrollbar style="max-height: 300px">
              <div class="eula-text" v-html="eulaContent"></div>
            </n-scrollbar>
          </div>

          <n-form
            v-if="eulaLoadedSuccessfully"
            ref="eulaFormRef"
            :model="eulaForm"
            :rules="eulaRules"
            class="eula-form"
          >
            <n-form-item path="agreed">
              <n-checkbox v-model:checked="eulaForm.agreed" :disabled="isLoading">
                I have read and agree to the Terms of Service
              </n-checkbox>
            </n-form-item>
          </n-form>

          <div v-if="eulaLoadedSuccessfully" class="form-actions">
            <n-button
              type="primary"
              size="large"
              :loading="isLoading"
              :disabled="!eulaForm.agreed"
              @click="handleEulaSubmit"
              block
            >
              Continue
            </n-button>
          </div>

          <div v-else class="error-actions">
            <n-button
              type="primary"
              size="large"
              :loading="isLoading"
              @click="loadEulaContent"
              block
            >
              Retry Loading Terms
            </n-button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue';
import {
  useMessage,
  NForm,
  NFormItem,
  NInput,
  NButton,
  NText,
  NAlert,
  NScrollbar,
  NCheckbox,
} from 'naive-ui';
import { invoke } from '@tauri-apps/api/core';
// import { listen } from '@tauri-apps/api/event'
import { debug, error as logError } from '../../utils/logger';
// import { unifiedEventService, EventCategory } from '../../services/unifiedEventService'

// Props
interface Props {
  show: boolean;
  initialMode?: 'checking' | 'login' | 'signup' | 'emailVerification' | 'eulaAgreement';
  userEmail?: string;
  authError?: string;
}

const props = defineProps<Props>();

// Emits
const emit = defineEmits<{
  'update:show': [value: boolean];
  complete: [];
  'mode-change': [mode: 'login' | 'signup'];
}>();

// Composables
const message = useMessage();

// Refs
const authFormRef = ref();
const verificationFormRef = ref();
const eulaFormRef = ref();

// UI State - only for form data and loading states
const isLoading = ref(false);
const resendCooldown = ref(0);

// Form data
const authForm = ref({
  email: '',
  password: '',
  confirmPassword: '',
});

const verificationForm = ref({
  code: '',
});

const eulaForm = ref({
  agreed: false,
});

// Error states
const verificationError = ref('');

// Computed values from props
const userEmail = computed(() => props.userEmail || '');
const authError = computed(() => props.authError || '');

// EULA content
const eulaContent = ref('');
const eulaLoadedSuccessfully = ref(false);

// Current mode - controlled by App.vue via props
const currentMode = computed(() => props.initialMode || 'checking');

// Computed
// Use props.show directly; parent controls visibility

const canSubmitAuth = computed(() => {
  if (currentMode.value === 'login') {
    return authForm.value.email && authForm.value.password && authForm.value.password.length >= 6;
  }
  return (
    authForm.value.email &&
    authForm.value.password &&
    authForm.value.password.length >= 6 &&
    authForm.value.confirmPassword &&
    authForm.value.password === authForm.value.confirmPassword
  );
});

// Form validation rules
const authRules = computed(() => ({
  email: [
    { required: true, message: 'Please enter your email address', trigger: 'blur' },
    {
      validator: (_rule: any, value: string) => {
        const emailRegex = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
        if (!emailRegex.test(value)) {
          return new Error('Please enter a valid email address');
        }
      },
      trigger: 'blur',
    },
  ],
  password: [
    { required: true, message: 'Please enter your password', trigger: 'blur' },
    { min: 6, message: 'Password must be at least 6 characters', trigger: 'blur' },
  ],
  confirmPassword:
    currentMode.value === 'login'
      ? []
      : [
          { required: true, message: 'Please confirm your password', trigger: 'blur' },
          {
            validator: (_rule: any, value: string) => {
              if (value !== authForm.value.password) {
                return new Error('Passwords do not match');
              }
            },
            trigger: 'blur',
          },
        ],
}));

const verificationRules = {
  code: [
    { required: true, message: 'Please enter the verification code', trigger: 'blur' },
    { len: 6, message: 'Verification code must be 6 digits', trigger: 'blur' },
  ],
};

const eulaRules = {
  agreed: [
    {
      validator: (_rule: any, value: boolean) => {
        if (!value) {
          return new Error('You must agree to the terms of service');
        }
      },
      trigger: 'change',
    },
  ],
};

// Event listener for account events

// No event handling needed - WelcomeModal is now purely controlled by App.vue

// Methods
const toggleMode = () => {
  if (currentMode.value === 'login') {
    emit('mode-change', 'signup');
  } else if (currentMode.value === 'signup') {
    emit('mode-change', 'login');
  }
  clearAuthError();
  authForm.value = {
    email: authForm.value.email,
    password: '',
    confirmPassword: '',
  };
};

const clearAuthError = () => {
  // authError is a computed prop from parent; notify parent to clear
  emit('update:show', true);
};

const clearVerificationError = () => {
  verificationError.value = '';
};

const handleAuthSubmit = async () => {
  try {
    await authFormRef.value?.validate();
  } catch (validationError) {
    return;
  }

  isLoading.value = true;
  clearAuthError();

  try {
    if (currentMode.value === 'login') {
      await invoke('account_login_attempt', {
        email: authForm.value.email,
        password: authForm.value.password,
      });
    } else {
      await invoke('account_registration_attempt', {
        email: authForm.value.email,
        password: authForm.value.password,
      });
    }
  } catch (error: any) {
    message.error(error.message || 'Authentication failed');
  } finally {
    isLoading.value = false;
  }
};

const handleVerificationSubmit = async () => {
  try {
    await verificationFormRef.value?.validate();
  } catch (error) {
    return;
  }

  isLoading.value = true;
  clearVerificationError();

  try {
    await invoke('account_email_verification_submit', {
      code: verificationForm.value.code,
    });
  } catch (error: any) {
    verificationError.value = error.message || 'Verification failed';
  } finally {
    isLoading.value = false;
  }
};

const handleResendVerification = async () => {
  if (resendCooldown.value > 0) return;

  isLoading.value = true;
  clearVerificationError();

  try {
    await invoke('account_email_verification_resend');
    message.success('Verification code sent successfully');
    startResendCooldown();
  } catch (error: any) {
    verificationError.value = error.message || 'Failed to resend verification code';
  } finally {
    isLoading.value = false;
  }
};

const startResendCooldown = () => {
  resendCooldown.value = 60;
  const interval = setInterval(() => {
    resendCooldown.value--;
    if (resendCooldown.value <= 0) {
      clearInterval(interval);
    }
  }, 1000);
};

const handleEulaSubmit = async () => {
  try {
    await eulaFormRef.value?.validate();
  } catch (error) {
    return;
  }

  isLoading.value = true;

  try {
    await invoke('account_eula_agreement_submit', {
      agreed: eulaForm.value.agreed,
    });
  } catch (error: any) {
    message.error(error.message || 'Failed to accept EULA');
  } finally {
    isLoading.value = false;
  }
};

// Load EULA content from backend
const loadEulaContent = async () => {
  try {
    const response = (await invoke('get_eula_content')) as any;
    if (response && response.content) {
      // Check if this is an error response
      if (response.status === 'error') {
        // Show error message instead of EULA content
        eulaContent.value = response.content
          .replace(/^# (.*$)/gim, '<h1>$1</h1>')
          .replace(/^## (.*$)/gim, '<h2>$1</h2>')
          .replace(/^### (.*$)/gim, '<h3>$1</h3>')
          .replace(/^#### (.*$)/gim, '<h4>$1</h4>')
          .replace(/^##### (.*$)/gim, '<h5>$1</h5>')
          .replace(/^###### (.*$)/gim, '<h6>$1</h6>')
          .replace(/\*\*(.*?)\*\*/g, '<strong>$1</strong>')
          .replace(/\*(.*?)\*/g, '<em>$1</em>')
          .replace(/^- (.*$)/gim, '<li>$1</li>')
          .replace(/(<li>.*<\/li>)/s, '<ul>$1</ul>')
          .replace(/\n\n/g, '</p><p>')
          .replace(/\n/g, '<br>')
          .replace(/^/, '<p>')
          .replace(/$/, '</p>')
          .replace(/<p><\/p>/g, '')
          .replace(/<p><ul>/g, '<ul>')
          .replace(/<\/ul><\/p>/g, '</ul>');
        eulaLoadedSuccessfully.value = false;
        return;
      }

      // Convert markdown to HTML for display (success case)
      eulaContent.value = response.content
        .replace(/^# (.*$)/gim, '<h1>$1</h1>')
        .replace(/^## (.*$)/gim, '<h2>$1</h2>')
        .replace(/^### (.*$)/gim, '<h3>$1</h3>')
        .replace(/^#### (.*$)/gim, '<h4>$1</h4>')
        .replace(/^##### (.*$)/gim, '<h5>$1</h5>')
        .replace(/^###### (.*$)/gim, '<h6>$1</h6>')
        .replace(/\*\*(.*?)\*\*/g, '<strong>$1</strong>')
        .replace(/\*(.*?)\*/g, '<em>$1</em>')
        .replace(/^- (.*$)/gim, '<li>$1</li>')
        .replace(/(<li>.*<\/li>)/s, '<ul>$1</ul>')
        .replace(/\n\n/g, '</p><p>')
        .replace(/\n/g, '<br>')
        .replace(/^/, '<p>')
        .replace(/$/, '</p>')
        .replace(/<p><\/p>/g, '')
        .replace(/<p><ul>/g, '<ul>')
        .replace(/<\/ul><\/p>/g, '</ul>');
      eulaLoadedSuccessfully.value = true;
    } else {
      throw new Error('No EULA content received from backend');
    }
  } catch (error) {
    await logError(`Failed to load EULA content: ${String(error)}`);
    // Show error message instead of fallback EULA
    eulaContent.value = `
      <h3>Unable to Load Terms of Service</h3>
      <p>We're currently unable to load the Terms of Service. Please check your internet connection and try again later.</p>
      <p>If the problem persists, please contact support.</p>
    `;
    eulaLoadedSuccessfully.value = false;
  }
};

// Lifecycle
onMounted(async () => {
  await debug('WelcomeModal: Component mounted');
  await debug(`WelcomeModal: Props show = ${props.show}`);
  await debug(`WelcomeModal: Current mode = ${currentMode.value}`);

  // Load EULA content
  await loadEulaContent();
});

onUnmounted(() => {});
</script>

<style scoped>
.modal-overlay {
  position: fixed;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  background-color: rgba(0, 0, 0, 0.7);
  display: flex;
  justify-content: center;
  align-items: center;
  z-index: 1000;
}

.modal-container {
  background: linear-gradient(135deg, #1e1e1e 0%, #2d2d2d 100%);
  color: #e0e0e0;
  padding: 1.5rem;
  border-radius: 12px;
  max-width: 90vw;
  max-height: 85vh;
  width: 700px;
  height: auto;
  box-shadow: 0 20px 40px rgba(0, 0, 0, 0.5);
  border: 1px solid #333;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

@media (max-width: 768px) {
  .modal-container {
    width: 95vw;
    padding: 1rem;
    max-height: 90vh;
  }
}

@media (max-width: 480px) {
  .modal-container {
    width: 98vw;
    padding: 0.75rem;
    max-height: 95vh;
  }
}

.header-content {
  text-align: center;
  margin-bottom: 0.75rem;
  flex-shrink: 0;
}

.header-row {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 1rem;
}

.logo {
  width: 60px;
  height: 60px;
  object-fit: contain;
}

.app-title {
  font-size: 1.5rem;
  font-weight: 600;
  margin: 0;
  color: #ffffff;
}

.modal-content {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow-y: auto;
  min-height: 0;
}

.auth-content,
.verification-content,
.eula-content {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.step-header {
  text-align: center;
  margin-bottom: 0.75rem;
}

.step-title {
  font-size: 1.2rem;
  font-weight: 600;
  color: #ffffff;
  margin: 0 0 0.5rem 0;
}

.step-subtitle {
  font-size: 0.9rem;
  color: #b0b0b0;
  margin: 0;
}

.form-actions {
  margin-top: 0.75rem;
}

.mode-switch {
  text-align: center;
  margin-top: 0.75rem;
}

.error-alert {
  margin-top: 1rem;
  background: rgba(244, 67, 54, 0.1);
  border: 1px solid #f44336;
  border-radius: 6px;
  padding: 0.75rem;
  color: #f44336;
}

.verification-actions {
  text-align: center;
  margin-top: 1rem;
}

.eula-container {
  border: 1px solid #444;
  border-radius: 6px;
  margin-bottom: 1.5rem;
  background: rgba(255, 255, 255, 0.05);
}

.eula-text {
  padding: 1rem;
  line-height: 1.6;
  max-height: 200px;
  overflow-y: auto;
  color: #e0e0e0;
}

.eula-content h3,
.eula-content h4 {
  margin: 1rem 0 0.5rem 0;
  color: #ffffff;
}

.eula-content p {
  margin: 0.5rem 0;
}

.eula-form {
  margin-bottom: 1.5rem;
}

.error-actions {
  margin-top: 1rem;
  text-align: center;
}

.auth-check-content {
  text-align: center;
  margin: 1rem 0;
}

.loading-spinner {
  display: flex;
  justify-content: center;
  margin-bottom: 1rem;
}

.spinner {
  width: 40px;
  height: 40px;
  border: 3px solid rgba(56, 152, 38, 0.3);
  border-top: 3px solid #389826;
  border-radius: 50%;
  animation: spin 1s linear infinite;
}

@keyframes spin {
  0% {
    transform: rotate(0deg);
  }
  100% {
    transform: rotate(360deg);
  }
}

.checking-text {
  color: #b0b0b0;
  font-size: 0.9rem;
  margin: 0;
}

/* Form styling */
:deep(.n-form-item) {
  margin-bottom: 0.5rem;
}

:deep(.n-form-item-label) {
  color: #e0e0e0 !important;
  font-weight: 500;
}

:deep(.n-input) {
  background: rgba(255, 255, 255, 0.1) !important;
  border: 1px solid #444 !important;
  color: #e0e0e0 !important;
}

:deep(.n-input:focus) {
  border-color: #389826 !important;
}

:deep(.n-input::placeholder) {
  color: #888 !important;
}

:deep(.n-button) {
  background: #389826 !important;
  border: none !important;
  color: white !important;
  font-weight: 500 !important;
}

:deep(.n-button:hover) {
  background: #4caf50 !important;
}

:deep(.n-button:disabled) {
  background: #555 !important;
  color: #888 !important;
}

:deep(.n-checkbox) {
  color: #e0e0e0 !important;
}

:deep(.n-checkbox-checked) {
  background: #389826 !important;
  border-color: #389826 !important;
}
</style>
