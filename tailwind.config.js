/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{js,ts,jsx,tsx}"],
  theme: { extend: {} },
  daisyui: {
    themes: [
      {
        taskmanager: {
          "primary": "#2474d6",
          "primary-content": "#ffffff",
          "secondary": "#64748b",
          "secondary-content": "#ffffff",
          "accent": "#16a34a",
          "accent-content": "#ffffff",
          "neutral": "#0f172a",
          "neutral-content": "#f8fafc",
          "base-100": "#ffffff",
          "base-200": "#f6f8fc",
          "base-300": "#dbe5f1",
          "base-content": "#0f172a",
          "info": "#2474d6",
          "success": "#16a34a",
          "warning": "#d97706",
          "error": "#e11d48",
        },
      },
    ],
  },
  plugins: [require("daisyui")],
}
