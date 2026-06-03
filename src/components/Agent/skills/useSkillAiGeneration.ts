import { computed, ref, type Ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { buildSkillPrompt, type TranslateFn } from '../skillsManagerHelpers'
import type { SkillCandidate, SkillForm } from './skillsManagerTypes'

interface UseSkillAiGenerationOptions {
  t: TranslateFn
  locale: Ref<string>
  editingSkill: Ref<SkillForm | null>
  briefDescription: Ref<string>
  candidateRefinementContext: Ref<SkillCandidate | null>
  hasExistingContent: Ref<boolean>
  nameError: Ref<string>
  descriptionError: Ref<string>
}

export const useSkillAiGeneration = ({
  t,
  locale,
  editingSkill,
  briefDescription,
  candidateRefinementContext,
  hasExistingContent,
  nameError,
  descriptionError,
}: UseSkillAiGenerationOptions) => {
  const aiGenerating = ref(false)
  const generatedContent = ref('')

  const canUseAI = computed(() =>
    hasExistingContent.value || briefDescription.value.trim().length > 0
  )

  const aiButtonText = computed(() => {
    if (aiGenerating.value) return t('agent.aiGenerating')
    if (briefDescription.value.trim()) return t('agent.aiGenerate')
    return hasExistingContent.value ? t('agent.aiExpand') : t('agent.aiGenerate')
  })

  const runAiGeneration = async (prompt: string, systemPrompt: string) => {
    const streamId = crypto.randomUUID()
    let unlistenDelta: (() => void) | undefined
    let unlistenComplete: (() => void) | undefined
    let unlistenError: (() => void) | undefined

    aiGenerating.value = true
    generatedContent.value = ''

    try {
      unlistenDelta = await listen('plugin_gen_delta', event => {
        const payload = event.payload as any
        if (payload.stream_id === streamId) {
          generatedContent.value += payload.delta
        }
      })

      unlistenComplete = await listen('plugin_gen_complete', event => {
        const payload = event.payload as any
        if (payload.stream_id === streamId) {
          try {
            let jsonStr = generatedContent.value.trim()
            const jsonMatch = jsonStr.match(/```(?:json)?\s*(\{[\s\S]*?\})\s*```/)
            if (jsonMatch) {
              jsonStr = jsonMatch[1]
            } else if (jsonStr.startsWith('```')) {
              jsonStr = jsonStr.replace(/^```(?:json)?/, '').replace(/```$/, '')
            }

            const data = JSON.parse(jsonStr)
            if (editingSkill.value) {
              editingSkill.value.name = data.name || editingSkill.value.name
              editingSkill.value.description = data.description || editingSkill.value.description
              editingSkill.value.argument_hint = data.argument_hint || ''
              editingSkill.value.content = data.content || ''
              editingSkill.value.disable_model_invocation = !!data.disable_model_invocation
              editingSkill.value.user_invocable = data.user_invocable !== false
              editingSkill.value.model = data.model || ''
              editingSkill.value.context = data.context || ''
              editingSkill.value.agent = data.agent || ''
              editingSkill.value.hooks_raw = JSON.stringify(data.hooks || {}, null, 2)

              const invalidFields: string[] = []
              if (nameError.value) invalidFields.push(t('agent.skillName'))
              if (descriptionError.value) invalidFields.push(t('agent.skillDescription'))
              if (invalidFields.length > 0) {
                alert(t('agent.aiGeneratedInvalid', { fields: invalidFields.join(', ') }))
              }
            }
          } catch (error) {
            console.error('Failed to parse AI response', error)
          } finally {
            aiGenerating.value = false
            if (unlistenDelta) unlistenDelta()
            if (unlistenComplete) unlistenComplete()
            if (unlistenError) unlistenError()
          }
        }
      })

      unlistenError = await listen('plugin_gen_error', event => {
        const payload = event.payload as any
        if (payload.stream_id === streamId) {
          console.error('AI Generation Error:', payload.error)
          aiGenerating.value = false
          if (unlistenDelta) unlistenDelta()
          if (unlistenComplete) unlistenComplete()
          if (unlistenError) unlistenError()
        }
      })

      await invoke('generate_plugin_stream', {
        request: {
          stream_id: streamId,
          message: prompt,
          system_prompt: systemPrompt,
          service_name: 'default',
        },
      })
    } catch (error) {
      console.error(error)
      aiGenerating.value = false
      if (unlistenDelta) unlistenDelta()
      if (unlistenComplete) unlistenComplete()
      if (unlistenError) unlistenError()
    }
  }

  const generateWithAI = async () => {
    if (!editingSkill.value) return
    if (!canUseAI.value) {
      alert(t('agent.selectToolsOrAddContent'))
      return
    }

    const { prompt, systemPrompt } = buildSkillPrompt({
      brief: briefDescription.value.trim(),
      hasExistingContent: hasExistingContent.value,
      existingName: editingSkill.value.name,
      existingDescription: editingSkill.value.description,
      existingArgumentHint: editingSkill.value.argument_hint,
      existingContent: editingSkill.value.content,
      existingModel: editingSkill.value.model,
      existingContext: editingSkill.value.context,
      existingAgent: editingSkill.value.agent,
      existingHooks: editingSkill.value.hooks_raw,
      candidateContext: candidateRefinementContext.value,
      locale: locale.value,
    })
    await runAiGeneration(prompt, systemPrompt)
  }

  return {
    aiGenerating,
    canUseAI,
    aiButtonText,
    generateWithAI,
  }
}
