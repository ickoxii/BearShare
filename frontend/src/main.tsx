import { render } from 'preact'
import './styles.css'
import { initLegacyApp } from './legacy-app'

window.addEventListener('DOMContentLoaded', async () => {
  const root = document.getElementById('app-root')

  // Optional Preact mount point (can replace later with real components)
  if (root) {
    render(null, root)
  }

  try {
    await initLegacyApp()
    console.log('BearShare legacy app initialized')
  } catch (err) {
    console.error('Failed to initialize app', err)
  }
})
