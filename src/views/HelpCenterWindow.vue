<template>
  <div
    class="help-center-window h-screen overflow-y-auto overflow-x-hidden bg-base-100 text-base-content"
  >
    <div class="help-center-backdrop"></div>

    <main
      class="relative mx-auto flex min-h-full w-full max-w-7xl flex-col gap-8 px-4 py-6 sm:px-6 lg:px-8"
    >
      <header class="help-center-hero overflow-hidden rounded-[2rem] border border-base-300/70">
        <div
          class="flex flex-col gap-8 p-6 sm:p-8 lg:flex-row lg:items-start lg:justify-between lg:p-10"
        >
          <div class="max-w-3xl space-y-5">
            <div
              class="inline-flex items-center rounded-full border border-primary/20 bg-primary/10 px-4 py-1 text-xs font-semibold uppercase tracking-[0.24em] text-primary"
            >
              {{ content.badge }}
            </div>

            <div class="space-y-4">
              <h1
                class="max-w-2xl text-3xl font-black leading-tight text-balance sm:text-4xl lg:text-5xl"
              >
                {{ content.title }}
              </h1>
              <p class="max-w-2xl text-sm leading-7 text-base-content/75 sm:text-base">
                {{ content.description }}
              </p>
            </div>

            <div class="flex flex-wrap gap-3 text-sm text-base-content/70">
              <span class="rounded-full border border-base-300 bg-base-100/80 px-4 py-2">
                <i class="fas fa-circle-info mr-2 text-primary"></i>{{ content.releaseNote }}
              </span>
              <span class="rounded-full border border-base-300 bg-base-100/80 px-4 py-2">
                <i class="fas fa-compass mr-2 text-secondary"></i
                >{{ t('common.tour.documentation') }}
              </span>
            </div>
          </div>

          <div class="flex shrink-0 flex-wrap items-start gap-3 lg:justify-end">
            <button class="btn btn-primary btn-sm sm:btn-md" type="button" @click="closeWindow">
              <i class="fas fa-xmark"></i>
              {{ t('common.close') }}
            </button>
          </div>
        </div>

        <div class="grid gap-px border-t border-base-300/70 bg-base-300/70 sm:grid-cols-3">
          <article
            v-for="stat in content.stats"
            :key="stat.label"
            class="bg-base-100/85 px-6 py-5 backdrop-blur"
          >
            <div class="text-xs uppercase tracking-[0.24em] text-base-content/45">
              {{ stat.label }}
            </div>
            <div class="mt-2 text-lg font-semibold text-base-content">{{ stat.value }}</div>
          </article>
        </div>
      </header>

      <section class="grid gap-6 lg:grid-cols-[0.95fr_1.05fr]">
        <article
          class="rounded-[1.75rem] border border-base-300 bg-base-100/90 p-6 shadow-sm sm:p-7"
        >
          <div
            class="mb-5 inline-flex items-center rounded-full bg-secondary/10 px-3 py-1 text-xs font-semibold uppercase tracking-[0.2em] text-secondary"
          >
            {{ content.workflowTitle }}
          </div>
          <h2 class="text-2xl font-bold">{{ content.workflowTitle }}</h2>
          <p class="mt-3 text-sm leading-7 text-base-content/70">
            {{ content.workflowDescription }}
          </p>
        </article>

        <div class="grid gap-4">
          <article
            v-for="step in content.workflowSteps"
            :key="step.title"
            class="rounded-[1.5rem] border border-base-300 bg-base-100/90 p-5 shadow-sm transition-transform duration-200 hover:-translate-y-0.5"
          >
            <h3 class="text-base font-semibold text-base-content">{{ step.title }}</h3>
            <p class="mt-2 text-sm leading-7 text-base-content/70">{{ step.description }}</p>
          </article>
        </div>
      </section>

      <section class="space-y-5">
        <div class="space-y-3">
          <div
            class="inline-flex items-center rounded-full bg-accent/10 px-3 py-1 text-xs font-semibold uppercase tracking-[0.2em] text-accent"
          >
            {{ content.modulesTitle }}
          </div>
          <div class="max-w-3xl">
            <h2 class="text-2xl font-bold">{{ content.modulesTitle }}</h2>
            <p class="mt-2 text-sm leading-7 text-base-content/70">
              {{ content.modulesDescription }}
            </p>
          </div>
        </div>

        <div class="grid gap-4 xl:grid-cols-2">
          <article
            v-for="module in content.modules"
            :key="`${module.title}-${module.route}`"
            class="rounded-[1.5rem] border border-base-300 bg-base-100/90 p-5 shadow-sm"
          >
            <div class="flex items-start gap-4">
              <div
                class="flex h-12 w-12 shrink-0 items-center justify-center rounded-2xl bg-primary/12 text-primary"
              >
                <i :class="[module.icon, 'text-lg']"></i>
              </div>

              <div class="min-w-0 flex-1">
                <div class="flex flex-wrap items-center gap-2">
                  <h3 class="text-lg font-semibold">{{ module.title }}</h3>
                  <span
                    class="rounded-full border border-base-300 px-3 py-1 text-xs text-base-content/55"
                  >
                    {{ module.route }}
                  </span>
                </div>
                <p class="mt-3 text-sm leading-7 text-base-content/72">{{ module.summary }}</p>

                <div class="mt-4 flex flex-wrap gap-2">
                  <span
                    v-for="scenario in module.scenarios"
                    :key="scenario"
                    class="rounded-full bg-base-200 px-3 py-1.5 text-xs text-base-content/70"
                  >
                    {{ scenario }}
                  </span>
                </div>
              </div>
            </div>
          </article>
        </div>
      </section>

      <section class="grid gap-6 lg:grid-cols-[0.95fr_1.05fr]">
        <article
          class="rounded-[1.75rem] border border-base-300 bg-base-100/90 p-6 shadow-sm sm:p-7"
        >
          <div
            class="inline-flex items-center rounded-full bg-primary/10 px-3 py-1 text-xs font-semibold uppercase tracking-[0.2em] text-primary"
          >
            {{ content.shortcutsTitle }}
          </div>
          <h2 class="mt-4 text-2xl font-bold">{{ content.shortcutsTitle }}</h2>
          <p class="mt-3 text-sm leading-7 text-base-content/70">
            {{ content.shortcutsDescription }}
          </p>
        </article>

        <div class="grid gap-4">
          <article
            v-for="shortcut in content.shortcuts"
            :key="shortcut.keys"
            class="rounded-[1.5rem] border border-base-300 bg-base-100/90 p-5 shadow-sm"
          >
            <div class="text-sm font-semibold text-base-content">{{ shortcut.keys }}</div>
            <p class="mt-2 text-sm leading-7 text-base-content/70">{{ shortcut.description }}</p>
          </article>
        </div>
      </section>

      <section class="space-y-5 pb-4">
        <div class="space-y-3">
          <div
            class="inline-flex items-center rounded-full bg-secondary/10 px-3 py-1 text-xs font-semibold uppercase tracking-[0.2em] text-secondary"
          >
            {{ content.faqTitle }}
          </div>
          <div class="max-w-3xl">
            <h2 class="text-2xl font-bold">{{ content.faqTitle }}</h2>
            <p class="mt-2 text-sm leading-7 text-base-content/70">{{ content.faqDescription }}</p>
          </div>
        </div>

        <div class="grid gap-4 xl:grid-cols-2">
          <article
            v-for="faq in content.faqs"
            :key="faq.question"
            class="rounded-[1.5rem] border border-base-300 bg-base-100/90 p-5 shadow-sm"
          >
            <h3 class="text-base font-semibold text-base-content">{{ faq.question }}</h3>
            <p class="mt-3 text-sm leading-7 text-base-content/72">{{ faq.answer }}</p>
          </article>
        </div>
      </section>

      <footer class="pb-6 text-sm text-base-content/60">
        {{ content.closingNote }}
      </footer>
    </main>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow'
