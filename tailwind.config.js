/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{vue,js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      fontFamily: {
        sans: ['Inter', 'system-ui', 'sans-serif'],
      },
      fontSize: {
        'xs': ['calc(var(--font-size-base, 14px) * 0.75)', { lineHeight: '1rem' }],
        'sm': ['calc(var(--font-size-base, 14px) * 0.875)', { lineHeight: '1.25rem' }],
        'base': ['var(--font-size-base, 14px)', { lineHeight: '1.5rem' }],
        'lg': ['calc(var(--font-size-base, 14px) * 1.125)', { lineHeight: '1.75rem' }],
        'xl': ['calc(var(--font-size-base, 14px) * 1.25)', { lineHeight: '1.75rem' }],
        '2xl': ['calc(var(--font-size-base, 14px) * 1.5)', { lineHeight: '2rem' }],
        '3xl': ['calc(var(--font-size-base, 14px) * 1.875)', { lineHeight: '2.25rem' }],
        '4xl': ['calc(var(--font-size-base, 14px) * 2.25)', { lineHeight: '2.5rem' }],
        '5xl': ['calc(var(--font-size-base, 14px) * 3)', { lineHeight: '1' }],
        '6xl': ['calc(var(--font-size-base, 14px) * 3.75)', { lineHeight: '1' }],
        '7xl': ['calc(var(--font-size-base, 14px) * 4.5)', { lineHeight: '1' }],
        '8xl': ['calc(var(--font-size-base, 14px) * 6)', { lineHeight: '1' }],
        '9xl': ['calc(var(--font-size-base, 14px) * 8)', { lineHeight: '1' }],
      },
    },
  },
  plugins: [
    require('daisyui'),
    require('@tailwindcss/typography'),
  ],
  daisyui: {
    themes: [
      {
        light: {
          "primary": "#4f46e5",
          "secondary": "#7c3aed", 
          "accent": "#0ea5e9",
          "neutral": "#374151",
          "base-100": "#ffffff",
          "base-200": "#f8fafc",
          "base-300": "#e2e8f0",
          "info": "#0ea5e9",
          "success": "#10b981",
          "warning": "#f59e0b",
          "error": "#ef4444",
        },
        dark: {
          "primary": "#6366f1",
          "secondary": "#8b5cf6",
          "accent": "#38bdf8",
          "neutral": "#1f2937",
          "base-100": "#0f172a",
          "base-200": "#1e293b",
          "base-300": "#334155",
          "info": "#0ea5e9",
          "success": "#10b981",
          "warning": "#f59e0b",
          "error": "#ef4444",
        },
        corporate: {
          "primary": "#1e40af",
          "secondary": "#6366f1",
          "accent": "#0ea5e9",
          "neutral": "#374151",
          "base-100": "#ffffff",
          "base-200": "#f9fafb",
          "base-300": "#f3f4f6",
          "info": "#0284c7",
          "success": "#059669",
          "warning": "#d97706",
          "error": "#dc2626",
        }
      }
    ],
    darkTheme: "dark",
    base: true,
    styled: true,
    utils: true,
    prefix: "",
    logs: true,
    themeRoot: ":root",
  },
} 