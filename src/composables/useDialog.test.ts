import { beforeEach, describe, expect, it, vi } from 'vitest'
import { dialog } from './useDialog'

describe('dialog service', () => {
  beforeEach(() => {
    document.body.innerHTML = ''
    ;(dialog as any).modalContainer = null

    HTMLDialogElement.prototype.showModal = vi.fn(function (this: HTMLDialogElement) {
      this.open = true
    })
    HTMLDialogElement.prototype.close = vi.fn(function (this: HTMLDialogElement) {
      this.open = false
    })
  })

  it('keeps confirm bound when passed as a standalone callback', async () => {
    const confirm = dialog.confirm

    const result = confirm({
      message: 'Delete skill?',
      confirmText: 'Delete',
      cancelText: 'Cancel',
    })

    const modal = document.getElementById('global-dialog-modal')
    expect(modal?.textContent).toContain('Delete skill?')
    modal
      ?.querySelector<HTMLButtonElement>('#dialog-confirm-btn')
      ?.dispatchEvent(new MouseEvent('click', { bubbles: true }))

    await expect(result).resolves.toBe(true)
  })
})
