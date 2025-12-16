<template>
  <div class="package-card" :class="cardClass" @click="handleCardClick">
    <div class="package-header">
      <div class="package-info">
        <h3 class="package-name">{{ packageData.package.name }}</h3>
        <div
          class="package-stats"
          v-if="packageData.package.stars || packageData.package.topics.length"
        >
          <span v-if="packageData.package.stars" class="stars">
            <n-icon><Star /></n-icon>
            {{ formatNumber(packageData.package.stars) }}
          </span>
          <span v-if="packageData.package.topics.length" class="topics-count">
            <n-icon><Pricetag /></n-icon>
            {{ packageData.package.topics.length }}
          </span>
        </div>
      </div>
      <div class="package-actions">
        <n-button
          v-if="!packageData.is_installed || packageData.is_direct === false"
          type="primary"
          size="small"
          @click.stop="handleAddPackage"
          :loading="addingPackage"
        >
          <template #icon>
            <n-icon><Add /></n-icon>
          </template>
          {{ packageData.is_installed && packageData.is_direct === false ? 'Make Direct' : 'Add' }}
        </n-button>
        <n-button v-else-if="packageData.is_direct === true" type="success" size="small" disabled>
          <template #icon>
            <n-icon><Checkmark /></n-icon>
          </template>
          Installed
        </n-button>
        <n-button size="small" @click.stop="handleShowDetails">
          <template #icon>
            <n-icon><InformationCircle /></n-icon>
          </template>
          Details
        </n-button>
      </div>
    </div>

    <p class="package-description" v-if="packageData.package.description">
      {{ packageData.package.description }}
    </p>

    <div class="package-tags" v-if="packageData.package.topics.length > 0">
      <n-tag v-for="topic in displayTopics" :key="topic" size="small" type="info" class="topic-tag">
        {{ topic }}
      </n-tag>
      <n-tag
        v-if="packageData.package.topics.length > maxTopics"
        size="small"
        type="default"
        class="more-topics"
      >
        +{{ packageData.package.topics.length - maxTopics }} more
      </n-tag>
    </div>

    <div class="package-recommendation" v-if="packageData.reason && packageData.relevance_score">
      <div class="recommendation-header">
        <n-icon class="recommendation-icon" :class="recommendationIconClass">
          <Bulb />
        </n-icon>
        <span class="recommendation-reason">{{ packageData.reason }}</span>
      </div>
      <div class="relevance-score">
        <n-progress
          type="circle"
          :percentage="Math.round(packageData.relevance_score * 100)"
          :show-indicator="false"
          size="small"
          :color="relevanceScoreColor"
        />
        <span class="score-label">Relevance</span>
      </div>
    </div>

    <div class="package-footer" v-if="packageData.package.repository_url">
      <a
        :href="packageData.package.repository_url"
        target="_blank"
        rel="noopener noreferrer"
        @click.stop
        class="repository-link"
      >
        <n-icon><LogoGithub /></n-icon>
        View on GitHub
      </a>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue';
import { NButton, NIcon, NTag, NProgress, useMessage } from 'naive-ui';
import {
  Star,
  Pricetag,
  Add,
  Checkmark,
  InformationCircle,
  Bulb,
  LogoGithub,
} from '@vicons/ionicons5';
import type { PackageCardData } from '../../types/packageTypes';

interface Props {
  packageData: PackageCardData;
  maxTopics?: number;
}

const props = withDefaults(defineProps<Props>(), {
  maxTopics: 3,
});

const emit = defineEmits<{
  'add-package': [packageName: string];
  'show-details': [packageData: PackageCardData];
  'card-click': [packageData: PackageCardData];
}>();

const message = useMessage();
const addingPackage = ref(false);

// Computed properties
const cardClass = computed(() => ({
  'package-card': true,
  'is-trending': props.packageData.is_trending,
  'is-recommended': props.packageData.relevance_score && props.packageData.relevance_score > 0.5,
  'is-installed': props.packageData.is_installed,
}));

const displayTopics = computed(() => props.packageData.package.topics.slice(0, props.maxTopics));

const recommendationIconClass = computed(() => {
  if (!props.packageData.category) return '';

  switch (props.packageData.category) {
    case 'essential':
      return 'essential';
    case 'ecosystem':
      return 'ecosystem';
    case 'complementary':
      return 'complementary';
    default:
      return 'general';
  }
});