import { useI18n } from 'vue-i18n'
import { getHelpCenterContent } from '@/views/help-center/helpCenterContent'

const { locale, t } = useI18n()

const content = computed(() => getHelpCenterContent(locale.value))

async function closeWindow() {
  try {
    await getCurrentWebviewWindow().close()
  } catch {
    window.close()
  }
}
</script>

<style scoped>
.help-center-window {
  position: relative;
  scrollbar-gutter: stable;
  background:
    radial-gradient(circle at top left, rgb(14 165 233 / 0.16), transparent 28%),
    radial-gradient(circle at top right, rgb(251 191 36 / 0.14), transparent 24%),
    linear-gradient(180deg, rgb(248 250 252 / 0.96), transparent 28rem);
}

.help-center-backdrop {
  position: fixed;
  inset: 0;
  pointer-events: none;
  background-image:
    linear-gradient(rgb(15 23 42 / 0.03) 1px, transparent 1px),
    linear-gradient(90deg, rgb(15 23 42 / 0.03) 1px, transparent 1px);
  background-size: 28px 28px;
  mask-image: linear-gradient(180deg, rgb(0 0 0 / 0.9), transparent 90%);
}

.help-center-hero {
  background:
    linear-gradient(135deg, rgb(255 255 255 / 0.92), rgb(248 250 252 / 0.86)),
    linear-gradient(120deg, rgb(14 165 233 / 0.08), rgb(251 191 36 / 0.08));
  box-shadow: 0 18px 48px rgb(15 23 42 / 0.08);
}

@media (prefers-color-scheme: dark) {
  .help-center-window {
    background:
      radial-gradient(circle at top left, rgb(14 165 233 / 0.16), transparent 28%),
      radial-gradient(circle at top right, rgb(251 191 36 / 0.1), transparent 24%),
      linear-gradient(180deg, rgb(10 15 23 / 0.98), transparent 28rem);
  }

  .help-center-backdrop {
    background-image:
      linear-gradient(rgb(255 255 255 / 0.03) 1px, transparent 1px),
      linear-gradient(90deg, rgb(255 255 255 / 0.03) 1px, transparent 1px);
  }

  .help-center-hero {
    background:
      linear-gradient(135deg, rgb(15 23 42 / 0.92), rgb(17 24 39 / 0.88)),
      linear-gradient(120deg, rgb(14 165 233 / 0.08), rgb(251 191 36 / 0.08));
    box-shadow: 0 18px 48px rgb(2 6 23 / 0.32);
  }
}
</style>
