<template>
  <div class="help-button-container">
    <button
      @click="toggleMenu"
      class="help-button"
      :title="t('common.tour.help')"
    >
      <svg
        xmlns="http://www.w3.org/2000/svg"
        class="h-5 w-5"
        viewBox="0 0 20 20"
        fill="currentColor"
      >
        <path
          fill-rule="evenodd"
          d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-8-3a1 1 0 00-.867.5 1 1 0 11-1.731-1A3 3 0 0113 8a3.001 3.001 0 01-2 2.83V11a1 1 0 11-2 0v-1a1 1 0 011-1 1 1 0 100-2zm0 8a1 1 0 100-2 1 1 0 000 2z"
          clip-rule="evenodd"
        />
      </svg>
    </button>

    <Transition name="menu-fade">
      <div v-if="showMenu" class="help-menu" @click.stop>
        <button @click="startGuide" class="help-menu-item">
          <svg
            xmlns="http://www.w3.org/2000/svg"
            class="h-4 w-4"
            viewBox="0 0 20 20"
            fill="currentColor"
          >
            <path
              d="M10.894 2.553a1 1 0 00-1.788 0l-7 14a1 1 0 001.169 1.409l5-1.429A1 1 0 009 15.571V11a1 1 0 112 0v4.571a1 1 0 00.725.962l5 1.428a1 1 0 001.17-1.408l-7-14z"
            />
          </svg>
          <span>{{ t('common.tour.guide') }}</span>
        </button>

        <button
          v-if="documentationUrl"
          @click="openDocumentation"
          class="help-menu-item"
        >
          <svg
            xmlns="http://www.w3.org/2000/svg"
            class="h-4 w-4"
            viewBox="0 0 20 20"
            fill="currentColor"
          >
            <path
              fill-rule="evenodd"
              d="M4 4a2 2 0 012-2h4.586A2 2 0 0112 2.586L15.414 6A2 2 0 0116 7.414V16a2 2 0 01-2 2H6a2 2 0 01-2-2V4zm2 6a1 1 0 011-1h6a1 1 0 110 2H7a1 1 0 01-1-1zm1 3a1 1 0 100 2h6a1 1 0 100-2H7z"
              clip-rule="evenodd"
            />
          </svg>
          <span>{{ t('common.tour.documentation') }}</span>
        </button>
      </div>
    </Transition>

    <div v-if="showMenu" class="help-overlay" @click="closeMenu"></div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'

interface Props {
  documentationUrl?: string
}

const props = defineProps<Props>()
const emit = defineEmits<{
  startGuide: []
}>()

const { t } = useI18n()
const showMenu = ref(false)

const toggleMenu = () => {
  showMenu.value = !showMenu.value
}

const closeMenu = () => {
  showMenu.value = false
}

const startGuide = () => {
  closeMenu()
  emit('startGuide')
}

const openDocumentation = () => {
  if (props.documentationUrl) {
    window.open(props.documentationUrl, '_blank')
  }
  closeMenu()
}
</script>

<style scoped>
.help-button-container {
  position: relative;
  display: inline-block;
}

.help-button {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 36px;
  height: 36px;
  border-radius: 50%;
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  color: white;
  border: none;
  cursor: pointer;
  transition: all 0.3s ease;
  box-shadow: 0 2px 8px rgba(102, 126, 234, 0.3);
}

.help-button:hover {
  transform: scale(1.1);
  box-shadow: 0 4px 12px rgba(102, 126, 234, 0.5);
}

.help-button:active {
  transform: scale(0.95);
}

.help-menu {
  position: absolute;
  bottom: calc(100% + 8px);
  right: 0;
  background: white;
  border-radius: 8px;
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.15);
  overflow: hidden;
  min-width: 180px;
  z-index: 1001;
}

.help-menu-item {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
  padding: 12px 16px;
  background: white;
  border: none;
  cursor: pointer;
  transition: background 0.2s ease;
  text-align: left;
  font-size: 14px;
  color: #333;
}

.help-menu-item:hover {
  background: #f5f5f5;
}

.help-menu-item svg {
  flex-shrink: 0;
}

.help-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  z-index: 1000;
  background: transparent;
}

.menu-fade-enter-active,
.menu-fade-leave-active {
  transition: all 0.2s ease;
}

.menu-fade-enter-from,
.menu-fade-leave-to {
  opacity: 0;
  transform: translateY(8px);
}

/* Dark mode support */
@media (prefers-color-scheme: dark) {
  .help-menu {
    background: #2d3748;
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.3);
  }

  .help-menu-item {
    background: #2d3748;
    color: #e2e8f0;
  }

  .help-menu-item:hover {
    background: #4a5568;
  }
}
</style>