const relevanceScoreColor = computed(() => {
  if (!props.packageData.relevance_score) return '#18a058';

  const score = props.packageData.relevance_score;
  if (score > 0.8) return '#18a058'; // Green for high relevance
  if (score > 0.6) return '#f0a020'; // Orange for medium relevance
  return '#d03050'; // Red for low relevance
});

// Methods
const formatNumber = (num: number): string => {
  if (num >= 1000) {
    return (num / 1000).toFixed(1) + 'k';
  }
  return num.toString();
};

const handleCardClick = () => {
  emit('card-click', props.packageData);
};

const handleAddPackage = async () => {
  if (addingPackage.value) return;

  addingPackage.value = true;
  try {
    emit('add-package', props.packageData.package.name);
    message.success(`Added ${props.packageData.package.name} to project`);
  } catch (error) {
    message.error(`Failed to add ${props.packageData.package.name}`);
  } finally {
    addingPackage.value = false;
  }
};

const handleShowDetails = () => {
  emit('show-details', props.packageData);
};
</script>

<style scoped>
.package-card {
  background: var(--n-card-color);
  border: 1px solid var(--n-border-color);
  border-radius: 8px;
  padding: 16px;
  transition: all 0.2s ease;
  cursor: pointer;
  position: relative;
}

.package-card:hover {
  border-color: var(--n-primary-color);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
  transform: translateY(-2px);
}

.package-card.is-trending {
  border-color: #f0a020;
  background: linear-gradient(135deg, var(--n-card-color) 0%, rgba(240, 160, 32, 0.05) 100%);
}

.package-card.is-recommended {
  border-color: #18a058;
  background: linear-gradient(135deg, var(--n-card-color) 0%, rgba(24, 160, 88, 0.05) 100%);
}

.package-card.is-installed {
  border-color: #18a058;
  background: linear-gradient(135deg, var(--n-card-color) 0%, rgba(24, 160, 88, 0.1) 100%);
}

.package-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  margin-bottom: 12px;
  gap: 12px;
}

.package-info {
  flex: 1;
  min-width: 0;
}

.package-name {
  font-size: 1.1rem;
  font-weight: 600;
  color: var(--n-text-color);
  margin: 0 0 8px 0;
  word-break: break-word;
}

.package-stats {
  display: flex;
  gap: 12px;
  align-items: center;
}

.package-stats .stars,
.package-stats .topics-count {
  display: flex;
  align-items: center;
  gap: 4px;
  font-size: 0.85rem;
  color: var(--n-text-color-3);
}

.package-actions {
  display: flex;
  gap: 8px;
  flex-shrink: 0;
}

.package-description {
  color: var(--n-text-color-2);
  font-size: 0.9rem;
  line-height: 1.4;
  margin: 0 0 12px 0;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

.package-tags {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  margin-bottom: 12px;
}

.topic-tag {
  font-size: 0.75rem;
}

.more-topics {
  font-size: 0.75rem;
  opacity: 0.7;
}

.package-recommendation {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 8px 12px;
  background: var(--n-color);
  border-radius: 6px;
  margin-bottom: 12px;
}

.recommendation-header {
  display: flex;
  align-items: center;
  gap: 8px;
  flex: 1;
}

.recommendation-icon {
  font-size: 1rem;
}

.recommendation-icon.essential {
  color: #18a058;
}

.recommendation-icon.ecosystem {
  color: #2080f0;
}

.recommendation-icon.complementary {
  color: #f0a020;
}

.recommendation-icon.general {
  color: var(--n-text-color-3);
}

.recommendation-reason {
  font-size: 0.85rem;
  color: var(--n-text-color-2);
  line-height: 1.3;
}

.relevance-score {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
  flex-shrink: 0;
}

.score-label {
  font-size: 0.7rem;
  color: var(--n-text-color-3);
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.package-footer {
  display: flex;
  justify-content: flex-end;
  padding-top: 8px;
  border-top: 1px solid var(--n-border-color);
}

.repository-link {
  display: flex;
  align-items: center;
  gap: 6px;
  color: var(--n-text-color-3);
  text-decoration: none;
  font-size: 0.85rem;
  transition: color 0.2s ease;
}

.repository-link:hover {
  color: var(--n-primary-color);
}

/* Responsive design */
@media (max-width: 768px) {
  .package-header {
    flex-direction: column;
    align-items: stretch;
    gap: 12px;
  }

  .package-actions {
    justify-content: stretch;
  }

  .package-actions .n-button {
    flex: 1;
  }

  .package-recommendation {
    flex-direction: column;
    align-items: stretch;
    gap: 8px;
  }

  .relevance-score {
    align-self: center;
  }
}
</style>
