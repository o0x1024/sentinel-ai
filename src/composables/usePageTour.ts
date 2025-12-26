import { driver, DriveStep, Config } from 'driver.js'
import 'driver.js/dist/driver.css'
import { useI18n } from 'vue-i18n'

export interface TourStep extends Omit<DriveStep, 'popover'> {
  element: string
  popover: {
    title: string
    description: string
    side?: 'top' | 'right' | 'bottom' | 'left'
    align?: 'start' | 'center' | 'end'
  }
}

export function usePageTour() {
  const { t } = useI18n()

  const createTour = (steps: TourStep[], onComplete?: () => void) => {
    const driverConfig: Config = {
      showProgress: true,
      showButtons: ['next', 'previous', 'close'],
      steps: steps.map(step => ({
        ...step,
        popover: {
          ...step.popover,
          title: step.popover.title,
          description: step.popover.description,
        }
      })),
      nextBtnText: t('common.tour.next'),
      prevBtnText: t('common.tour.previous'),
      doneBtnText: t('common.tour.done'),
      progressText: t('common.tour.progress'),
      onDestroyStarted: () => {
        if (driverObj.hasNextStep() || driverObj.hasPreviousStep()) {
          const confirmed = confirm(t('common.tour.confirmExit'))
          if (!confirmed) {
            return
          }
        }
        driverObj.destroy()
        if (onComplete) {
          onComplete()
        }
      }
    }

    const driverObj = driver(driverConfig)
    return driverObj
  }

  const startTour = (tourKey: string, steps: TourStep[]) => {
    // 检查用户是否已经看过这个向导
    const tourCompleted = localStorage.getItem(`tour_completed_${tourKey}`)
    
    if (tourCompleted === 'true') {
      return null
    }

    const tour = createTour(steps, () => {
      // 标记向导已完成
      localStorage.setItem(`tour_completed_${tourKey}`, 'true')
    })

    // 延迟启动，确保 DOM 已渲染
    setTimeout(() => {
      tour.drive()
    }, 500)

    return tour
  }

  const resetTour = (tourKey: string) => {
    localStorage.removeItem(`tour_completed_${tourKey}`)
  }

  const manualStartTour = (steps: TourStep[]) => {
    const tour = createTour(steps)
    setTimeout(() => {
      tour.drive()
    }, 100)
    return tour
  }

  return {
    createTour,
    startTour,
    resetTour,
    manualStartTour
  }
}

