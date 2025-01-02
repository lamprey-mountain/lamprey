import type { Config } from 'tailwindcss'

export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    colors: {
      "bg1": "#0c1012",
      "bg2": "#171c1f",
      "bg3": "#22292c",
      "bg4": "#303a3d",
      "sep": "#616c6e",
    	"fg1": "#ffffff",
    	"fg2": "#eeeeee",
    	"fg3": "#dddddd",
    	"fg4": "#cccccc",
    },
    fontFamily: {
      sans: ["Atkinson Hyperlegible", "Inter", "system-ui", "Avenir", "Helvetica", "Arial", "sans-serif"],
    },
    boxShadow: {
      "arst": "3px 0 0 none inset",
    },
  },
  plugins: [],
} satisfies Config

