import { expect, test, type Page } from '@playwright/test'

async function waitForAtelier(page: Page) {
  await page.goto('/?tab=atelier')
  await page.waitForFunction(
    () => document.querySelector('.wasm-status')?.textContent?.includes('WASM'),
    { timeout: 15_000 },
  )
  await page.waitForSelector('.compile-time', { timeout: 10_000 })
  await expect(page.locator('.code-output .code-content').first()).toBeVisible()
}

async function getCodeText(page: Page) {
  return (await page.locator('.code-output .code-content').first().textContent()) ?? ''
}

test('atelier code targets expose VDOM, SSR, and Vapor outputs with stable toggles', async ({ page }) => {
  await waitForAtelier(page)

  const vdomButton = page.getByRole('button', { name: 'VDOM' })
  const ssrButton = page.getByRole('button', { name: 'SSR' })
  const vaporButton = page.getByRole('button', { name: 'Vapor' })
  const vaporSsrButton = page.getByRole('button', { name: 'Vapor SSR' })
  const tsButton = page.locator('.code-view-toggle').getByRole('button', { name: 'TS' })
  const jsButton = page.locator('.code-view-toggle').getByRole('button', { name: 'JS' })

  await expect(vdomButton).toBeVisible()
  await expect(ssrButton).toBeVisible()
  await expect(vaporButton).toBeVisible()
  await expect(vaporSsrButton).toHaveCount(0)
  await expect(tsButton).toBeEnabled()
  await expect(jsButton).toBeEnabled()

  await ssrButton.click()
  await expect(page.locator('.code-header h4')).toHaveText('SSR Output')
  await expect(tsButton).toBeDisabled()
  await expect(jsButton).toBeDisabled()
  await expect(page.getByText('SFC Output')).toHaveCount(0)

  const ssrCode = await getCodeText(page)
  expect(ssrCode).toContain('ssrRender')
  expect(ssrCode).not.toContain('_ctx.name')
  expect(ssrCode).not.toContain('_ctx.doubled')

  await vaporButton.click()
  await expect(page.locator('.code-header h4')).toHaveText('Vapor Output')
  await expect(tsButton).toBeDisabled()
  await expect(jsButton).toBeDisabled()
  await expect(page.getByText('Template Fragments')).toHaveCount(0)
  await expect(page.getByText('SFC Output')).toHaveCount(0)

  const vaporCode = await getCodeText(page)
  expect(vaporCode).toContain('const t0 = _template')
  expect(vaporCode).toContain('_renderEffect')
})
