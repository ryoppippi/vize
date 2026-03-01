/**
 * CSS theme variables and style definitions for the Musea gallery.
 *
 * Extracted from gallery.ts to keep file sizes manageable.
 */

/**
 * Generate the full gallery CSS styles string.
 */
export function generateGalleryStyles(): string {
  return `
    :root {
      --musea-bg-primary: #E6E2D6;
      --musea-bg-secondary: #ddd9cd;
      --musea-bg-tertiary: #d4d0c4;
      --musea-bg-elevated: #E6E2D6;
      --musea-accent: #121212;
      --musea-accent-hover: #2a2a2a;
      --musea-accent-subtle: rgba(18, 18, 18, 0.08);
      --musea-text: #121212;
      --musea-text-secondary: #3a3a3a;
      --musea-text-muted: #6b6b6b;
      --musea-border: #c8c4b8;
      --musea-border-subtle: #d4d0c4;
      --musea-success: #16a34a;
      --musea-shadow: 0 4px 24px rgba(0, 0, 0, 0.08);
      --musea-radius-sm: 4px;
      --musea-radius-md: 6px;
      --musea-radius-lg: 8px;
      --musea-transition: 0.15s ease;
    }

    * { box-sizing: border-box; margin: 0; padding: 0; }

    body {
      font-family: 'Helvetica Neue', Helvetica, Arial, sans-serif;
      background: var(--musea-bg-primary);
      color: var(--musea-text);
      min-height: 100vh;
      line-height: 1.5;
      -webkit-font-smoothing: antialiased;
    }

    /* Header */
    .header {
      background: var(--musea-bg-secondary);
      border-bottom: 1px solid var(--musea-border);
      padding: 0 1.5rem;
      height: 56px;
      display: flex;
      align-items: center;
      justify-content: space-between;
      position: sticky;
      top: 0;
      z-index: 100;
    }

    .header-left {
      display: flex;
      align-items: center;
      gap: 1.5rem;
    }

    .logo {
      display: flex;
      align-items: center;
      gap: 0.5rem;
      font-size: 1.125rem;
      font-weight: 700;
      color: var(--musea-accent);
      text-decoration: none;
    }

    .logo-svg {
      width: 32px;
      height: 32px;
      flex-shrink: 0;
    }

    .logo-icon svg {
      width: 16px;
      height: 16px;
      color: var(--musea-text);
    }

    .header-subtitle {
      color: var(--musea-text-muted);
      font-size: 0.8125rem;
      font-weight: 500;
      padding-left: 1.5rem;
      border-left: 1px solid var(--musea-border);
    }

    .search-container {
      position: relative;
      width: 280px;
    }

    .search-input {
      width: 100%;
      background: var(--musea-bg-tertiary);
      border: 1px solid var(--musea-border);
      border-radius: var(--musea-radius-md);
      padding: 0.5rem 0.75rem 0.5rem 2.25rem;
      color: var(--musea-text);
      font-size: 0.8125rem;
      outline: none;
      transition: border-color var(--musea-transition), background var(--musea-transition);
    }

    .search-input::placeholder {
      color: var(--musea-text-muted);
    }

    .search-input:focus {
      border-color: var(--musea-accent);
      background: var(--musea-bg-elevated);
    }

    .search-icon {
      position: absolute;
      left: 0.75rem;
      top: 50%;
      transform: translateY(-50%);
      color: var(--musea-text-muted);
      pointer-events: none;
    }

    /* Layout */
    .main {
      display: grid;
      grid-template-columns: 260px 1fr;
      min-height: calc(100vh - 56px);
    }

    /* Sidebar */
    .sidebar {
      background: var(--musea-bg-secondary);
      border-right: 1px solid var(--musea-border);
      overflow-y: auto;
      overflow-x: hidden;
    }

    .sidebar::-webkit-scrollbar {
      width: 6px;
    }

    .sidebar::-webkit-scrollbar-track {
      background: transparent;
    }

    .sidebar::-webkit-scrollbar-thumb {
      background: var(--musea-border);
      border-radius: 3px;
    }

    .sidebar-section {
      padding: 0.75rem;
    }

    .category-header {
      display: flex;
      align-items: center;
      gap: 0.5rem;
      padding: 0.625rem 0.75rem;
      font-size: 0.6875rem;
      font-weight: 600;
      text-transform: uppercase;
      letter-spacing: 0.08em;
      color: var(--musea-text-muted);
      cursor: pointer;
      user-select: none;
      border-radius: var(--musea-radius-sm);
      transition: background var(--musea-transition);
    }

    .category-header:hover {
      background: var(--musea-bg-tertiary);
    }

    .category-icon {
      width: 16px;
      height: 16px;
      transition: transform var(--musea-transition);
    }

    .category-header.collapsed .category-icon {
      transform: rotate(-90deg);
    }

    .category-count {
      margin-left: auto;
      background: var(--musea-bg-tertiary);
      padding: 0.125rem 0.375rem;
      border-radius: 4px;
      font-size: 0.625rem;
    }

    .art-list {
      list-style: none;
      margin-top: 0.25rem;
    }

    .art-item {
      display: flex;
      align-items: center;
      gap: 0.625rem;
      padding: 0.5rem 0.75rem 0.5rem 1.75rem;
      border-radius: var(--musea-radius-sm);
      cursor: pointer;
      font-size: 0.8125rem;
      color: var(--musea-text-secondary);
      transition: all var(--musea-transition);
      position: relative;
    }

    .art-item::before {
      content: '';
      position: absolute;
      left: 0.75rem;
      top: 50%;
      transform: translateY(-50%);
      width: 6px;
      height: 6px;
      border-radius: 50%;
      background: var(--musea-border);
      transition: background var(--musea-transition);
    }

    .art-item:hover {
      background: var(--musea-bg-tertiary);
      color: var(--musea-text);
    }

    .art-item:hover::before {
      background: var(--musea-text-muted);
    }

    .art-item.active {
      background: var(--musea-accent-subtle);
      color: var(--musea-accent-hover);
    }

    .art-item.active::before {
      background: var(--musea-accent);
    }

    .art-variant-count {
      margin-left: auto;
      font-size: 0.6875rem;
      color: var(--musea-text-muted);
      opacity: 0;
      transition: opacity var(--musea-transition);
    }

    .art-item:hover .art-variant-count {
      opacity: 1;
    }

    /* Content */
    .content {
      background: var(--musea-bg-primary);
      overflow-y: auto;
    }

    .content-inner {
      max-width: 1400px;
      margin: 0 auto;
      padding: 2rem;
    }

    .content-header {
      margin-bottom: 2rem;
    }

    .content-title {
      font-size: 1.5rem;
      font-weight: 700;
      margin-bottom: 0.5rem;
    }

    .content-description {
      color: var(--musea-text-muted);
      font-size: 0.9375rem;
      max-width: 600px;
    }

    .content-meta {
      display: flex;
      align-items: center;
      gap: 1rem;
      margin-top: 1rem;
    }

    .meta-tag {
      display: inline-flex;
      align-items: center;
      gap: 0.375rem;
      padding: 0.25rem 0.625rem;
      background: var(--musea-bg-secondary);
      border: 1px solid var(--musea-border);
      border-radius: var(--musea-radius-sm);
      font-size: 0.75rem;
      color: var(--musea-text-muted);
    }

    .meta-tag svg {
      width: 12px;
      height: 12px;
    }

    /* Gallery Grid */
    .gallery {
      display: grid;
      grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
      gap: 1.25rem;
    }

    /* Variant Card */
    .variant-card {
      background: var(--musea-bg-secondary);
      border: 1px solid var(--musea-border);
      border-radius: var(--musea-radius-lg);
      overflow: hidden;
      transition: all var(--musea-transition);
    }

    .variant-card:hover {
      border-color: var(--musea-text-muted);
      box-shadow: var(--musea-shadow);
      transform: translateY(-2px);
    }

    .variant-preview {
      aspect-ratio: 16 / 10;
      background: var(--musea-bg-tertiary);
      display: flex;
      align-items: center;
      justify-content: center;
      position: relative;
      overflow: hidden;
    }

    .variant-preview iframe {
      width: 100%;
      height: 100%;
      border: none;
      background: white;
    }

    .variant-preview-placeholder {
      color: var(--musea-text-muted);
      font-size: 0.8125rem;
      text-align: center;
      padding: 1rem;
    }

    .variant-preview-code {
      font-family: 'JetBrains Mono', 'SF Mono', 'Fira Code', monospace;
      font-size: 0.75rem;
      color: var(--musea-text-muted);
      background: var(--musea-bg-primary);
      padding: 1rem;
      overflow: auto;
      max-height: 100%;
      width: 100%;
    }

    .variant-info {
      padding: 1rem;
      border-top: 1px solid var(--musea-border);
      display: flex;
      align-items: center;
      justify-content: space-between;
    }

    .variant-name {
      font-weight: 600;
      font-size: 0.875rem;
    }

    .variant-badge {
      font-size: 0.625rem;
      font-weight: 600;
      text-transform: uppercase;
      letter-spacing: 0.04em;
      padding: 0.1875rem 0.5rem;
      border-radius: 4px;
      background: var(--musea-accent-subtle);
      color: var(--musea-accent);
    }

    .variant-actions {
      display: flex;
      gap: 0.5rem;
    }

    .variant-action-btn {
      width: 28px;
      height: 28px;
      border: none;
      background: var(--musea-bg-tertiary);
      border-radius: var(--musea-radius-sm);
      color: var(--musea-text-muted);
      cursor: pointer;
      display: flex;
      align-items: center;
      justify-content: center;
      transition: all var(--musea-transition);
    }

    .variant-action-btn:hover {
      background: var(--musea-bg-elevated);
      color: var(--musea-text);
    }

    .variant-action-btn svg {
      width: 14px;
      height: 14px;
    }

    /* Empty State */
    .empty-state {
      display: flex;
      flex-direction: column;
      align-items: center;
      justify-content: center;
      min-height: 400px;
      text-align: center;
      padding: 2rem;
    }

    .empty-state-icon {
      width: 80px;
      height: 80px;
      background: var(--musea-bg-secondary);
      border-radius: var(--musea-radius-lg);
      display: flex;
      align-items: center;
      justify-content: center;
      margin-bottom: 1.5rem;
    }

    .empty-state-icon svg {
      width: 40px;
      height: 40px;
      color: var(--musea-text-muted);
    }

    .empty-state-title {
      font-size: 1.125rem;
      font-weight: 600;
      margin-bottom: 0.5rem;
    }

    .empty-state-text {
      color: var(--musea-text-muted);
      font-size: 0.875rem;
      max-width: 300px;
    }

    /* Loading */
    .loading {
      display: flex;
      align-items: center;
      justify-content: center;
      min-height: 200px;
      color: var(--musea-text-muted);
      gap: 0.75rem;
    }

    .loading-spinner {
      width: 20px;
      height: 20px;
      border: 2px solid var(--musea-border);
      border-top-color: var(--musea-accent);
      border-radius: 50%;
      animation: spin 0.8s linear infinite;
    }

    @keyframes spin {
      to { transform: rotate(360deg); }
    }

    /* Responsive */
    @media (max-width: 768px) {
      .main {
        grid-template-columns: 1fr;
      }
      .sidebar {
        display: none;
      }
      .header-subtitle {
        display: none;
      }
    }`;
}
